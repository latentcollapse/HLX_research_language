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

fn main() -> Result<()> {
    let args = Args::parse();

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
                eprintln!("[APE] Governance loaded: {}", resolved_path.display());
                Some(engine)
            }
            Err(e) => {
                eprintln!(
                    "[APE] Warning: Could not load policy '{}': {}",
                    resolved_path.display(),
                    e
                );
                eprintln!(
                    "[APE] Running without governance. Use --no-verify to suppress this warning."
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
                eprintln!("[APE] ✓ Compile verified");
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

    let mut vm = Vm::new().with_max_steps(args.max_steps);

    // Load existing patterns from DB into VM memory at startup
    match load_patterns_from_db(&db_conn, 500) {
        Ok(patterns) => {
            for (pattern, confidence) in patterns {
                vm.mem_store(pattern, confidence);
            }
            eprintln!(
                "[Memory] Loaded {} patterns from {}",
                vm.memory().len(),
                args.memory_db
            );
        }
        Err(e) => {
            eprintln!("[Memory] Warning: Could not load patterns from DB: {}", e);
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
                eprintln!("[Memory] Warning: Failed to store pattern in DB: {}", e);
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

    // Register functions with VM
    let bytecode_hex = bytecode
        .code
        .iter()
        .map(|b| format!("{:02x}", b))
        .collect::<Vec<_>>()
        .join(" ");
    eprintln!("Bytecode: {}", bytecode_hex);
    for (name, (start_pc, params)) in &functions {
        eprintln!(
            "Registering function: {} at PC {} with {} params",
            name, start_pc, params
        );
        vm.register_function(name, *start_pc as usize, *params as usize);
    }

    let result = vm
        .run(&bytecode)
        .map_err(|e| anyhow::anyhow!("Runtime error: {}", e.message))?;

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
                eprintln!("[APE] ✓ Output verified");
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
                            eprintln!("Bytecode len: {}", bytecode.code.len());
                            eprintln!(
                                "Bytecode: {:?}",
                                &bytecode.code[..bytecode.code.len().min(100)]
                            );
                            for (name, (start_pc, params)) in &functions {
                                eprintln!(
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
