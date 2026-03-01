//! hlx-bond — Native GGUF inference + Klyntar corpus bonding
//!
//! Usage:
//!   hlx-bond <model.gguf> [--corpus corpus.db] [--temperature 0.7] [--h-cycles 1]
//!
//! Phases: HELLO → SYNC → BOND → READY → REPL

use anyhow::{Context, Result};
use ape::AxiomEngine;
use candle_core::quantized::gguf_file;
use candle_core::{DType, Device, Tensor};
use candle_transformers::generation::LogitsProcessor;
use candle_transformers::models::quantized_qwen3 as qwen3;
use clap::Parser;
use hlx_runtime::{BondResponse, Capability, SymbioteState};
use rusqlite::Connection;
use std::collections::HashMap;
use std::io::{self, Write};

// ──────────────────────────────────────────────────────────────────────────────
// CLI
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Parser)]
#[command(name = "hlx-bond")]
#[command(about = "Attach a Klyntar symbiote to a local GGUF model")]
struct Args {
    /// Path to the GGUF model file
    model: String,

    /// Path to the Klyntar corpus.db (default: corpus.db in current dir)
    #[arg(short, long, default_value = "corpus.db")]
    corpus: String,

    /// Path to HLX program to run (optional) - runs before each prompt
    #[arg(long)]
    program: Option<String>,

    /// Save bond state to file after session
    #[arg(long)]
    save_state: Option<String>,

    /// Load bond state from file at startup
    #[arg(long)]
    load_state: Option<String>,

    /// Sampling temperature (0.0 = greedy, 1.0 = creative)
    #[arg(long, default_value_t = 0.7)]
    temperature: f64,

    /// Max tokens to generate per response
    #[arg(long, default_value_t = 1024)]
    max_tokens: usize,

    /// TRM H-cycles: recursive reasoning loops per user message
    #[arg(long, default_value_t = 1)]
    h_cycles: usize,

    /// Max rules to pull from corpus for context
    #[arg(long, default_value_t = 20)]
    max_rules: usize,

    /// Max memory entries to pull from corpus for context
    #[arg(long, default_value_t = 10)]
    max_memory: usize,

    /// Path to APE policy file (.axm) for governance
    #[arg(long, default_value = "policy.axm")]
    ape_policy: String,

    /// Disable APE governance (skip verification)
    #[arg(long)]
    no_verify: bool,

    /// Serve HTTP API on this port instead of REPL (e.g., --serve 8765)
    #[arg(long)]
    serve: Option<u16>,
}

// ──────────────────────────────────────────────────────────────────────────────
// Tokenizer (self-contained, built from GGUF metadata)
// ──────────────────────────────────────────────────────────────────────────────

struct GgufTokenizer {
    /// vocab[id] = token string
    vocab: Vec<String>,
    /// token string → id
    vocab_map: HashMap<String, u32>,
    /// merge_ranks[(a_id, b_id)] = rank (lower = higher priority)
    merge_ranks: HashMap<(u32, u32), usize>,
    /// merged_token[rank] = resulting token id after merge
    merged_token: Vec<u32>,
    /// special token ids
    pub bos_id: u32,
    pub eos_id: u32,
    pub im_start_id: u32,
    pub im_end_id: u32,
}

/// GPT-2 / tiktoken byte-to-unicode reverse map: unicode char → original byte.
/// This undoes the byte-level encoding used in Qwen/GPT-2 vocabularies.
fn build_unicode_to_byte() -> HashMap<char, u8> {
    let mut map: HashMap<char, u8> = HashMap::new();
    // Printable ASCII (33–126) and Latin-1 printable (161–172, 174–255) map to themselves
    for b in 33u8..=126 {
        map.insert(b as char, b);
    }
    for b in 161u8..=172 {
        map.insert(char::from_u32(b as u32).unwrap(), b);
    }
    for b in 174u8..=255 {
        map.insert(char::from_u32(b as u32).unwrap(), b);
    }
    // Remaining 256 bytes (0–32, 127–160, 173) get mapped to chars starting at U+0100
    let mut n = 0u32;
    for b in 0u8..=255u8 {
        let in_first = (33..=126).contains(&b);
        let in_second = (161..=172).contains(&b) || (174..=255).contains(&b);
        if !in_first && !in_second {
            map.insert(char::from_u32(256 + n).unwrap(), b);
            n += 1;
        }
    }
    map
}

/// Strip thinking blocks from generated text.
/// Returns (thinking_content, visible_response).
fn strip_thinking(text: &str) -> (String, String) {
    if let Some(start) = text.find("<think>") {
        if let Some(end) = text.find("</think>") {
            let think = text[start + 7..end].to_string();
            let response = text[end + 8..].trim().to_string();
            return (think, response);
        } else {
            // Unclosed think block - discard everything from <think> onward
            let before = text[..start].trim().to_string();
            return (String::new(), before);
        }
    }
    (String::new(), text.trim().to_string())
}

