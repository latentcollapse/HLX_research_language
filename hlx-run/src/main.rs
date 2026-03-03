//! hlx-run — Execute HLX programs
//!
//! Usage:
//!   hlx-run <program.hlx> [args...]
//!   hlx-run <program.hlx> --func <function_name> [args...]
//!
//! Reads an HLX source file, compiles to bytecode, and executes.

use anyhow::{Context, Result};
use ape::AxiomEngine;
use clap::Parser;
use rusqlite::Connection;
use serde_json::json;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

const DB_PATH: &str = "hlx_memory.db";

#[derive(Parser)]
#[command(name = "hlx-run")]
#[command(about = "Execute HLX programs")]
struct Args {
    /// Path to the HLX source file
    #[arg(value_name = "FILE")]
    file: Option<String>,

    /// Function to call (default: main)
    #[arg(short, long)]
    func: Option<String>,

    /// Arguments to pass to the function
    #[arg(value_name = "ARGS")]
    args: Vec<String>,

    /// Input to pass to the program (for bond integration)
    #[arg(long)]
    input: Option<String>,

    /// Run in REPL mode
    #[arg(short, long)]
    repl: bool,

    /// Show bytecode before executing
    #[arg(short, long)]
    verbose: bool,

    /// Trace execution (show each opcode and register state)
    #[arg(short, long)]
    debug: bool,

    /// Maximum array size (elements)
    #[arg(long, default_value_t = 1_000_000)]
    max_array_size: usize,

    /// Maximum string size (bytes)
    #[arg(long, default_value_t = 10_000_000)]
    max_string_size: usize,

    /// Wall-clock timeout in milliseconds
    #[arg(long)]
    timeout_ms: Option<u64>,

    /// Maximum steps to execute
    #[arg(long, default_value_t = 1_000_000)]
    max_steps: usize,

    /// Path to APE policy file (.axm) for governance
    #[arg(long, default_value = "policy.axm")]
    ape_policy: String,

    /// Disable APE governance (skip verification)
    #[arg(long)]
    no_verify: bool,

    /// Path to SQLite memory database (default: hlx_memory.db)
    #[arg(long, default_value = "hlx_memory.db")]
    memory_db: String,

    /// Bond endpoint URL for LLM connection (e.g., http://localhost:8765)
    #[arg(long, env = "HLX_BOND_ENDPOINT")]
    bond_endpoint: Option<String>,
}

fn init_memory_db(db_path: &str) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS patterns (
            hash TEXT PRIMARY KEY,
            pattern TEXT NOT NULL,
            confidence REAL NOT NULL,
            observation_count INTEGER NOT NULL DEFAULT 1
        );
        CREATE INDEX IF NOT EXISTS idx_patterns_confidence ON patterns(confidence DESC);",
    )?;
    Ok(conn)
}

fn load_patterns_from_db(conn: &Connection, limit: usize) -> Result<Vec<(String, f64)>> {
    let mut stmt =
        conn.prepare("SELECT pattern, confidence FROM patterns ORDER BY confidence DESC LIMIT ?")?;
    let rows = stmt.query_map([limit], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, f64>(1)?))
    })?;

    let mut patterns = Vec::new();
    for row in rows {
        patterns.push(row?);
    }
    Ok(patterns)
}

// ── Pattern matching helper functions ────────────────────────────────────────

fn longest_common_substring(s1: &str, s2: &str) -> Option<String> {
    let s1_chars: Vec<char> = s1.chars().collect();
    let s2_chars: Vec<char> = s2.chars().collect();
    let n = s1_chars.len();
    let m = s2_chars.len();

    if n == 0 || m == 0 {
        return None;
    }

    let mut max_len = 0;
    let mut max_end = 0;
    let mut dp = vec![vec![0; m + 1]; n + 1];

    for i in 1..=n {
        for j in 1..=m {
            if s1_chars[i - 1] == s2_chars[j - 1] {
                dp[i][j] = dp[i - 1][j - 1] + 1;
                if dp[i][j] > max_len {
                    max_len = dp[i][j];
                    max_end = i;
                }
            }
        }
    }

    if max_len > 0 {
        Some(s1_chars[max_end - max_len..max_end].iter().collect())
    } else {
        None
    }
}

