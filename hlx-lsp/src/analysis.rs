use hlx_runtime::{AstParser, Item};
use tower_lsp::lsp_types::*;

/// Get diagnostics from parsing HLX code
pub fn get_diagnostics(content: &str) -> Vec<Diagnostic> {
    match AstParser::parse(content) {
        Ok(_) => vec![],
        Err(e) => {
            let line = if e.line > 0 { e.line - 1 } else { 0 } as u32;
            let col = if e.col > 0 { e.col - 1 } else { 0 } as u32;

            vec![Diagnostic {
                range: Range {
                    start: Position { line, character: col },
                    end: Position { line, character: col + 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("hlx".to_string()),
                message: e.message,
                related_information: None,
                tags: None,
                data: None,
            }]
        }
    }
}

/// Get hover information at a position
pub fn get_hover_info(content: &str, line: u32, character: u32) -> Option<String> {
    let word = get_word_at_position(content, line, character)?;

    // Check builtins
    if let Some(desc) = get_builtin_description(&word) {
        return Some(desc);
    }

    // Check for keywords
    let keywords: &[(&str, &str)] = &[
        ("agent", "Defines a recursive agent with governance"),
        ("recursive", "Marks an agent as recursively self-improving"),
        ("fn", "Defines a function"),
        ("let", "Defines a variable binding"),
        ("if", "Conditional statement"),
        ("else", "Alternative branch for if"),
        ("loop", "Loop with condition and max iterations"),
        ("return", "Return from function"),
        ("import", "Import from another module"),
        ("export", "Export for other modules"),
        ("struct", "Define a data structure"),
        ("module", "Define a module namespace"),
        ("govern", "Governance block — declares effect class, conscience predicate, and trust requirements"),
        ("scale", "Multi-agent coordination cluster"),
        ("cycle", "Reasoning cycle (H = high frequency, L = low frequency)"),
        ("modify", "Self-modification block for RSI (Recursive Self-Improvement)"),
        ("proof", "Proof gate — requires formal verification before promotion"),
        ("consensus", "Consensus gate — requires multi-agent agreement"),
        ("human", "Human authorization gate — requires human-in-the-loop approval"),
        ("migrate", "Migrate an agent between scales"),
        ("intent", "Declares an intent — a named action requiring conscience verification"),
        ("do", "Execute an intent: `do IntentName { field: value }`"),
        ("contract", "Contract expression — declares a typed interface"),
        ("effect", "Effect class declaration within a govern block (Execute, Write, Read, Network, ModifyAgent)"),
        ("conscience", "Conscience predicate within a govern block (G1-G6)"),
        ("latent", "Declares a latent state tensor within an agent"),
        ("spawn", "Spawn a new agent instance"),
        ("dissolvable", "Marks a spawned agent as reclaimable by GC"),
        ("barrier", "Synchronization barrier for multi-agent coordination"),
        ("try", "Begin a try block for catchable error handling"),
        ("catch", "Catch a non-fatal error from a try block"),
        ("throw", "Throw a catchable error"),
        ("for", "For loop — iterates over a range or collection"),
        ("while", "While loop — repeats while condition is true"),
        ("match", "Pattern matching expression"),
        ("switch", "Switch statement — dispatches on discriminant value"),
        ("extern", "Declare an external (FFI) function"),
        ("collapse", "Collapse a latent tensor to a concrete value"),
        ("resolve", "Resolve a latent tensor (alias for collapse)"),
        ("const", "Constant binding"),
        ("mut", "Mutable binding"),
    ];

    for (kw, desc) in keywords {
        if *kw == word {
            return Some(format!("**{}** (keyword)\n\n{}", kw, desc));
        }
    }

    None
}

/// Get completion items
pub fn get_completions(_content: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();

    let keywords = &[
        "agent", "recursive", "fn", "let", "if", "else", "loop",
        "return", "import", "export", "struct", "module", "govern",
        "scale", "cycle", "modify", "proof", "consensus", "human",
        "migrate", "to", "intent", "do", "contract", "effect",
        "conscience", "try", "catch", "throw", "latent", "spawn",
        "takes", "gives", "gate", "budget", "cooldown", "approve",
        "action", "when", "const", "mut", "collapse", "resolve",
        "match", "switch", "case", "default", "while", "for", "as",
        "extern", "dissolvable", "inherit", "barrier", "sync",
        "true", "false", "nil",
    ];
    for kw in keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }

    // Add builtins
    for (name, desc) in get_all_builtins() {
        items.push(CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("builtin".to_string()),
            documentation: Some(Documentation::String(desc)),
            ..Default::default()
        });
    }

    items
}