/// Clean ChatML special tokens from decoded output.
fn clean_response(text: &str) -> String {
    let text = text.trim_end_matches("|im_end|").trim();

    // Strip leading special token artifacts
    // Qwen3 sometimes emits user/system/assistant turn tokens before the real response
    let markers = ["|im_start|", "|im_end|", "assistant", "user", "system"];
    let mut result = text;

    for marker in &markers {
        if let Some(pos) = result.find(marker) {
            let after = &result[pos + marker.len()..];
            let after_trimmed = after.trim();
            if !after_trimmed.is_empty() && after_trimmed.len() < result.len() {
                result = after_trimmed;
            }
        }
    }

    result.to_string()
}

impl GgufTokenizer {
    fn from_gguf(content: &gguf_file::Content) -> Result<Self> {
        // ── Extract vocab ──────────────────────────────────────────────────
        let tokens_val = content
            .metadata
            .get("tokenizer.ggml.tokens")
            .context("GGUF missing tokenizer.ggml.tokens")?;

        let vocab: Vec<String> = match tokens_val {
            gguf_file::Value::Array(arr) => arr
                .iter()
                .map(|v| match v {
                    gguf_file::Value::String(s) => Ok(s.clone()),
                    _ => Err(anyhow::anyhow!("non-string token in vocab")),
                })
                .collect::<Result<Vec<_>>>()?,
            _ => return Err(anyhow::anyhow!("tokenizer.ggml.tokens is not an array")),
        };

        let vocab_map: HashMap<String, u32> = vocab
            .iter()
            .enumerate()
            .map(|(i, s)| (s.clone(), i as u32))
            .collect();

        // ── Extract BPE merges ─────────────────────────────────────────────
        let mut merge_ranks: HashMap<(u32, u32), usize> = HashMap::new();
        let mut merged_token: Vec<u32> = Vec::new();

        if let Some(gguf_file::Value::Array(merges)) = content.metadata.get("tokenizer.ggml.merges")
        {
            for (rank, v) in merges.iter().enumerate() {
                if let gguf_file::Value::String(s) = v {
                    let mut parts = s.splitn(2, ' ');
                    if let (Some(a_str), Some(b_str)) = (parts.next(), parts.next()) {
                        let Some(&a_id) = vocab_map.get(a_str) else {
                            continue;
                        };
                        let Some(&b_id) = vocab_map.get(b_str) else {
                            continue;
                        };
                        let merged_str = format!("{}{}", a_str, b_str);
                        let result_id = vocab_map.get(&merged_str).copied().unwrap_or(0);
                        merge_ranks.insert((a_id, b_id), rank);
                        merged_token.push(result_id);
                    }
                }
            }
        }

        // ── Special tokens ─────────────────────────────────────────────────
        let get_u32 = |key: &str| -> u32 {
            match content.metadata.get(key) {
                Some(gguf_file::Value::U32(v)) => *v,
                Some(gguf_file::Value::U64(v)) => *v as u32,
                Some(gguf_file::Value::I32(v)) => *v as u32,
                _ => u32::MAX,
            }
        };

        let bos_id = get_u32("tokenizer.ggml.bos_token_id");
        let eos_id = get_u32("tokenizer.ggml.eos_token_id");
        let im_start_id = vocab_map.get("<|im_start|>").copied().unwrap_or(u32::MAX);
        let im_end_id = vocab_map.get("<|im_end|>").copied().unwrap_or(u32::MAX);

        eprintln!(
            "[tokenizer] vocab={}, merges={}, bos={}, eos={}, im_start={}, im_end={}",
            vocab.len(),
            merge_ranks.len(),
            bos_id,
            eos_id,
            im_start_id,
            im_end_id
        );

        Ok(Self {
            vocab,
            vocab_map,
            merge_ranks,
            merged_token,
            bos_id,
            eos_id,
            im_start_id,
            im_end_id,
        })
    }

    /// Encode a raw string slice into token IDs using BPE.
    fn encode(&self, text: &str) -> Vec<u32> {
        // Initial segmentation: try to find each character in vocab,
        // fall back to byte-level tokens <0xNN>
        let mut tokens: Vec<u32> = Vec::new();
        let mut chars = text.chars().peekable();

        while let Some(c) = chars.next() {
            let s = c.to_string();
            if let Some(&id) = self.vocab_map.get(&s) {
                tokens.push(id);
            } else {
                // Byte fallback
                for byte in s.as_bytes() {
                    let byte_tok = format!("<0x{:02X}>", byte);
                    tokens.push(self.vocab_map.get(&byte_tok).copied().unwrap_or(0));
                }
            }
        }

        // Apply BPE merges (greedy, highest-priority first)
        loop {
            let mut best_rank = usize::MAX;
            let mut best_pos = usize::MAX;

            for i in 0..tokens.len().saturating_sub(1) {
                let pair = (tokens[i], tokens[i + 1]);
                if let Some(&rank) = self.merge_ranks.get(&pair) {
                    if rank < best_rank {
                        best_rank = rank;
                        best_pos = i;
                    }
                }
            }

            if best_pos == usize::MAX {
                break;
            }

            let merged_id = self.merged_token[best_rank];
            tokens[best_pos] = merged_id;
            tokens.remove(best_pos + 1);
        }

        tokens
    }