fn compute_similarity(s1: &str, s2: &str) -> f64 {
    let len1 = s1.len();
    let len2 = s2.len();

    if len1 == 0 && len2 == 0 {
        return 1.0;
    }

    let max_len = std::cmp::max(len1, len2);
    let matches = s1
        .chars()
        .zip(s2.chars())
        .filter(|(c1, c2)| c1 == c2)
        .count();

    matches as f64 / max_len as f64
}

fn store_pattern_in_db(conn: &Connection, pattern: &str, confidence: f64) -> Result<()> {
    // Ensure table exists (for when this is called from a fresh connection)
    conn.execute(
        "CREATE TABLE IF NOT EXISTS patterns (
            hash TEXT PRIMARY KEY,
            pattern TEXT NOT NULL,
            confidence REAL NOT NULL,
            observation_count INTEGER NOT NULL DEFAULT 1
        )",
        [],
    )?;
    let hash = blake3::hash(pattern.as_bytes()).to_hex().to_string();
    conn.execute(
        "INSERT OR REPLACE INTO patterns (hash, pattern, confidence, observation_count)
         VALUES (?1, ?2, ?3, 
            COALESCE((SELECT observation_count FROM patterns WHERE hash=?1), 0) + 1)",
        (&hash, pattern, confidence),
    )?;
    Ok(())
}

/// Run the full Bond protocol handshake: HELLO -> SYNC -> BOND -> READY
fn run_bond_handshake(endpoint: &str, prompt: &str, _context: &str) -> String {
    use hlx_runtime::{BondResponse, SymbioteState};

    let mut state = SymbioteState::new();

    // Step 1: HELLO - Send BondRequest to /bond endpoint
    let bond_request = state.create_bond_request();
    let bond_url = format!("{}/bond", endpoint.trim_end_matches('/'));

    let bond_response = match ureq::post(&bond_url)
        .set("Content-Type", "application/json")
        .send_json(&bond_request)
    {
        Ok(res) => match res.into_json::<BondResponse>() {
            Ok(r) => r,
            Err(e) => return format!("[Bond Error: Failed to parse bond response: {}]", e),
        },
        Err(e) => return format!("[Bond Error: Failed to connect to {}: {}]", bond_url, e),
    };

    // Process HELLO phase
    if let Err(e) = state.process_hello(&bond_response) {
        return format!("[Bond Error: HELLO failed: {}]", e.message);
    }

    // Step 2: SYNC
    if let Err(e) = state.process_sync() {
        return format!("[Bond Error: SYNC failed: {}]", e.message);
    }

    // Step 3: BOND
    if let Err(e) = state.process_bond() {
        return format!("[Bond Error: BOND failed: {}]", e.message);
    }

    // Step 4: READY
    if let Err(e) = state.process_ready() {
        return format!("[Bond Error: READY failed: {}]", e.message);
    }

    // Verify state is ready
    if !state.is_ready() {
        return "[Bond Error: State did not reach Ready]".to_string();
    }

    // Step 5: INFER - Send prompt to /infer endpoint
    let infer_url = format!("{}/infer", endpoint.trim_end_matches('/'));
    let infer_body = json!({
        "prompt": prompt,
        "context": state.to_context_string(),
        "symbiote_id": state.id,
    });

    match ureq::post(&infer_url)
        .set("Content-Type", "application/json")
        .send_json(&infer_body)
    {
        Ok(res) => match res.into_json::<serde_json::Value>() {
            Ok(json) => {
                if let Some(response) = json.get("response").and_then(|v| v.as_str()) {
                    response.to_string()
                } else {
                    format!("[Bond Error: No 'response' field in infer response]")
                }
            }
            Err(e) => format!("[Bond Error: Failed to parse infer response: {}]", e),
        },
        Err(e) => format!("[Bond Error: Failed to POST to {}: {}]", infer_url, e),
    }
}