/// Go to definition — uses the real AST parser to find function/agent definitions
pub fn goto_definition(content: &str, line: u32, character: u32, uri: Url) -> Option<Location> {
    let word = get_word_at_position(content, line, character)?;

    // Parse the AST and collect definitions
    let program = AstParser::parse(content).ok()?;

    for item in &program.items {
        match item {
            Item::Function(f) => {
                if f.name == word {
                    let def_line = if f.span.start_line > 0 { f.span.start_line - 1 } else { 0 } as u32;
                    return Some(Location {
                        uri,
                        range: Range {
                            start: Position { line: def_line, character: 0 },
                            end: Position { line: def_line, character: f.name.len() as u32 + 3 },
                        },
                    });
                }
            }
            Item::Agent(a) => {
                if a.name == word {
                    let def_line = if a.span.start_line > 0 { a.span.start_line - 1 } else { 0 } as u32;
                    return Some(Location {
                        uri,
                        range: Range {
                            start: Position { line: def_line, character: 0 },
                            end: Position { line: def_line, character: a.name.len() as u32 + 6 },
                        },
                    });
                }
            }
            Item::Module(m) => {
                if m.name == word {
                    return Some(Location {
                        uri,
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: m.name.len() as u32 + 7 },
                        },
                    });
                }
            }
            Item::Struct(s) => {
                if s.name == word {
                    return Some(Location {
                        uri,
                        range: Range {
                            start: Position { line: 0, character: 0 },
                            end: Position { line: 0, character: s.name.len() as u32 + 7 },
                        },
                    });
                }
            }
            _ => {}
        }
    }

    None
}

/// Get document symbols (outline view)
#[allow(deprecated)]
pub fn get_document_symbols(content: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Agent definitions
        if trimmed.starts_with("agent ") || trimmed.starts_with("recursive agent ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "agent") {
                symbols.push(make_symbol(name, "agent", SymbolKind::CLASS, line_num, line.len()));
            }
        }

        // Function definitions
        if trimmed.starts_with("fn ") || trimmed.starts_with("export fn ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "fn") {
                symbols.push(make_symbol(name, "function", SymbolKind::FUNCTION, line_num, line.len()));
            }
        }

        // Struct definitions
        if trimmed.starts_with("struct ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "struct") {
                symbols.push(make_symbol(name, "struct", SymbolKind::STRUCT, line_num, line.len()));
            }
        }

        // Module definitions
        if trimmed.starts_with("module ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "module") {
                symbols.push(make_symbol(name, "module", SymbolKind::MODULE, line_num, line.len()));
            }
        }

        // Import statements
        if trimmed.starts_with("import ") || trimmed.starts_with("use ") {
            let detail = trimmed.to_string();
            symbols.push(make_symbol(detail.clone(), "import", SymbolKind::NAMESPACE, line_num, line.len()));
        }

        // Latent declarations inside agents
        if trimmed.starts_with("latent ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "latent") {
                symbols.push(make_symbol(name, "latent tensor", SymbolKind::VARIABLE, line_num, line.len()));
            }
        }

        // Intent declarations
        if trimmed.starts_with("intent ") {
            if let Some(name) = extract_name_after_keyword(trimmed, "intent") {
                symbols.push(make_symbol(name, "intent", SymbolKind::EVENT, line_num, line.len()));
            }
        }
    }

    symbols
}

#[allow(deprecated)]
fn make_symbol(name: String, detail: &str, kind: SymbolKind, line_num: usize, line_len: usize) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail: Some(detail.to_string()),
        kind,
        tags: None,
        deprecated: None,
        range: Range {
            start: Position { line: line_num as u32, character: 0 },
            end: Position { line: line_num as u32, character: line_len as u32 },
        },
        selection_range: Range {
            start: Position { line: line_num as u32, character: 0 },
            end: Position { line: line_num as u32, character: line_len as u32 },
        },
        children: None,
    }
}

fn get_word_at_position(content: &str, line: u32, character: u32) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if line as usize >= lines.len() {
        return None;
    }

    let line_content = lines[line as usize];
    let chars: Vec<char> = line_content.chars().collect();

    if character as usize >= chars.len() {
        return None;
    }

    let mut start = character as usize;
    let mut end = character as usize;

    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }

    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    if start >= end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == ':'
}