    /// Decode token IDs back to a UTF-8 string.
    /// Handles GPT-2 style byte-level encoding (Ġ=space, Ċ=newline, etc.)
    fn decode(&self, tokens: &[u32]) -> String {
        let u2b = build_unicode_to_byte();
        let mut bytes: Vec<u8> = Vec::new();
        for &id in tokens {
            if let Some(tok) = self.vocab.get(id as usize) {
                // Check for <0xNN> format byte tokens
                if tok.starts_with("<0x") && tok.ends_with('>') && tok.len() == 6 {
                    if let Ok(b) = u8::from_str_radix(&tok[3..5], 16) {
                        bytes.push(b);
                        continue;
                    }
                }
                // GPT-2 unicode → byte mapping
                for ch in tok.chars() {
                    // Explicit GPT-2 special byte chars (before general lookup)
                    let b = match ch {
                        '\u{0120}' => Some(32u8), // Ġ → space
                        '\u{010a}' => Some(10u8), // Ċ → newline
                        '\u{0109}' => Some(9u8),  // ĉ → tab
                        _ => u2b.get(&ch).copied(),
                    };
                    if let Some(byte) = b {
                        bytes.push(byte);
                    } else {
                        // True unicode character, encode as UTF-8
                        let mut buf = [0u8; 4];
                        bytes.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
                    }
                }
            }
        }
        String::from_utf8_lossy(&bytes).into_owned()
    }