fn format_runtime_error(e: &hlx_runtime::RuntimeError) -> anyhow::Error {
    if e.message.starts_with("Shutdown requested") {
        eprintln!("Interrupted.");
        std::process::exit(130);
    }
    let line_info = if e.line > 0 {
        format!(" at line {}", e.line)
    } else {
        String::new()
    };
    let mut msg = format!("Runtime error{}: {} (pc {})", line_info, e.message, e.pc);
    for frame in &e.call_stack {
        msg.push_str(&format!("\n  in {}", frame));
    }
    anyhow::anyhow!("{}", msg)
}

fn main() -> Result<()> {
    // Initialize logging: WARN by default, override with RUST_LOG or --debug/--verbose
    let args = Args::parse();

    let log_level = if args.debug {
        log::LevelFilter::Trace
    } else if args.verbose {
        log::LevelFilter::Info
    } else {
        log::LevelFilter::Warn
    };
    env_logger::Builder::from_env(env_logger::Env::default())
        .filter_level(log_level)
        .format_timestamp(None)
        .init();

    // Ctrl+C shutdown flag — set by signal handler, polled by VM every 1000 steps
    let shutdown_flag = Arc::new(AtomicBool::new(false));
    let shutdown_flag_clone = Arc::clone(&shutdown_flag);
    ctrlc::set_handler(move || {
        if shutdown_flag_clone.swap(true, Ordering::Relaxed) {
            // Second Ctrl+C — force exit immediately
            std::process::exit(130);
        }
        eprintln!("\nInterrupted — finishing current step...");
    })
    .ok(); // Non-fatal if signal registration fails (e.g. in tests)

    if args.repl {
        run_repl(args.verbose, args.max_steps)?;
        return Ok(());
    }

    // Initialize SQLite memory database
    let db_conn = init_memory_db(&args.memory_db)
        .with_context(|| format!("Failed to init memory DB at {}", args.memory_db))?;

    let file = args
        .file
        .context("HLX file required (use --repl for interactive mode)")?;

    // Read source
    let source = fs::read_to_string(&file).with_context(|| format!("Failed to read: {}", file))?;

    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    HLX Program Runner                              ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();
    println!("File: {}", file);
    println!("Source lines: {}", source.lines().count());
    println!();

    // Initialize APE engine for governance
    let ape_engine = if args.no_verify {
        None
    } else {
        // Resolve policy path: try CWD first, then binary location, then HLX_ROOT
        let policy_path = std::path::PathBuf::from(&args.ape_policy);
        let resolved_path = if policy_path.exists() {
            policy_path
        } else if let Ok(exe_path) = std::env::current_exe() {
            if let Some(exe_dir) = exe_path.parent() {
                let binary_relative = exe_dir.join(&args.ape_policy);
                if binary_relative.exists() {
                    binary_relative
                } else if let Ok(hlx_root) = std::env::var("HLX_ROOT") {
                    std::path::PathBuf::from(hlx_root).join(&args.ape_policy)
                } else {
                    policy_path
                }
            } else {
                policy_path
            }
        } else {
            policy_path
        };

        match AxiomEngine::from_file(&resolved_path) {
            Ok(engine) => {
                log::info!(target: "ape", "Governance loaded: {}", resolved_path.display());
                Some(engine)
            }
            Err(e) => {
                log::warn!(
                    target: "ape",
                    "Could not load policy '{}': {}. Running without governance.",
                    resolved_path.display(),
                    e
                );
                None
            }
        }
    };

    // APE: Verify RunCommand intent before compiling
    if let Some(ref engine) = ape_engine {
        let verdict = engine.verify(
            "RunCommand",
            &[("command", "compile"), ("verified", "true")],
        );
        match verdict {
            Ok(v) if !v.allowed() => {
                let reason = v.reason().unwrap_or("policy violation");
                return Err(anyhow::anyhow!("[APE] ✗ Compile blocked: {}", reason));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "[APE] ⚠ Verification error during compile: {}",
                    e
                ));
            }
            _ => {
                log::info!(target: "ape", "Compile verified");
            }
        }
    }

    // Compile using canonical pipeline: AstParser -> Lowerer
    print!("Compiling... ");
    std::io::stdout().flush()?;

    use hlx_runtime::{AstParser, Lowerer, ModuleResolver, Vm};
    use std::path::Path;

    // Phase 1: Parse source to AST
    let program = AstParser::parse(&source)
        .map_err(|e| anyhow::anyhow!("Parse error at line {}: {}", e.line, e.message))?;

    // Phase 1.5: Resolve imports
    let mut resolver = ModuleResolver::new();

    // Add the source file's directory as a search path for relative imports
    if let Some(parent) = Path::new(&file).parent() {
        resolver.add_search_path(parent);
        // Also add the parent/hlx/stdlib for stdlib resolution from project root
        resolver.add_search_path(parent.join("hlx/stdlib"));
    }

    let imported_functions = resolver
        .resolve_program(&program)
        .map_err(|e| anyhow::anyhow!("Module resolution error: {}", e))?;

    // Phase 2: Lower AST to bytecode (associated function, not method)
    let (bytecode, functions) = Lowerer::lower_with_imports(&program, imported_functions)
        .map_err(|e| anyhow::anyhow!("Lower error: {}", e.message))?;

    println!("✓");
    println!("Functions: {:?}", functions.keys().collect::<Vec<_>>());

    if args.verbose {
        println!();
        println!("═══ Bytecode ({} bytes) ═══", bytecode.code.len());
        println!("Constants: {:?}", bytecode.constants);
        println!("Strings: {:?}", bytecode.strings);
    }

    // Run
    println!();
    print!("Executing... ");
    std::io::stdout().flush()?;

    let mut vm = Vm::new()
        .with_max_steps(args.max_steps)
        .with_debug(args.debug)
        .with_memory_limits(args.max_array_size, args.max_string_size)
        .with_shutdown_flag(Arc::clone(&shutdown_flag));

    if let Some(timeout) = args.timeout_ms {
        vm = vm.with_timeout(timeout);
    }

    // Load existing patterns from DB into VM memory at startup
    match load_patterns_from_db(&db_conn, 500) {
        Ok(patterns) => {
            for (pattern, confidence) in patterns {
                vm.mem_store(pattern, confidence);
            }
            log::info!(
                target: "memory",
                "Loaded {} patterns from {}",
                vm.memory().len(),
                args.memory_db
            );
        }
        Err(e) => {
            log::warn!(target: "memory", "Could not load patterns from DB: {}", e);
        }
    }

    // Register native bond() function for HIL inference
    let bond_endpoint = args.bond_endpoint.clone();
    vm.register_native("bond", move |_vm, args| {
        let prompt = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.clone(),
            _ => {
                return hlx_runtime::Value::String("[Error: bond() requires string prompt]".into())
            }
        };
        let context = match args.get(1) {
            Some(hlx_runtime::Value::String(s)) => s.clone(),
            _ => String::new(),
        };

        // If bond endpoint is configured, run the full handshake
        if let Some(ref endpoint) = bond_endpoint {
            let response = run_bond_handshake(endpoint, &prompt, &context);
            hlx_runtime::Value::String(response)
        } else {
            // Stub mode for tests without endpoint
            hlx_runtime::Value::String(format!("[Bond LLM response to: {}]", prompt))
        }
    });

    // Store memory DB path for use in native functions
    let memory_db_path = args.memory_db.clone();

    // Register native memory functions for HIL learn/recall with persistence
    vm.register_native("mem_store", move |vm, args| {
        let pattern = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.clone(),
            _ => return hlx_runtime::Value::Bool(false),
        };
        let confidence = match args.get(1) {
            Some(hlx_runtime::Value::F64(f)) => *f,
            Some(hlx_runtime::Value::I64(i)) => *i as f64,
            _ => return hlx_runtime::Value::Bool(false),
        };
        // Store in VM memory
        vm.mem_store(pattern.clone(), confidence);
        // Store in SQLite DB for persistence (open new connection for thread safety)
        if let Ok(conn) = Connection::open(&memory_db_path) {
            if let Err(e) = store_pattern_in_db(&conn, &pattern, confidence) {
                log::warn!(target: "memory", "Failed to store pattern in DB: {}", e);
            }
        }
        hlx_runtime::Value::Bool(true)
    });

    vm.register_native("mem_query", |vm, args| {
        let query = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.clone(),
            _ => return hlx_runtime::Value::Array(Vec::new()),
        };
        let limit = match args.get(1) {
            Some(hlx_runtime::Value::I64(i)) => *i as usize,
            Some(hlx_runtime::Value::F64(f)) => *f as usize,
            _ => 10,
        };
        // Query VM memory
        let results = vm.mem_query(&query, limit);
        let array_values: Vec<hlx_runtime::Value> = results
            .into_iter()
            .map(hlx_runtime::Value::String)
            .collect();
        hlx_runtime::Value::Array(array_values)
    });

    // Register pattern extraction natives for hil::pattern
    vm.register_native("pat_extract", |_vm, args| {
        // arg[0]: List of String observations
        let observations = match args.get(0) {
            Some(hlx_runtime::Value::Array(arr)) => arr,
            _ => return hlx_runtime::Value::Array(Vec::new()),
        };

        let obs_strings: Vec<&str> = observations
            .iter()
            .filter_map(|v| match v {
                hlx_runtime::Value::String(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();

        if obs_strings.len() < 2 {
            return hlx_runtime::Value::Array(Vec::new());
        }

        // Find common substrings (simple pattern extraction)
        let mut patterns: std::collections::HashMap<String, usize> =
            std::collections::HashMap::new();

        for (i, s1) in obs_strings.iter().enumerate() {
            for s2 in obs_strings.iter().skip(i + 1) {
                // Find longest common substring
                if let Some(lcs) = longest_common_substring(s1, s2) {
                    if lcs.len() >= 4 {
                        // Minimum pattern length
                        *patterns.entry(lcs).or_insert(0) += 1;
                    }
                }
            }
        }

        // Build result list
        let total_pairs = (obs_strings.len() * (obs_strings.len() - 1)) / 2;
        let results: Vec<hlx_runtime::Value> = patterns
            .iter()
            .map(|(pattern, count)| {
                let confidence = *count as f64 / total_pairs as f64;
                let mut map = std::collections::BTreeMap::new();
                map.insert(
                    "pattern".to_string(),
                    hlx_runtime::Value::String(pattern.clone()),
                );
                map.insert(
                    "confidence".to_string(),
                    hlx_runtime::Value::F64(confidence),
                );
                hlx_runtime::Value::Map(map)
            })
            .collect();

        hlx_runtime::Value::Array(results)
    });

    vm.register_native("pat_match", |_vm, args| {
        // arg[0]: observation String, arg[1]: known_patterns List of String
        let observation = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.as_str(),
            _ => return hlx_runtime::Value::Map(std::collections::BTreeMap::new()),
        };

        let patterns = match args.get(1) {
            Some(hlx_runtime::Value::Array(arr)) => arr,
            _ => return hlx_runtime::Value::Map(std::collections::BTreeMap::new()),
        };

        let mut best_match = ("", 0.0);

        for pattern_val in patterns.iter() {
            if let hlx_runtime::Value::String(pattern) = pattern_val {
                let score = compute_similarity(observation, pattern);
                if score > best_match.1 {
                    best_match = (pattern.as_str(), score);
                }
            }
        }

        let mut result = std::collections::BTreeMap::new();
        if best_match.1 > 0.0 {
            result.insert(
                "pattern".to_string(),
                hlx_runtime::Value::String(best_match.0.to_string()),
            );
            result.insert(
                "confidence".to_string(),
                hlx_runtime::Value::F64(best_match.1),
            );
        }

        hlx_runtime::Value::Map(result)
    });

    vm.register_native("pat_matches", |_vm, args| {
        // arg[0]: observation, arg[1]: pattern, arg[2]: min_confidence f64
        let observation = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.as_str(),
            _ => return hlx_runtime::Value::Bool(false),
        };

        let pattern = match args.get(1) {
            Some(hlx_runtime::Value::String(s)) => s.as_str(),
            _ => return hlx_runtime::Value::Bool(false),
        };

        let min_confidence = match args.get(2) {
            Some(hlx_runtime::Value::F64(f)) => *f,
            Some(hlx_runtime::Value::I64(i)) => *i as f64,
            _ => return hlx_runtime::Value::Bool(false),
        };

        let score = compute_similarity(observation, pattern);
        hlx_runtime::Value::Bool(score >= min_confidence)
    });

    vm.register_native("pat_frequency", |_vm, args| {
        // arg[0]: pattern String, arg[1]: observations List
        let pattern = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.as_str(),
            _ => return hlx_runtime::Value::I64(0),
        };

        let observations = match args.get(1) {
            Some(hlx_runtime::Value::Array(arr)) => arr,
            _ => return hlx_runtime::Value::I64(0),
        };

        let count = observations
            .iter()
            .filter(|v| {
                if let hlx_runtime::Value::String(obs) = v {
                    obs.contains(pattern)
                } else {
                    false
                }
            })
            .count();

        hlx_runtime::Value::I64(count as i64)
    });

    // Register eval_hlx for RSI loop - compiles and executes HLX code
    vm.register_native("eval_hlx", |_vm, args| {
        let code = match args.get(0) {
            Some(hlx_runtime::Value::String(s)) => s.as_str(),
            _ => return hlx_runtime::Value::String("[eval error: expected string code]".into()),
        };

        // Compile the HLX code in a fresh compiler
        let eval_result = hlx_runtime::Compiler::compile(code);
        let (eval_bytecode, _eval_functions) = match eval_result {
            Ok((bc, funcs)) => (bc, funcs),
            Err(e) => {
                return hlx_runtime::Value::String(format!(
                    "[eval error: compile failed: {}]",
                    e.message
                ))
            }
        };

        // Execute in a fresh VM (isolated from caller)
        let mut eval_vm = hlx_runtime::Vm::new();
        match eval_vm.run(&eval_bytecode) {
            Ok(result) => hlx_runtime::Value::String(format!("{}", result)),
            Err(e) => hlx_runtime::Value::String(format!("[eval error: runtime: {}]", e.message)),
        }
    });

    // Register functions with VM
    let bytecode_hex = bytecode
        .code
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");
    log::debug!(target: "hlx_run", "Bytecode: {}", bytecode_hex);
    for (name, (start_pc, params)) in &functions {
        log::debug!(
            target: "hlx_run",
            "Registering function: {} at PC {} with {} params",
            name, start_pc, params
        );
        vm.register_function(name, *start_pc as usize, *params as usize);
    }

    // Execute: either call specific function or run from start
    let result = if let Some(func_name) = args.func {
        // First, run __top_level__ to initialize module-level latent variables
        if functions.contains_key("__top_level__") {
            log::debug!(target: "hlx_run", "Running __top_level__ initialization...");
            vm.call_function(&bytecode, "__top_level__", &[])
                .map_err(|e| {
                    anyhow::anyhow!("__top_level__ initialization error: {}", e.message)
                })?;
        }

        // Call specific exported function with arguments
        let func_args: Vec<hlx_runtime::Value> = args
            .args
            .iter()
            .map(|arg| {
                // Try to parse as number, fallback to string
                if let Ok(n) = arg.parse::<i64>() {
                    hlx_runtime::Value::I64(n)
                } else if let Ok(f) = arg.parse::<f64>() {
                    hlx_runtime::Value::F64(f)
                } else {
                    hlx_runtime::Value::String(arg.clone())
                }
            })
            .collect();

        log::debug!(target: "hlx_run", "Calling function: {} with {} args", func_name, func_args.len());
        vm.call_function(&bytecode, &func_name, &func_args)
            .map_err(|e| format_runtime_error(&e))?
    } else {
        // Run from beginning (default behavior)
        vm.run(&bytecode).map_err(|e| format_runtime_error(&e))?
    };

    println!("✓");
    println!();

    // APE: Verify output before displaying (Governance at the boundary)
    if let Some(ref engine) = ape_engine {
        let result_str = format!("{}", result);
        let verdict = engine.verify(
            "GenerateResponse",
            &[
                ("output", &result_str),
                ("verified", "true"), // Required for Execute-class intents
            ],
        );
        match verdict {
            Ok(v) if !v.allowed() => {
                let reason = v.reason().unwrap_or("policy violation");
                return Err(anyhow::anyhow!("[APE] ✗ Output blocked: {}", reason));
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "[APE] ⚠ Verification error for output: {}",
                    e
                ));
            }
            _ => {
                log::info!(target: "ape", "Output verified");
            }
        }
    }

    println!("═══ Result ═══");
    println!("{}", result);
    println!();

    Ok(())
}