fn extract_name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == keyword && i + 1 < parts.len() {
            let name = parts[i + 1];
            return Some(name.trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string());
        }
    }
    None
}

fn get_builtin_description(name: &str) -> Option<String> {
    let desc = match name {
        "print" => "Print a value to stdout",
        "println" => "Print a value to stdout with newline",
        "strlen" => "Get the length of a string",
        "concat" => "Concatenate two strings",
        "substring" => "Extract a substring: substring(str, start, len)",
        "ord" => "Get the ASCII/Unicode code point of a character",
        "char" => "Convert a code point to a character",
        "sqrt" => "Calculate square root",
        "abs" => "Calculate absolute value",
        "floor" => "Floor of a float",
        "ceil" => "Ceiling of a float",
        "pow" => "Raise to a power: pow(base, exp)",
        "log" => "Natural logarithm",
        "sin" => "Sine (radians)",
        "cos" => "Cosine (radians)",
        "min" => "Minimum of two values",
        "max" => "Maximum of two values",
        "rand" => "Deterministic PRNG float in [0, 1) — Blake3-seeded",
        "rand_range" => "Deterministic PRNG integer in [min, max]",
        "native_rand" => "Host OS random (non-deterministic, requires Read effect)",
        "read_file" => "Read a file (sandboxed, TOCTOU-safe)",
        "write_file" => "Write a file (sandboxed, TOCTOU-safe)",
        "bond" => "Call bonded LLM (Qwen3-4B Q8) for inference",
        "mem_query_vec" => "Native vector similarity search in memory pool",
        "mem_store_vec" => "Store a vector embedding in memory pool",
        "__native_embed" => "Generate embedding vector via bonded model",
        "tensor_create" => "Create a zero tensor: tensor_create(shape_array)",
        "tensor_from_data" => "Create tensor from shape + data arrays",
        "tensor_from_json" => "Parse JSON {\"shape\":[...],\"data\":[...]} into a tensor",
        "tensor_blend" => "Blend two tensors: tensor_blend(a, b, alpha)",
        "tensor_slice" => "Slice a tensor along an axis",
        "tensor_merge" => "Merge two tensors",
        "tensor_convolve" => "Convolve two tensors",
        "tensor_correlate" => "Compute tensor correlation",
        "tensor_normalize" => "Normalize a tensor to unit length",
        "tensor_topology_score" => "Compute topology score of a tensor",
        "native_zeros" => "Create a zero-filled float array",
        "set_tensor" => "Set a value in a tensor at index",
        "get_tensor" => "Get a value from a tensor at index",
        "snapshot" => "Serialize VM heap state to JSON",
        "restore" => "Restore VM heap state from JSON snapshot",
        "poll_inbox" => "Non-blocking pop from agent inbox (returns nil if empty)",
        "drain_sync_events" => "Drain queued sync events (RSI, promotions) for SQLite flush",
        "fitness_snapshot" => "Capture current agent fitness metrics",
        "fitness_compare" => "Compare two fitness snapshots",
        "reload_governance" => "Hot-reload governance policy from file",
        "send_message" => "Send a message to another agent",
        "receive_message" => "Receive a message from the agent's mailbox",
        "get_substrate_pressure" => "Get current substrate memory pressure",
        "patch_module" => "Hot-patch a module at runtime",
        "clock_ms" => "Get logical clock time in milliseconds",
        _ => return None,
    };
    Some(desc.to_string())
}

fn get_all_builtins() -> Vec<(String, String)> {
    let names = [
        "print", "println", "strlen", "concat", "substring", "ord", "char",
        "sqrt", "abs", "floor", "ceil", "pow", "log", "sin", "cos", "min", "max",
        "rand", "rand_range", "native_rand",
        "read_file", "write_file",
        "bond", "mem_query_vec", "mem_store_vec", "__native_embed",
        "tensor_create", "tensor_from_data", "tensor_from_json",
        "tensor_blend", "tensor_slice", "tensor_merge",
        "tensor_convolve", "tensor_correlate", "tensor_normalize", "tensor_topology_score",
        "native_zeros", "set_tensor", "get_tensor",
        "snapshot", "restore", "poll_inbox", "drain_sync_events",
        "fitness_snapshot", "fitness_compare",
        "reload_governance", "send_message", "receive_message",
        "get_substrate_pressure", "patch_module", "clock_ms",
    ];
    names.iter()
        .filter_map(|&n| get_builtin_description(n).map(|d| (n.to_string(), d)))
        .collect()
}