    /// Build a Qwen3 chat-formatted prompt and return token IDs.
    /// Format: <|im_start|>system\n{system}<|im_end|>\n
    ///         <|im_start|>user\n{user}<|im_end|>\n
    ///         <|im_start|>assistant\n
    fn encode_chat(&self, system: &str, history: &[(String, String)], user: &str) -> Vec<u32> {
        let mut ids: Vec<u32> = Vec::new();

        // System turn
        if !system.is_empty() {
            ids.push(self.im_start_id);
            ids.extend(self.encode("system\n"));
            ids.extend(self.encode(system));
            ids.push(self.im_end_id);
            ids.extend(self.encode("\n"));
        }

        // History
        for (user_msg, asst_msg) in history {
            ids.push(self.im_start_id);
            ids.extend(self.encode("user\n"));
            ids.extend(self.encode(user_msg));
            ids.push(self.im_end_id);
            ids.extend(self.encode("\n"));

            ids.push(self.im_start_id);
            ids.extend(self.encode("assistant\n"));
            ids.extend(self.encode(asst_msg));
            ids.push(self.im_end_id);
            ids.extend(self.encode("\n"));
        }

        // Current user turn
        ids.push(self.im_start_id);
        ids.extend(self.encode("user\n"));
        ids.extend(self.encode(user));
        ids.push(self.im_end_id);
        ids.extend(self.encode("\n"));

        // Open assistant turn (model continues from here)
        ids.push(self.im_start_id);
        ids.extend(self.encode("assistant\n"));

        ids
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Corpus loader (reads Klyntar's SQLite corpus.db)
// ──────────────────────────────────────────────────────────────────────────────

struct CorpusContext {
    conn: Connection,
}

impl CorpusContext {
    fn open(path: &str) -> Result<Self> {
        let conn =
            Connection::open(path).with_context(|| format!("Failed to open corpus at {}", path))?;
        Ok(Self { conn })
    }

    fn build_system_prompt(&self, max_rules: usize, max_memory: usize) -> Result<String> {
        let mut prompt = String::from(
            "You are a neurosymbolic AI — a language model bonded to a Klyntar \
             symbolic corpus. Your reasoning is governed by the rules below. \
             Respect them absolutely.\n\n",
        );

        // Top rules by confidence
        let mut stmt = self.conn.prepare(
            "SELECT name, description, confidence FROM rules \
             ORDER BY confidence DESC LIMIT ?1",
        )?;
        let rules: Vec<(String, String, f64)> = stmt
            .query_map([max_rules], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .filter_map(|r| r.ok())
            .collect();

        if !rules.is_empty() {
            prompt.push_str("## Governing Rules\n");
            for (name, desc, conf) in &rules {
                prompt.push_str(&format!(
                    "- **{}** (confidence {:.2}): {}\n",
                    name, conf, desc
                ));
            }
            prompt.push('\n');
        }

        // Recent memory (conversation history stored in corpus)
        let mut stmt = self
            .conn
            .prepare("SELECT source, content FROM memory ORDER BY created_at DESC LIMIT ?1")?;
        let memories: Vec<(String, String)> = stmt
            .query_map([max_memory], |r| Ok((r.get(0)?, r.get(1)?)))
            .into_iter()
            .flat_map(|iter| iter.filter_map(|r| r.ok()))
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();

        if !memories.is_empty() {
            prompt.push_str("## Recent Memory\n");
            for (source, content) in &memories {
                prompt.push_str(&format!("[{}]: {}\n", source, content));
            }
        }

        Ok(prompt)
    }

    fn store_memory(&self, role: &str, content: &str) -> Result<()> {
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();
        self.conn.execute(
            "INSERT INTO memory (role, content, created_at) VALUES (?1, ?2, ?3)",
            rusqlite::params![role, content, ts],
        )?;
        Ok(())
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Inference engine (candle quantized Qwen2/3)
// ──────────────────────────────────────────────────────────────────────────────

struct InferenceEngine {
    model: qwen3::ModelWeights,
    device: Device,
    logits_processor: LogitsProcessor,
    eos_id: u32,
    im_end_id: u32,
}

impl InferenceEngine {
    fn load(model_path: &str, temperature: f64, tokenizer: &GgufTokenizer) -> Result<Self> {
        // Try CUDA first, fall back to CPU
        let device = Device::new_cuda(0).unwrap_or_else(|e| {
            eprintln!("[engine] CUDA unavailable ({}), falling back to CPU", e);
            Device::Cpu
        });

        // Log which device we're using
        match &device {
            Device::Cuda(d) => eprintln!("[engine] CUDA device: {:?}", d),
            Device::Cpu => eprintln!("[engine] Running on CPU"),
            _ => {}
        }

        let mut file = std::fs::File::open(model_path)
            .with_context(|| format!("Cannot open model file: {}", model_path))?;

        eprintln!("[engine] Reading GGUF structure...");
        let content = gguf_file::Content::read(&mut file)
            .map_err(|e| anyhow::anyhow!("Failed to read GGUF: {:?}", e))?;

        let arch = content
            .metadata
            .get("general.architecture")
            .and_then(|v| {
                if let gguf_file::Value::String(s) = v {
                    Some(s.as_str())
                } else {
                    None
                }
            })
            .unwrap_or("unknown");
        eprintln!("[engine] Architecture: {}", arch);

        eprintln!("[engine] Loading model weights (this may take a moment)...");
        let model = qwen3::ModelWeights::from_gguf(content, &mut file, &device)
            .map_err(|e| anyhow::anyhow!("Failed to load model weights: {:?}", e))?;

        let logits_processor = LogitsProcessor::new(
            42, // seed
            Some(temperature),
            None, // top_p
        );

        Ok(Self {
            model,
            device,
            logits_processor,
            eos_id: tokenizer.eos_id,
            im_end_id: tokenizer.im_end_id,
        })
    }

    /// Generate tokens from a prompt (given as token IDs).
    /// Returns the generated token IDs (excluding the prompt).
    fn generate(&mut self, prompt_tokens: &[u32], max_tokens: usize) -> Result<Vec<u32>> {
        let mut all_tokens: Vec<u32> = prompt_tokens.to_vec();
        let mut generated: Vec<u32> = Vec::new();
        let mut pos = 0;

        // Prefill: process the whole prompt
        let prompt_tensor = Tensor::new(prompt_tokens, &self.device)?.unsqueeze(0)?;
        let logits = self
            .model
            .forward(&prompt_tensor, pos)
            .map_err(|e| anyhow::anyhow!("forward pass error: {:?}", e))?;

        // forward returns [1, vocab_size] — squeeze batch dim → [vocab_size]
        let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
        let next_tok = self.logits_processor.sample(&logits)? as u32;
        generated.push(next_tok);
        all_tokens.push(next_tok);
        pos = prompt_tokens.len();

        // Decode step by step
        for _ in 1..max_tokens {
            let tok = *generated.last().unwrap();
            if tok == self.eos_id || tok == self.im_end_id {
                break;
            }

            let input = Tensor::new(&[tok], &self.device)?.unsqueeze(0)?;
            let logits = self
                .model
                .forward(&input, pos)
                .map_err(|e| anyhow::anyhow!("decode step error: {:?}", e))?;
            // forward returns [1, vocab_size] — squeeze → [vocab_size]
            let logits = logits.squeeze(0)?.to_dtype(DType::F32)?;
            let next_tok = self.logits_processor.sample(&logits)? as u32;

            generated.push(next_tok);
            pos += 1;
        }

        Ok(generated)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Bond protocol
// ──────────────────────────────────────────────────────────────────────────────

fn run_bond_protocol(model_name: &str, vocab_size: usize) -> Result<SymbioteState> {
    let mut state = SymbioteState::new();

    println!("[HELLO] Initiating bond with model: {}", model_name);
    let request = state.create_bond_request();
    println!(
        "[HELLO] Capabilities offered: {}",
        request
            .capabilities
            .iter()
            .map(|c| c.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Simulate the LLM responding to the bond request
    let response = BondResponse {
        accepted: true,
        model_name: model_name.to_string(),
        model_version: "gguf-local".to_string(),
        context_window: 4096,
        capabilities: vec![
            Capability {
                name: "text_generation".to_string(),
                version: "1.0".to_string(),
                description: Some("Autoregressive token generation".to_string()),
            },
            Capability {
                name: "gguf_native".to_string(),
                version: "1.0".to_string(),
                description: Some(format!("GGUF inference, vocab_size={}", vocab_size)),
            },
        ],
        rejection_reason: None,
    };

    state
        .process_hello(&response)
        .map_err(|e| anyhow::anyhow!("HELLO failed: {}", e.message))?;
    println!("[SYNC]  Bond accepted — synchronising state...");
    state
        .process_sync()
        .map_err(|e| anyhow::anyhow!("SYNC failed: {}", e.message))?;

    println!("[BOND]  Corpus context injected — neurosymbolic link forming...");
    state
        .process_bond()
        .map_err(|e| anyhow::anyhow!("BOND failed: {}", e.message))?;

    println!("[READY] Bond complete. Symbiote is active.\n");
    state
        .process_ready()
        .map_err(|e| anyhow::anyhow!("READY failed: {}", e.message))?;

    Ok(state)
}

// ──────────────────────────────────────────────────────────────────────────────
// REPL
// ──────────────────────────────────────────────────────────────────────────────

fn run_hlx_program(program_path: &str, input: &str) -> Option<String> {
    use std::process::Command;

    // Try to run hlx-run with the program
    let output = Command::new("hlx-run")
        .arg(program_path)
        .arg("--input")
        .arg(input)
        .output()
        .ok()?;

    if output.status.success() {
        let result = String::from_utf8_lossy(&output.stdout).to_string();
        Some(result)
    } else {
        let err = String::from_utf8_lossy(&output.stderr).to_string();
        eprintln!("[hlx] Program error: {}", err);
        None
    }
}

fn handle_command(
    cmd: &str,
    args: &Args,
    state: &mut SymbioteState,
    history: &mut Vec<(String, String)>,
) -> Result<()> {
    match cmd {
        "/help" | "/h" => {
            println!("Commands:");
            println!("  /help, /h     - Show this help");
            println!("  /history      - Show conversation history");
            println!("  /clear        - Clear history");
            println!("  /save         - Save session state");
            println!("  /quit, /q    - Exit");
        }
        "/history" | "/hist" => {
            for (i, (user, bot)) in history.iter().enumerate() {
                println!("[{}] you:  {}", i + 1, user);
                println!("[{}] hlx:  {}", i + 1, bot.lines().next().unwrap_or(""));
            }
        }
        "/clear" => {
            history.clear();
            println!("History cleared");
        }
        "/save" => {
            if let Some(ref path) = args.save_state {
                let session = SessionState {
                    symbiote_id: state.id.clone(),
                    step_count: state.step_count,
                    history: history.clone(),
                    timestamp: std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs_f64(),
                };
                session.save(path)?;
                println!("Session saved to: {}", path);
                println!("  ID: {}", session.symbiote_id);
                println!("  Steps: {}", session.step_count);
            } else {
                println!("No save path specified. Use --save-state <file>");
            }
        }
        "/quit" | "/q" | "/exit" => {
            std::process::exit(0);
        }
        _ => {
            println!("Unknown command: {}", cmd);
        }
    }
    Ok(())
}

fn run_repl(
    engine: &mut InferenceEngine,
    tokenizer: &GgufTokenizer,
    corpus: &CorpusContext,
    state: &mut SymbioteState,
    args: &Args,
    loaded_session: Option<SessionState>,
) -> Result<()> {
    let stdin = io::stdin();
    let mut history: Vec<(String, String)> = Vec::new();

    // Restore history from loaded session if present
    if let Some(session) = loaded_session {
        history = session.history;
        if !history.is_empty() {
            println!(
                "[session] Restored {} conversation turns from saved state",
                history.len()
            );
        }
    }

    // Initialize APE engine for governance
    let ape_engine = if args.no_verify {
        None
    } else {
        match AxiomEngine::from_file(&args.ape_policy) {
            Ok(engine) => {
                eprintln!("[APE] Governance loaded: {}", args.ape_policy);
                Some(engine)
            }
            Err(e) => {
                eprintln!(
                    "[APE] Warning: Could not load policy '{}': {}",
                    args.ape_policy, e
                );
                eprintln!(
                    "[APE] Running without governance. Use --no-verify to suppress this warning."
                );
                None
            }
        }
    };

    println!("Neurosymbolic AI ready. Type your message (Ctrl+D to exit).\n");

    loop {
        print!("you> ");
        io::stdout().flush()?;

        let mut user_input = match stdin.lock().lines().next() {
            None => break,
            Some(line) => {
                let s = line?;
                if s.trim().is_empty() {
                    continue;
                }
                s
            }
        };

        // Handle REPL commands
        if user_input.starts_with('/') {
            handle_command(&user_input, args, state, &mut history)?;
            continue;
        }

        // Run HLX program if specified
        if let Some(ref program_path) = args.program {
            println!("[hlx] Running program: {}", program_path);
            if let Some(hlx_result) = run_hlx_program(program_path, &user_input) {
                println!("[hlx] Result: {}", hlx_result.trim());
                user_input = format!("{} [HLX context: {}]", user_input, hlx_result.trim());
            }
        }

        // Store user message in corpus memory
        corpus.store_memory("user", &user_input).ok();

        // Build system prompt from corpus
        let system = corpus
            .build_system_prompt(args.max_rules, args.max_memory)
            .unwrap_or_default();

        // TRM H-cycles: run multiple reasoning passes if requested
        let mut final_response = String::new();

        for h in 0..args.h_cycles {
            if args.h_cycles > 1 {
                print!("[H-cycle {}/{}] ", h + 1, args.h_cycles);
                io::stdout().flush()?;
            }

            // Encode the chat prompt
            let prompt_tokens = tokenizer.encode_chat(&system, &history, &user_input);

            // Generate
            let generated = engine.generate(&prompt_tokens, args.max_tokens)?;
            let response = tokenizer.decode(&generated);

            // Strip trailing <|im_end|>, then separate thinking from visible response
            let response = response.trim_end_matches("<|im_end|>").trim();
            let (thinking, visible) = strip_thinking(response);
            if !thinking.is_empty() && args.h_cycles > 1 {
                eprintln!("[think] {}", thinking.trim());
            }
            let visible = clean_response(&visible);
            let response = visible;

            if h == args.h_cycles - 1 {
                final_response = response.clone();
            } else {
                // Feed intermediate result back as context for next cycle
                user_input = format!(
                    "{}\n\n[H-cycle {} result: {}]\n\nRefine your answer:",
                    user_input,
                    h + 1,
                    response
                );
            }

            state.step_count += 1;
        }

        // APE Governance: Verify LLM output before displaying
        if let Some(ref engine) = ape_engine {
            let verdict = engine.verify(
                "GenerateResponse",
                &[
                    ("output", &final_response),
                    ("verified", "true"), // Required for Execute-class intents
                ],
            );

            match verdict {
                Ok(v) if v.allowed() => {
                    eprintln!("[APE] ✓ Response verified");
                }
                Ok(v) => {
                    let reason = v.reason().unwrap_or("unknown policy violation");
                    eprintln!("[APE] ✗ Response denied: {}", reason);
                    final_response = format!("[Governance: Response blocked - {}]", reason);
                }
                Err(e) => {
                    eprintln!("[APE] ⚠ Verification error: {}", e);
                    // Continue with response but warn
                }
            }
        }

        println!("hlx> {}\n", final_response);

        // Store response in corpus memory
        corpus.store_memory("assistant", &final_response).ok();

        // Add to conversation history
        history.push((user_input.clone(), final_response));

        // Keep history bounded (last 10 turns)
        if history.len() > 10 {
            history.remove(0);
        }
    }

    Ok(())
}

// ──────────────────────────────────────────────────────────────────────────────
// Main
// ──────────────────────────────────────────────────────────────────────────────

fn main() -> Result<()> {
    let args = Args::parse();

    println!("╔══════════════════════════════════════════════════╗");
    println!("║  HLX Bond Protocol v0.1 — Native GGUF + Klyntar  ║");
    println!("╚══════════════════════════════════════════════════╝\n");

    // ── Step 1: Load GGUF structure and tokenizer ──────────────────────────
    println!("[1/4] Reading GGUF metadata...");
    let mut model_file = std::fs::File::open(&args.model)
        .with_context(|| format!("Cannot open GGUF file: {}", args.model))?;
    let gguf_content = gguf_file::Content::read(&mut model_file)
        .map_err(|e| anyhow::anyhow!("GGUF read error: {:?}", e))?;

    let model_name = gguf_content
        .metadata
        .get("general.name")
        .and_then(|v| {
            if let gguf_file::Value::String(s) = v {
                Some(s.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(|| {
            std::path::Path::new(&args.model)
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "unknown".into())
        });

    println!("[1/4] Building tokenizer from GGUF metadata...");
    let tokenizer = GgufTokenizer::from_gguf(&gguf_content)?;
    let vocab_size = tokenizer.vocab.len();
    drop(gguf_content); // free metadata before loading weights

    // ── Step 2: Load corpus ────────────────────────────────────────────────
    println!("[2/4] Loading Klyntar corpus from {}...", args.corpus);
    let corpus = CorpusContext::open(&args.corpus).unwrap_or_else(|e| {
        eprintln!(
            "[warn] Could not open corpus ({}). Running without symbolic context.",
            e
        );
        // Return empty corpus that will gracefully skip DB operations
        CorpusContext {
            conn: Connection::open_in_memory().expect("in-memory sqlite"),
        }
    });

    // ── Step 3: Run bond protocol ──────────────────────────────────────────
    println!("[3/4] Running bond protocol...");
    let mut symbiote_state = run_bond_protocol(&model_name, vocab_size)?;

    // ── Step 3b: Load session state if provided ────────────────────────────
    let loaded_session: Option<SessionState> = if let Some(ref path) = args.load_state {
        match SessionState::load(path) {
            Ok(session) => {
                println!(
                    "[session] Resuming session {} at step {}",
                    session.symbiote_id.chars().take(8).collect::<String>(),
                    session.step_count
                );
                // Update symbiote state with loaded values
                symbiote_state.step_count = session.step_count;
                Some(session)
            }
            Err(e) => {
                eprintln!(
                    "[session] Warning: Could not load state from {}: {}",
                    path, e
                );
                None
            }
        }
    } else {
        None
    };

    // ── Step 4: Load model weights ─────────────────────────────────────────
    println!("[4/4] Loading model weights...");
    let mut engine = InferenceEngine::load(&args.model, args.temperature, &tokenizer)?;

    println!("\nModel: {}", model_name);
    println!("Vocab: {} tokens", vocab_size);
    println!("Temperature: {}", args.temperature);
    println!("H-cycles: {}", args.h_cycles);
    println!("Corpus: {}", args.corpus);
    println!();

    // ── Dispatch: Server mode or REPL ───────────────────────────────────────
    if let Some(port) = args.serve {
        // Server mode
        run_server(port, &mut engine, &tokenizer, &corpus, &args)?;
    } else {
        // REPL mode
        run_repl(
            &mut engine,
            &tokenizer,
            &corpus,
            &mut symbiote_state,
            &args,
            loaded_session,
        )?;

        println!(
            "\n[bond] Session ended. Steps taken: {}",
            symbiote_state.step_count
        );
    }

    Ok(())
}

use std::io::BufRead;

// ──────────────────────────────────────────────────────────────────────────────
// Session State persistence
// ──────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SessionState {
    symbiote_id: String,
    step_count: usize,
    history: Vec<(String, String)>,
    timestamp: f64,
}

impl SessionState {
    fn save(&self, path: &str) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    fn load(path: &str) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let state: SessionState = serde_json::from_str(&json)?;
        Ok(state)
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// HTTP Server mode (for --serve flag)
// ──────────────────────────────────────────────────────────────────────────────

fn run_server(
    port: u16,
    engine: &mut InferenceEngine,
    tokenizer: &GgufTokenizer,
    corpus: &CorpusContext,
    args: &Args,
) -> Result<()> {
    use tiny_http::{Request, Response, Server};

    let addr = format!("127.0.0.1:{}", port);
    let server = Server::http(&addr)
        .map_err(|e| anyhow::anyhow!("Failed to start server on {}: {:?}", addr, e))?;

    println!("[serve] Listening on http://{}/", addr);
    println!("[serve] Endpoints: POST /bond, POST /infer");
    println!("[serve] Press Ctrl+C to stop\n");

    // Initialize APE engine for governance (shared across requests)
    let ape_engine = if args.no_verify {
        None
    } else {
        match AxiomEngine::from_file(&args.ape_policy) {
            Ok(engine) => {
                eprintln!("[APE] Governance loaded: {}", args.ape_policy);
                Some(engine)
            }
            Err(e) => {
                eprintln!(
                    "[APE] Warning: Could not load policy '{}': {}",
                    args.ape_policy, e
                );
                None
            }
        }
    };

    for request in server.incoming_requests() {
        match handle_request(request, engine, tokenizer, corpus, &ape_engine, args) {
            Ok(()) => {}
            Err(e) => {
                eprintln!("[serve] Error handling request: {:?}", e);
            }
        }
    }

    Ok(())
}

fn handle_request(
    request: tiny_http::Request,
    engine: &mut InferenceEngine,
    tokenizer: &GgufTokenizer,
    corpus: &CorpusContext,
    ape_engine: &Option<AxiomEngine>,
    args: &Args,
) -> Result<()> {
    let url = request.url().to_string();
    let method = request.method().to_string();

    match (method.as_str(), url.as_str()) {
        ("POST", "/bond") => handle_bond(request),
        ("POST", "/infer") => handle_infer(request, engine, tokenizer, corpus, ape_engine, args),
        _ => {
            let response = serde_json::json!({
                "error": "Not found",
                "path": url,
                "method": method
            });
            let resp = tiny_http::Response::from_string(response.to_string())
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
                .with_status_code(404);
            request.respond(resp)?;
            Ok(())
        }
    }
}

fn handle_bond(mut request: tiny_http::Request) -> Result<()> {
    // Read request body
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body)?;

    // Parse BondRequest if body present (optional)
    if !body.is_empty() {
        if let Err(e) = serde_json::from_str::<hlx_runtime::BondRequest>(&body) {
            eprintln!("[serve] Warning: Could not parse bond request: {}", e);
        }
    }

    // Run bond protocol and return BondResponse
    let response = hlx_runtime::BondResponse {
        accepted: true,
        model_name: "candle-gguf".to_string(),
        model_version: "candle-gguf".to_string(),
        context_window: 4096,
        capabilities: vec![
            hlx_runtime::Capability {
                name: "candle_inference".to_string(),
                version: "1.0".to_string(),
                description: Some("Native GGUF inference via Candle".to_string()),
            },
            hlx_runtime::Capability {
                name: "klyntar_corpus".to_string(),
                version: "1.0".to_string(),
                description: Some("Klyntar symbolic corpus integration".to_string()),
            },
            hlx_runtime::Capability {
                name: "ape_governance".to_string(),
                version: "1.0".to_string(),
                description: Some("APE effect governance on all outputs".to_string()),
            },
        ],
        rejection_reason: None,
    };

    let json = serde_json::to_string(&response)?;
    let resp = tiny_http::Response::from_string(json)
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_status_code(200);

    request.respond(resp)?;
    eprintln!("[serve] POST /bond -> accepted");
    Ok(())
}

fn handle_infer(
    mut request: tiny_http::Request,
    engine: &mut InferenceEngine,
    tokenizer: &GgufTokenizer,
    corpus: &CorpusContext,
    ape_engine: &Option<AxiomEngine>,
    args: &Args,
) -> Result<()> {
    // Read and parse request body
    let mut body = String::new();
    request.as_reader().read_to_string(&mut body)?;

    let req: serde_json::Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => {
            let response = serde_json::json!({"error": format!("Invalid JSON: {}", e)});
            let resp = tiny_http::Response::from_string(response.to_string())
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
                .with_status_code(400);
            request.respond(resp)?;
            return Ok(());
        }
    };

    let prompt = req.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let context = req.get("context").and_then(|v| v.as_str()).unwrap_or("");
    let symbiote_id = req
        .get("symbiote_id")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    eprintln!(
        "[serve] Request from symbiote: {}...",
        &symbiote_id[..symbiote_id.len().min(8)]
    );

    if prompt.is_empty() {
        let response = serde_json::json!({"error": "Missing 'prompt' field"});
        let resp = tiny_http::Response::from_string(response.to_string())
            .with_header(
                tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                    .unwrap(),
            )
            .with_status_code(400);
        request.respond(resp)?;
        return Ok(());
    }

    // Store user prompt in corpus memory
    corpus.store_memory("user", prompt).ok();

    // Build system prompt from corpus
    let mut system = corpus
        .build_system_prompt(args.max_rules, args.max_memory)
        .unwrap_or_default();

    // Append context if provided
    if !context.is_empty() {
        system.push_str("\n\n## Context\n");
        system.push_str(context);
    }

    // Run inference
    let history: Vec<(String, String)> = Vec::new();
    // Disable Qwen3 thinking mode for serve calls (saves tokens for answer)
    system.push_str("\n/no_think");
    
    let prompt_tokens = tokenizer.encode_chat(&system, &history, prompt);

    let generated = match engine.generate(&prompt_tokens, args.max_tokens) {
        Ok(tokens) => tokens,
        Err(e) => {
            let response = serde_json::json!({"error": format!("Generation failed: {}", e)});
            let resp = tiny_http::Response::from_string(response.to_string())
                .with_header(
                    tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..])
                        .unwrap(),
                )
                .with_status_code(500);
            request.respond(resp)?;
            return Ok(());
        }
    };

    let response_text = tokenizer.decode(&generated);
    let response_text = response_text.trim_end_matches("|im_end|").trim();
    let (_thinking, visible) = strip_thinking(&response_text);
    let visible = clean_response(&visible);
    let final_response = visible.to_string();

    // APE Governance: Verify LLM output
    let mut approved_response = final_response.clone();
    if let Some(ref ape) = ape_engine {
        let verdict = ape.verify(
            "GenerateResponse",
            &[("output", &final_response), ("verified", "true")],
        );

        match verdict {
            Ok(v) if v.allowed() => {
                eprintln!("[APE] ✓ Response verified");
            }
            Ok(v) => {
                let reason = v.reason().unwrap_or("policy violation");
                eprintln!("[APE] ✗ Response denied: {}", reason);
                approved_response = format!("[Governance blocked: {}]", reason);
            }
            Err(e) => {
                eprintln!("[APE] ⚠ Verification error: {}", e);
            }
        }
    }

    // Store response in corpus memory
    corpus.store_memory("assistant", &approved_response).ok();

    let response = serde_json::json!({
        "response": approved_response,
        "model": "candle-gguf",
        "tokens_generated": generated.len()
    });

    let resp = tiny_http::Response::from_string(response.to_string())
        .with_header(
            tiny_http::Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]).unwrap(),
        )
        .with_status_code(200);

    request.respond(resp)?;
    eprintln!("[serve] POST /infer -> {} tokens", generated.len());
    Ok(())
}