fn run_repl(verbose: bool, max_steps: usize) -> Result<()> {
    println!("╔══════════════════════════════════════════════════════════════════════╗");
    println!("║                    HLX REPL                                        ║");
    println!("║  Type HLX code, :help for commands, :quit to exit               ║");
    println!("╚══════════════════════════════════════════════════════════════════════╝");
    println!();

    use hlx_runtime::{Compiler, Vm};

    let mut source = String::new();

    loop {
        print!("hlx> ");
        std::io::stdout().flush()?;

        let mut line = String::new();
        std::io::stdin().read_line(&mut line)?;
        let line = line.trim();

        if line == ":quit" || line == ":q" || line == "exit" {
            println!("Goodbye!");
            break;
        }

        if line == ":help" || line == ":h" {
            println!("Commands:");
            println!("  :help, :h   - Show this help");
            println!("  :run, :r    - Execute current source");
            println!("  :clear, :c  - Clear current source");
            println!("  :quit, :q   - Exit REPL");
            println!();
            println!("Any other input is appended to the current source.");
            println!("Execute with :run");
            continue;
        }

        if line == ":run" || line == ":r" {
            if source.trim().is_empty() {
                println!("No source to run");
                continue;
            }

            print!("Compiling... ");
            std::io::stdout().flush()?;

            use hlx_runtime::{AstParser, Lowerer, Vm};

            match AstParser::parse(&source) {
                Ok(program) => {
                    match Lowerer::lower(&program) {
                        Ok((bytecode, functions)) => {
                            println!("✓");
                            println!("Functions: {:?}", functions.keys().collect::<Vec<_>>());

                            if verbose {
                                println!("Constants: {:?}", bytecode.constants);
                            }

                            print!("Executing... ");
                            std::io::stdout().flush()?;

                            let mut vm = Vm::new().with_max_steps(max_steps);

                            // Register functions with VM
                            log::debug!(target: "hlx_run", "Bytecode len: {}", bytecode.code.len());
                            for (name, (start_pc, params)) in &functions {
                                log::debug!(
                                    target: "hlx_run",
                                    "Registering function: {} at PC {} with {} params",
                                    name, start_pc, params
                                );
                                vm.register_function(name, *start_pc as usize, *params as usize);
                            }

                            // Entry point is PC 0 (main is lowered first)

                            match vm.run(&bytecode) {
                                Ok(result) => {
                                    println!("✓");
                                    println!("=> {}", result);
                                }
                                Err(e) => {
                                    println!("Error: {}", e.message);
                                }
                            }
                        }
                        Err(e) => {
                            println!("Lower error: {}", e.message);
                        }
                    }
                }
                Err(e) => {
                    println!("Parse error: {}", e.message);
                }
            }

            source.clear();
            continue;
        }

        if line == ":clear" || line == ":c" {
            source.clear();
            println!("Source cleared");
            continue;
        }

        // Append line to source
        source.push_str(line);
        source.push('\n');
    }

    Ok(())
}
