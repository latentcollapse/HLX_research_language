//! HLX Command-Line Interface
//! 
//! ```bash
//! # Compile HLXA to crate
//! hlx compile program.hlxa -o program.lcc
//! 
//! # Run a crate
//! hlx run program.lcc
//! 
//! # Transliterate between forms
//! hlx translate --from hlxa --to hlxr program.hlxa
//! 
//! # Inspect a crate
//! hlx inspect program.lcc
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;

use hlx_core::{HlxCrate, Value, lcb, RuntimeCapabilities, BuiltinSpecBuilder, StabilityLevel};
use hlx_compiler::{HlxaParser, HlxaEmitter, RunicEmitter, parser::Parser as ParseTrait, Emitter, lower};
use hlx_runtime::execute;

#[derive(Parser)]
#[command(name = "hlx")]
#[command(author, version, about = "HLX Compiler and Runtime", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
    
    /// Enable verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Compile HLXA/HLXR source to LC-B crate
    Compile {
        /// Input source file
        input: PathBuf,
        
        /// Output crate file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Input format (hlxa or hlxr)
        #[arg(long, default_value = "hlxa")]
        format: String,
    },
    
    /// Run a crate or source file
    Run {
        /// Input file (crate or source)
        input: PathBuf,

        /// Force CPU backend
        #[arg(long)]
        cpu: bool,

        /// Output file for result (LC-B encoded)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Input data to pass to main() function (as file path)
        #[arg(long)]
        main_input: Option<PathBuf>,
    },
    
    /// Transliterate between HLXA and HLXR
    Translate {
        /// Input source file
        input: PathBuf,
        
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Source format
        #[arg(long, default_value = "hlxa")]
        from: String,
        
        /// Target format
        #[arg(long, default_value = "hlxr")]
        to: String,
    },
    
    /// Inspect a crate
    Inspect {
        /// Crate file to inspect
        input: PathBuf,
        
        /// Output as JSON
        #[arg(long)]
        json: bool,
    },
    
    /// Encode a value to LC-B
    Encode {
        /// Value as JSON
        value: String,
        
        /// Output file (stdout hex if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },
    
    /// Decode LC-B to value
    Decode {
        /// Input file or hex string
        input: String,
        
        /// Input is hex string (not file)
        #[arg(long)]
        hex: bool,
    },
    
    /// Convert raw instruction values to LC-B crate
    BuildCrate {
        /// Input file (LC-B encoded Value)
        input: PathBuf,
        
        /// Output crate file
        #[arg(short, long)]
        output: PathBuf,
    },

    /// Replay a crash dump (snapshot)
    Replay {
        /// Path to the snapshot JSON file
        input: PathBuf,
    },

    /// Run smoke tests
    Test,

    /// Benchmark execution performance
    Bench {
        /// Input file (crate or source)
        input: PathBuf,

        /// Enable HLX-Scale speculation
        #[arg(long)]
        hlx_s: bool,

        /// Number of iterations
        #[arg(short = 'n', long, default_value = "10")]
        iterations: usize,

        /// Output as JSON
        #[arg(long)]
        json: bool,

        /// Generate flamegraph
        #[arg(long)]
        flamegraph: bool,

        /// Flamegraph sampling frequency (Hz)
        #[arg(long, default_value = "1000")]
        frequency: u32,
    },

    /// Compile to native code using LLVM backend
    CompileNative {
        /// Input source file
        input: PathBuf,

        /// Output file (.o object file, .s assembly, or .so/.dll shared library)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target triple (e.g., "x86_64-unknown-none-elf" for bare metal)
        #[arg(long)]
        target: Option<String>,

        /// Emit assembly instead of object file
        #[arg(long)]
        asm: bool,

        /// Emit shared library (.so/.dll/.dylib)
        #[arg(long)]
        shared: bool,

        /// Print LLVM IR to stderr
        #[arg(long)]
        print_ir: bool,
    },

    /// Emit runtime capabilities schema
    Capabilities {
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Output format (json or toml)
        #[arg(long, default_value = "json")]
        format: String,
    },

    /// Generate C header file from FFI exports
    GenerateHeader {
        /// Input crate file
        input: PathBuf,

        /// Output header file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
    },

    /// Generate Python wrapper from FFI exports
    GeneratePython {
        /// Input crate file
        input: PathBuf,

        /// Output Python file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Library name (defaults to crate name without extension)
        #[arg(long)]
        lib_name: Option<String>,
    },

    /// Generate Rust wrapper from FFI exports
    GenerateRust {
        /// Input crate file
        input: PathBuf,

        /// Output Rust file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Library name (defaults to crate name without extension)
        #[arg(long)]
        lib_name: Option<String>,

        /// Also generate Cargo.toml
        #[arg(long)]
        cargo_toml: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Setup logging - initialize if --verbose or RUST_LOG is set
    if cli.verbose || std::env::var("RUST_LOG").is_ok() {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
            .with_target(false)
            .init();
    }
    
    match cli.command {
        Commands::Compile { input, output, format } => {
            compile(&input, output, &format)?;
        }
        Commands::Run { input, cpu, output, main_input } => {
            run(&input, cpu, output, main_input)?;
        }
        Commands::Translate { input, output, from, to } => {
            translate(&input, output, &from, &to)?;
        }
        Commands::Inspect { input, json } => {
            inspect(&input, json)?;
        }
        Commands::Encode { value, output } => {
            encode(&value, output)?;
        }
        Commands::Decode { input, hex } => {
            decode(&input, hex)?;
        }
        Commands::BuildCrate { input, output } => {
            build_crate(&input, &output)?;
        }
        Commands::Replay { input } => {
            replay(&input)?;
        }
        Commands::Test => {
            run_tests()?;
        }
        Commands::Bench { input, hlx_s, iterations, json, flamegraph, frequency } => {
            bench(&input, hlx_s, iterations, json, flamegraph, frequency)?;
        }
        Commands::CompileNative { input, output, target, asm, shared, print_ir } => {
            compile_native(&input, output, target.as_deref(), asm, shared, print_ir)?;
        }
        Commands::Capabilities { output, format } => {
            emit_capabilities(output, &format)?;
        }

        Commands::GenerateHeader { input, output } => {
            generate_header(&input, output.as_ref())?;
        }

        Commands::GeneratePython { input, output, lib_name } => {
            generate_python(&input, output.as_ref(), lib_name.as_deref())?;
        }

        Commands::GenerateRust { input, output, lib_name, cargo_toml } => {
            generate_rust(&input, output.as_ref(), lib_name.as_deref(), cargo_toml)?;
        }
    }

    Ok(())
}

fn compile(input: &PathBuf, output: Option<PathBuf>, format: &str) -> Result<()> {
    let source = fs::read_to_string(input)
        .context("Failed to read input file")?;
    
    // Parse source
    let ast = match format {
        "hlxa" | "hlxl" | "hlxc" => {
            let parser = HlxaParser::new();
            parser.parse(&source).context("Parse error")?
        }
        "hlxr" | "hlx" => {
            let hlxa = hlx_compiler::runic::transliterate_to_hlxl(&source)?;
            let parser = HlxaParser::new();
            parser.parse(&hlxa).context("Parse error")?
        }
        _ => anyhow::bail!("Unknown format: {}", format),
    };
    
    // Lower to crate
    let krate = lower::lower_to_crate(&ast)
        .context("Lowering failed")?;
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut p = input.clone();
        p.set_extension("lcc");
        p
    });
    
    // Serialize crate
    let bytes = bincode::serialize(&krate)
        .context("Serialization failed")?;
    
    fs::write(&output_path, &bytes)
        .context("Failed to write output")?;
    
    println!("Compiled {} -> {}", input.display(), output_path.display());
    println!("  Instructions: {}", krate.len());
    println!("  Hash: {}", hex::encode(&krate.hash[..8]));
    
    Ok(())
}

fn run(input: &PathBuf, cpu_only: bool, output: Option<PathBuf>, main_input: Option<PathBuf>) -> Result<()> {
    let mut config = hlx_runtime::RuntimeConfig::default();
    config.debug = false; // Disable debug trace
    if cpu_only {
        config.backend = hlx_runtime::config::BackendType::Cpu;
    }

    // Load main input if provided
    if let Some(input_path) = main_input {
        let input_data = fs::read_to_string(&input_path)
            .context("Failed to read main_input file")?;
        config.main_input = Some(input_data);
    }

    // Check if input is a crate or source
    let ext = input.extension().and_then(|e| e.to_str()).unwrap_or("");

    let krate = if ext == "lcc" || ext == "lcb" {
        // Load crate
        let bytes = fs::read(input).context("Failed to read crate")?;
        bincode::deserialize(&bytes).context("Failed to deserialize crate")?
    } else {
        // Compile source first
        let source = fs::read_to_string(input).context("Failed to read source")?;
        let parser = HlxaParser::new();
        let ast = parser.parse(&source).context("Parse error")?;
        lower::lower_to_crate(&ast).context("Lowering failed")?
    };

    // Execute
    let result = hlx_runtime::execute_with_config(&krate, &config)
        .context("Execution failed")?;

    if let Some(path) = output {
        let bytes = lcb::encode(&result).context("Encoding failed")?;
        fs::write(&path, &bytes).context("Failed to write output")?;
        println!("Output written to {}", path.display());
    } else {
        println!("{}", result);
    }

    Ok(())
}

fn translate(input: &PathBuf, output: Option<PathBuf>, from: &str, to: &str) -> Result<()> {
    let source = fs::read_to_string(input)
        .context("Failed to read input")?;
    
    // Parse
    let ast = match from {
        "hlxa" | "hlxl" | "hlxc" => {
            let parser = HlxaParser::new();
            parser.parse(&source).context("Parse error")?
        }
        "hlxr" | "hlx" => {
            let hlxa = hlx_compiler::runic::transliterate_to_hlxl(&source)?;
            let parser = HlxaParser::new();
            parser.parse(&hlxa).context("Parse error")?
        }
        _ => anyhow::bail!("Unknown source format: {}", from),
    };
    
    // Emit
    let result = match to {
        "hlxa" | "hlxl" | "hlxc" => {
            let emitter = HlxaEmitter::new();
            emitter.emit(&ast)?
        }
        "hlxr" | "hlx" => {
            let emitter = RunicEmitter::new();
            emitter.emit(&ast)?
        }
        _ => anyhow::bail!("Unknown target format: {}", to),
    };
    
    // Output
    if let Some(path) = output {
        fs::write(&path, &result).context("Failed to write output")?;
        println!("Translated to {}", path.display());
    } else {
        println!("{}", result);
    }
    
    Ok(())
}

fn inspect(input: &PathBuf, json: bool) -> Result<()> {
    let bytes = fs::read(input).context("Failed to read crate")?;
    let krate: HlxCrate = bincode::deserialize(&bytes)
        .context("Failed to deserialize crate")?;
    
    if json {
        let info = serde_json::json!({
            "version": krate.version,
            "hash": hex::encode(&krate.hash),
            "instruction_count": krate.len(),
            "is_valid": krate.is_valid(),
            "metadata": krate.metadata,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Crate: {}", input.display());
        println!("  Version: {}", krate.version);
        println!("  Hash: {}", hex::encode(&krate.hash));
        println!("  Instructions: {}", krate.len());
        println!("  Valid: {}", krate.is_valid());
        
        if let Some(meta) = &krate.metadata {
            if let Some(src) = &meta.source_file {
                println!("  Source: {}", src);
            }
            if let Some(ver) = &meta.compiler_version {
                println!("  Compiler: {}", ver);
            }
            if !meta.debug_symbols.is_empty() {
                println!("  Debug symbols: {} entries", meta.debug_symbols.len());
            }
            if !meta.ffi_exports.is_empty() {
                println!("  FFI Exports:");
                for (name, info) in &meta.ffi_exports {
                    let attrs = if info.no_mangle && info.export {
                        "#[no_mangle] #[export]"
                    } else if info.no_mangle {
                        "#[no_mangle]"
                    } else {
                        "#[export]"
                    };
                    println!("    {} {} -> {:?}", attrs, name, info.return_type);
                }
            }
        }

        // Build debug symbol map for display
        let mut debug_map = std::collections::HashMap::new();
        if let Some(meta) = &krate.metadata {
            for sym in &meta.debug_symbols {
                debug_map.insert(sym.inst_idx, (sym.line, sym.col));
            }
        }

        println!("\nInstructions:");
        for (i, inst) in krate.instructions.iter().enumerate() {
            if let Some((line, col)) = debug_map.get(&i) {
                println!("  {:4}: [line {}:{}] {:?}", i, line, col, inst);
            } else {
                println!("  {:4}: {:?}", i, inst);
            }
        }
    }
    
    Ok(())
}

fn encode(value_json: &str, output: Option<PathBuf>) -> Result<()> {
    let value: serde_json::Value = serde_json::from_str(value_json)
        .context("Invalid JSON")?;
    
    let hlx_value = json_to_value(&value)?;
    let encoded = lcb::encode(&hlx_value).context("Encoding failed")?;
    
    if let Some(path) = output {
        fs::write(&path, &encoded).context("Failed to write")?;
        println!("Encoded {} bytes to {}", encoded.len(), path.display());
    } else {
        println!("{}", hex::encode(&encoded));
    }
    
    Ok(())
}

fn decode(input: &str, is_hex: bool) -> Result<()> {
    let bytes = if is_hex {
        hex::decode(input).context("Invalid hex")?
    } else {
        fs::read(input).context("Failed to read file")?
    };
    
    let value = lcb::decode(&bytes).context("Decoding failed")?;
    
    // Convert to JSON for display
    let json = value_to_json(&value);
    println!("{}", serde_json::to_string_pretty(&json)?);
    
    Ok(())
}

/// Transform a raw JSON value to tagged enum format for Value deserialization
fn transform_value_to_tagged(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Null => serde_json::json!("Null"),
        serde_json::Value::Bool(b) => serde_json::json!({"Boolean": b}),
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                serde_json::json!({"Integer": n.as_i64().unwrap()})
            } else {
                serde_json::json!({"Float": n.as_f64().unwrap()})
            }
        }
        serde_json::Value::String(s) => serde_json::json!({"String": s}),
        serde_json::Value::Array(arr) => {
            let transformed: Vec<serde_json::Value> = arr.iter()
                .map(transform_value_to_tagged)
                .collect();
            serde_json::json!({"Array": transformed})
        }
        serde_json::Value::Object(obj) => {
            let transformed: serde_json::Map<String, serde_json::Value> = obj.iter()
                .map(|(k, v)| (k.clone(), transform_value_to_tagged(v)))
                .collect();
            serde_json::json!({"Object": transformed})
        }
    }
}

fn build_crate(input: &PathBuf, output: &PathBuf) -> Result<()> {
    let bytes = fs::read(input).context("Failed to read input")?;
    let value = lcb::decode(&bytes).context("Failed to decode LC-B")?;

    // Convert Value -> JSON
    let json = value_to_json(&value);

    // Transform internally tagged {"op": "Name", ...} to externally tagged {"Name": {...}}
    // Also transform val fields for Constant instructions and field name mappings
    let instructions_json = if let serde_json::Value::Array(arr) = json {
        let new_arr: Vec<serde_json::Value> = arr.into_iter().map(|mut v| {
            if let Some(map) = v.as_object_mut() {
                if let Some(serde_json::Value::String(op)) = map.remove("op") {
                    // Transform val field if this is a Constant instruction
                    if op == "Constant" {
                        if let Some(val) = map.get("val") {
                            let transformed_val = transform_value_to_tagged(val);
                            map.insert("val".to_string(), transformed_val);
                        }
                    }
                    // Map If instruction field names
                    if op == "If" {
                        if let Some(then_val) = map.remove("then") {
                            map.insert("then_block".to_string(), then_val);
                        }
                        if let Some(else_val) = map.remove("else_target") {
                            map.insert("else_block".to_string(), else_val);
                        }
                    }
                    // Handle unit variants (no fields)
                    if map.is_empty() && (op == "Break" || op == "Continue") {
                        return serde_json::Value::String(op);
                    }
                    let mut wrapper = serde_json::Map::new();
                    let inner_map = std::mem::take(map);
                    wrapper.insert(op, serde_json::Value::Object(inner_map));
                    return serde_json::Value::Object(wrapper);
                }
            }
            v
        }).collect();
        serde_json::Value::Array(new_arr)
    } else {
        json
    };

    // The value should be an Array of Instructions (Objects)
    let instructions: Vec<hlx_core::Instruction> = serde_json::from_value(instructions_json)
        .context("Failed to convert Value to Instructions")?;

    let krate = HlxCrate::new(instructions);

    // Serialize crate
    let bytes = bincode::serialize(&krate)
        .context("Serialization failed")?;

    fs::write(output, &bytes)
        .context("Failed to write output")?;

    println!("Built crate {} -> {}", input.display(), output.display());
    println!("  Instructions: {}", krate.len());
    println!("  Hash: {}", hex::encode(&krate.hash[..8]));

    Ok(())
}

fn run_tests() -> Result<()> {
    println!("Running HLX smoke tests...\n");
    
    // Test 1: Basic arithmetic
    print!("Test 1 (5 + 3 = 8): ");
    let krate = HlxCrate::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(5) },
        hlx_core::Instruction::Constant { out: 1, val: Value::Integer(3) },
        hlx_core::Instruction::Add { out: 2, lhs: 0, rhs: 1 },
        hlx_core::Instruction::Return { val: 2 },
    ]);
    let result = execute(&krate)?;
    assert_eq!(result, Value::Integer(8));
    println!("✓ PASS");
    
    // Test 2: Determinism
    print!("Test 2 (Determinism): ");
    let c1 = HlxCrate::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(42) },
    ]);
    let c2 = HlxCrate::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(42) },
    ]);
    assert_eq!(c1.hash, c2.hash);
    println!("✓ PASS");
    
    // Test 3: LC-B roundtrip
    print!("Test 3 (LC-B roundtrip): ");
    let value = Value::from(std::collections::BTreeMap::from([("name".to_string(), Value::String("Alice".to_string())),
        ("age".to_string(), Value::Integer(30)),
    ]));
    let encoded = lcb::encode(&value)?;
    let decoded = lcb::decode(&encoded)?;
    assert_eq!(value, decoded);
    println!("✓ PASS");
    
    // Test 4: CAS roundtrip
    print!("Test 4 (CAS roundtrip): ");
    let krate = HlxCrate::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::String("test".to_string()) },
        hlx_core::Instruction::Collapse { handle_out: 1, val: 0 },
        hlx_core::Instruction::Resolve { val_out: 2, handle: 1 },
        hlx_core::Instruction::Return { val: 2 },
    ]);
    let result = execute(&krate)?;
    assert_eq!(result, Value::String("test".to_string()));
    println!("✓ PASS");
    
    println!("\nAll tests passed!");

    Ok(())
}

/// Check if profiling is available (perf_event_paranoid setting)
fn can_profile() -> bool {
    #[cfg(target_os = "linux")]
    {
        if let Ok(contents) = fs::read_to_string("/proc/sys/kernel/perf_event_paranoid") {
            if let Ok(value) = contents.trim().parse::<i32>() {
                return value <= 1;  // Need <=1 for user profiling
            }
        }
        false
    }
    #[cfg(not(target_os = "linux"))]
    {
        true  // Assume other platforms are OK
    }
}

/// Benchmark execution performance
fn bench(input: &PathBuf, hlx_s: bool, iterations: usize, json: bool, flamegraph: bool, frequency: u32) -> Result<()> {
    use std::time::{Duration, Instant};
    use serde_json::json;

    // Load the crate
    let krate = if input.extension().and_then(|s| s.to_str()) == Some("lcc") {
        // Load compiled crate
        let bytes = fs::read(input)
            .with_context(|| format!("Failed to read crate: {}", input.display()))?;
        bincode::deserialize(&bytes).context("Failed to deserialize crate")?
    } else {
        // Compile source file
        let source = fs::read_to_string(input)
            .with_context(|| format!("Failed to read source: {}", input.display()))?;
        let parser = HlxaParser::new();
        let ast = parser.parse(&source).context("Parse error")?;
        lower::lower_to_crate(&ast).context("Lowering failed")?
    };

    if !json {
        println!("Benchmarking: {}", input.display());
        println!("Iterations: {}", iterations);
        if hlx_s {
            println!("Mode: HLX-Scale speculation");
        } else {
            println!("Mode: Serial execution");
        }
        println!();
    }

    // Check if profiling is possible before attempting
    if flamegraph && !can_profile() {
        eprintln!("Warning: Flamegraph requested but profiling is not available.");
        eprintln!("On Linux, run: sudo sysctl -w kernel.perf_event_paranoid=-1");
        eprintln!("Current value is too restrictive. Continuing without flamegraph...");
        eprintln!();
    }

    // Warmup run
    let _ = execute(&krate)?;

    // Benchmark runs
    let mut durations = Vec::with_capacity(iterations);

    // Start profiling if flamegraph requested and available
    let guard = if flamegraph && can_profile() {
        match pprof::ProfilerGuardBuilder::default()
            .frequency(frequency as i32)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build() {
            Ok(g) => Some(g),
            Err(e) => {
                eprintln!("Warning: Could not start profiler: {}", e);
                eprintln!("Continuing without flamegraph...");
                None
            }
        }
    } else {
        None
    };

    for i in 0..iterations {
        let start = Instant::now();
        let result = execute(&krate)?;
        let elapsed = start.elapsed();
        durations.push(elapsed);

        if !json && iterations <= 20 {
            println!("  Run {}: {:?} -> {:?}", i + 1, elapsed, result);
        }
    }

    // Compute statistics
    let total: Duration = durations.iter().sum();
    let mean = total / iterations as u32;

    // Generate flamegraph if requested
    if let Some(guard) = guard {
        if let Err(e) = generate_flamegraph(&guard, input, hlx_s, iterations, mean, &krate) {
            eprintln!("Warning: Flamegraph generation failed: {}", e);
            eprintln!("Note: On Linux, you may need to run:");
            eprintln!("  sudo sysctl -w kernel.perf_event_paranoid=-1");
            eprintln!("  or use: cargo build --release && perf record ...");
        }
    }
    let min = durations.iter().min().unwrap();
    let max = durations.iter().max().unwrap();

    // Compute median
    let mut sorted = durations.clone();
    sorted.sort();
    let median = if sorted.len() % 2 == 0 {
        (sorted[sorted.len() / 2 - 1] + sorted[sorted.len() / 2]) / 2
    } else {
        sorted[sorted.len() / 2]
    };

    // Compute standard deviation
    let variance: f64 = durations.iter()
        .map(|d| {
            let diff = d.as_secs_f64() - mean.as_secs_f64();
            diff * diff
        })
        .sum::<f64>() / iterations as f64;
    let std_dev = variance.sqrt();

    if json {
        let output = json!({
            "file": input.display().to_string(),
            "mode": if hlx_s { "speculation" } else { "serial" },
            "iterations": iterations,
            "mean_ms": mean.as_secs_f64() * 1000.0,
            "median_ms": median.as_secs_f64() * 1000.0,
            "min_ms": min.as_secs_f64() * 1000.0,
            "max_ms": max.as_secs_f64() * 1000.0,
            "std_dev_ms": std_dev * 1000.0,
            "total_ms": total.as_secs_f64() * 1000.0,
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!();
        println!("Results:");
        println!("  Mean:   {:?}", mean);
        println!("  Median: {:?}", median);
        println!("  Min:    {:?}", min);
        println!("  Max:    {:?}", max);
        println!("  StdDev: {:.3}ms", std_dev * 1000.0);
        println!("  Total:  {:?}", total);

        if hlx_s {
            println!();
            println!("Tip: Compare with serial execution:");
            println!("  hlx bench {} -n {}", input.display(), iterations);
        }
    }

    Ok(())
}

/// Generate flamegraph from profiling data
fn generate_flamegraph(
    guard: &pprof::ProfilerGuard,
    input: &PathBuf,
    hlx_s: bool,
    iterations: usize,
    mean: std::time::Duration,
    krate: &HlxCrate,
) -> Result<()> {
    use std::io::Write;
    use chrono::Local;

    // Create perf_data directory if it doesn't exist
    fs::create_dir_all("perf_data")?;

    // Generate timestamp
    let timestamp = Local::now().format("%Y%m%d_%H%M%S");

    // Extract test name from input path
    let test_name = input.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown");

    // Generate mode label
    let mode = if hlx_s { "speculation" } else { "serial" };

    // Check if @scale pragma exists
    let has_scale = krate.metadata.as_ref()
        .and_then(|m| m.hlx_scale_substrates.get("main"))
        .map(|info| info.enable_speculation && info.agent_count > 1)
        .unwrap_or(false);

    let scale_info = if has_scale {
        krate.metadata.as_ref()
            .and_then(|m| m.hlx_scale_substrates.get("main"))
            .map(|info| format!("_scale{}_barriers{}", info.agent_count, info.barrier_count))
            .unwrap_or_default()
    } else {
        String::new()
    };

    // Build detailed filename
    let filename = format!(
        "perf_data/flamegraph-{}-{}{}_n{}_mean{:.0}ms_{}.svg",
        test_name,
        mode,
        scale_info,
        iterations,
        mean.as_secs_f64() * 1000.0,
        timestamp
    );

    // Generate report
    let report = guard.report().build()
        .context("Failed to build profiler report")?;

    // Create flamegraph
    let file = fs::File::create(&filename)
        .with_context(|| format!("Failed to create flamegraph file: {}", filename))?;
    let mut writer = std::io::BufWriter::new(file);

    // Generate title with metadata
    let title = format!(
        "HLX Bench: {} | {} | {} iterations | mean: {:.2}ms{}",
        test_name,
        mode,
        iterations,
        mean.as_secs_f64() * 1000.0,
        if has_scale {
            krate.metadata.as_ref()
                .and_then(|m| m.hlx_scale_substrates.get("main"))
                .map(|info| format!(" | @scale(size={}) | {} barriers", info.agent_count, info.barrier_count))
                .unwrap_or_default()
        } else {
            String::new()
        }
    );

    // Create custom options for detailed flamegraph
    let mut options = inferno::flamegraph::Options::default();
    options.title = title.clone();
    options.count_name = "samples".to_string();
    options.name_type = "Function:".to_string();
    options.hash = true;  // Consistent colors
    options.font_size = 12;  // Larger font
    options.min_width = 0.1;  // Show more detail

    // Convert report to flamegraph format
    let mut collapsed = Vec::new();
    report.flamegraph(&mut collapsed)
        .context("Failed to generate flamegraph data")?;

    // Generate SVG
    inferno::flamegraph::from_reader(
        &mut options,
        &collapsed[..],
        &mut writer,
    ).context("Failed to write flamegraph SVG")?;

    writer.flush()?;

    println!();
    println!("Flamegraph generated: {}", filename);
    println!("  Title: {}", title);

    Ok(())
}

// Helper: Convert JSON to HLX Value
fn json_to_value(json: &serde_json::Value) -> Result<Value> {
    Ok(match json {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                Value::float(f)?
            } else {
                anyhow::bail!("Invalid number")
            }
        }
        serde_json::Value::String(s) => Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            let values: Result<Vec<_>> = arr.iter().map(json_to_value).collect();
            Value::from(values?)
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v)?);
            }
            Value::from(map)
        }
    })
}

// Helper: Convert HLX Value to JSON
fn value_to_json(value: &Value) -> serde_json::Value {
    match value {
        Value::Null => serde_json::Value::Null,
        Value::Boolean(b) => serde_json::Value::Bool(*b),
        Value::Integer(i) => serde_json::json!(i),
        Value::Float(f) => serde_json::json!(f),
        Value::String(s) => serde_json::Value::String(s.clone()),
        Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(value_to_json).collect())
        }
        Value::Object(obj) => {
            let map: serde_json::Map<String, serde_json::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), value_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
        }
        Value::Contract(c) => {
            serde_json::json!({
                "@_id": c.id,
                "fields": c.fields.iter()
                    .map(|(idx, v)| (format!("@{}", idx), value_to_json(v)))
                    .collect::<serde_json::Map<String, serde_json::Value>>()
            })
        }
                Value::Handle(h) => serde_json::Value::String(h.clone()),
            }
        }
        
        fn replay(input: &PathBuf) -> Result<()> {
            let content = fs::read_to_string(input).context("Failed to read snapshot file")?;
            let snap: serde_json::Value = serde_json::from_str(&content).context("Invalid snapshot JSON")?;
        
            println!("=== HLX FLIGHT RECORDER: POST-MORTEM INVESTIGATION ===");
            println!("File: {}", input.display());
            if let Some(ts) = snap.get("timestamp").and_then(|v| v.as_i64()) {
                println!("Timestamp: {}", ts);
            }
            println!("Instruction Pointer (PC): {}", snap.get("pc").unwrap_or(&serde_json::json!(0)));
            println!("------------------------------------------------------");
        
            if let Some(stack) = snap.get("call_stack").and_then(|v| v.as_array()) {
                println!("Call Stack ({} frames):", stack.len());
                for (i, frame) in stack.iter().enumerate().rev() {
                    println!("\n[Frame {}]", i);
                    if let Some(ret) = frame.get("return_pc") {
                        if ret.is_null() {
                            println!("  Return PC: <ENTRY>");
                        } else {
                            println!("  Return PC: {}", ret);
                        }
                    }
                    if let Some(out) = frame.get("out_reg") {
                        println!("  Output Reg: r{}", out);
                    }
                    if let Some(regs) = frame.get("registers").and_then(|v| v.as_object()) {
                        println!("  Registers:");
                        let mut keys: Vec<_> = regs.keys().collect();
                        keys.sort_by_key(|k| {
                            if k.starts_with('r') {
                                k[1..].parse::<u32>().unwrap_or(0)
                            } else {
                                0
                            }
                        });
                        for k in keys {
                            println!("    {:4}: {}", k, regs.get(k).unwrap());
                        }
                    }
                }
            }
        
            println!("\n=== END OF INVESTIGATION ===");
            Ok(())
        }
        
fn compile_native(
    input: &PathBuf,
    output: Option<PathBuf>,
    target: Option<&str>,
    emit_asm: bool,
    emit_shared: bool,
    print_ir: bool,
) -> Result<()> {
    use hlx_backend_llvm::CodeGen;
    use inkwell::context::Context;

    // Read and parse source
    let source = fs::read_to_string(input)
        .context("Failed to read input file")?;

    let parser = HlxaParser::new();
    let ast = parser.parse(&source)
        .context("Parse error")?;

    // Lower to crate
    let krate = lower::lower_to_crate(&ast)
        .context("Lowering failed")?;

    // Create LLVM context and code generator
    let context = Context::create();
    let mut codegen = if let Some(target_triple) = target {
        println!("Compiling for target: {}", target_triple);
        CodeGen::with_target(&context, "hlx_program", Some(target_triple))?
    } else {
        println!("Compiling for host target");
        CodeGen::new(&context, "hlx_program")?
    };

    // Compile crate to LLVM IR
    codegen.compile_crate(&krate)
        .context("LLVM compilation failed")?;

    // Optionally print IR
    if print_ir {
        eprintln!("\n=== LLVM IR ===");
        codegen.print_ir();
        eprintln!("=== END IR ===\n");
    }

    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut p = input.clone();
        if emit_asm {
            p.set_extension("s");
        } else if emit_shared {
            // Use platform-specific extension
            let ext = match std::env::consts::OS {
                "linux" => "so",
                "macos" => "dylib",
                "windows" => "dll",
                _ => "so", // Default to .so
            };
            p.set_extension(ext);
        } else {
            p.set_extension("o");
        }
        p
    });

    // Emit native code
    if emit_asm {
        codegen.emit_assembly(&output_path)
            .context("Failed to emit assembly")?;
        println!("Assembly written to: {}", output_path.display());
    } else if emit_shared {
        codegen.emit_shared(&output_path)
            .context("Failed to emit shared library")?;
        println!("Shared library written to: {}", output_path.display());
    } else {
        codegen.emit_object(&output_path)
            .context("Failed to emit object file")?;
        println!("Object file written to: {}", output_path.display());
    }

    if target.is_some() {
        println!("\nNext steps:");
        println!("  1. Link with appropriate linker script for your target");
        println!("  2. For bare metal: ld -T linker.ld {} -o kernel.elf", output_path.display());
    }

    Ok(())
}

/// Emit runtime capabilities schema
fn emit_capabilities(output: Option<PathBuf>, format: &str) -> Result<()> {
    let mut caps = RuntimeCapabilities::new();

    // Add all known builtins
    // Core builtins (always available)
    caps.add_builtin(
        BuiltinSpecBuilder::new("print")
            .signature("print(message: String) -> ()")
            .description("Print a message to stdout")
            .parameter("message", "String", "Message to print", false)
            .returns("()")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("to_string")
            .signature("to_string(value: int|float) -> String")
            .description("Convert a value to string")
            .parameter("value", "int|float", "Value to convert", false)
            .returns("String")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("to_int")
            .signature("to_int(value: float|String) -> int")
            .description("Convert a value to integer")
            .parameter("value", "float|String", "Value to convert", false)
            .returns("int")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("to_float")
            .signature("to_float(value: int|String) -> float")
            .description("Convert a value to float")
            .parameter("value", "int|String", "Value to convert", false)
            .returns("float")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("arr_concat")
            .signature("arr_concat(a: Array, b: Array) -> Array")
            .description("Concatenate two arrays")
            .parameter("a", "Array", "First array", false)
            .parameter("b", "Array", "Second array", false)
            .returns("Array")
            .build(),
    );

    // GPU builtins (require vulkan or cuda)
    caps.add_builtin(
        BuiltinSpecBuilder::new("alloc_tensor")
            .signature("alloc_tensor(shape: Array, dtype: String) -> Handle")
            .description("Allocate a GPU tensor")
            .require_feature("vulkan")
            .parameter("shape", "Array", "Tensor shape dimensions", false)
            .parameter("dtype", "String", "Data type (i32, f32, etc.)", false)
            .returns("Handle")
            .stability(StabilityLevel::Stable)
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("gpu_dispatch")
            .signature("gpu_dispatch(shader: String, bindings: Array, push_constants: Array, x: int, y: int, z: int) -> ()")
            .description("Dispatch a compute shader")
            .require_feature("vulkan")
            .parameter("shader", "String", "Path to SPIR-V shader", false)
            .parameter("bindings", "Array<Handle>", "Buffer bindings", false)
            .parameter("push_constants", "Array", "Push constant data", false)
            .parameter("x", "int", "Workgroup count X", false)
            .parameter("y", "int", "Workgroup count Y", false)
            .parameter("z", "int", "Workgroup count Z", false)
            .returns("()")
            .stability(StabilityLevel::Stable)
            .build(),
    );

    // Pipe/IPC builtins
    caps.add_builtin(
        BuiltinSpecBuilder::new("pipe_open")
            .signature("pipe_open(command: String) -> int")
            .description("Open a pipe to a shell command")
            .parameter("command", "String", "Shell command to execute", false)
            .returns("int")
            .stability(StabilityLevel::Stable)
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("pipe_write")
            .signature("pipe_write(pid: int, data: Handle|String) -> ()")
            .description("Write data to a pipe")
            .parameter("pid", "int", "Pipe ID from pipe_open", false)
            .parameter("data", "Handle|String", "Data to write", false)
            .returns("()")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("pipe_read")
            .signature("pipe_read(pid: int) -> String")
            .description("Read data from a pipe")
            .parameter("pid", "int", "Pipe ID from pipe_open", false)
            .returns("String")
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("pipe_close")
            .signature("pipe_close(pid: int) -> ()")
            .description("Close a pipe")
            .parameter("pid", "int", "Pipe ID to close", false)
            .returns("()")
            .build(),
    );

    // Snapshot/debugging builtins
    caps.add_builtin(
        BuiltinSpecBuilder::new("snapshot")
            .signature("snapshot() -> Handle")
            .description("Capture current execution state")
            .returns("Handle")
            .stability(StabilityLevel::Experimental)
            .build(),
    );

    caps.add_builtin(
        BuiltinSpecBuilder::new("restore")
            .signature("restore(handle: Handle) -> ()")
            .description("Restore execution state from snapshot")
            .parameter("handle", "Handle", "Snapshot handle", false)
            .returns("()")
            .stability(StabilityLevel::Experimental)
            .build(),
    );

    // Add backend capabilities
    // Check if Vulkan is available (simplified check)
    #[cfg(feature = "vulkan")]
    {
        use hlx_runtime::backends::VulkanBackend;
        match VulkanBackend::new() {
            Ok(_backend) => {
                caps.add_backend(hlx_core::capabilities::BackendCapability {
                    name: "vulkan".to_string(),
                    available: true,
                    version: Some("1.3".to_string()),
                    operations: vec![
                        "alloc_tensor".to_string(),
                        "gpu_dispatch".to_string(),
                    ],
                    limits: std::collections::HashMap::new(),
                });
            }
            Err(_) => {
                caps.add_backend(hlx_core::capabilities::BackendCapability {
                    name: "vulkan".to_string(),
                    available: false,
                    version: None,
                    operations: vec![],
                    limits: std::collections::HashMap::new(),
                });
            }
        }
    }

    // CPU backend is always available
    caps.add_backend(hlx_core::capabilities::BackendCapability {
        name: "cpu".to_string(),
        available: true,
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        operations: vec!["interpret".to_string()],
        limits: std::collections::HashMap::new(),
    });

    // Serialize to requested format
    let output_str = match format {
        "json" => caps.to_json().context("Failed to serialize capabilities to JSON")?,
        "toml" => {
            // For now, just use JSON - TOML serialization can be added later
            caps.to_json().context("Failed to serialize capabilities")?
        }
        _ => anyhow::bail!("Unsupported format: {}", format),
    };

    // Write to output or stdout
    if let Some(path) = output {
        fs::write(&path, output_str)
            .with_context(|| format!("Failed to write to {}", path.display()))?;
        eprintln!("Capabilities written to: {}", path.display());
    } else {
        println!("{}", output_str);
    }

    Ok(())
}

fn generate_header(input: &PathBuf, output: Option<&PathBuf>) -> Result<()> {
    use hlx_core::ffi;

    // Read and deserialize crate
    let bytes = fs::read(input)
        .with_context(|| format!("Failed to read crate: {}", input.display()))?;
    let krate: HlxCrate = bincode::deserialize(&bytes)
        .context("Failed to deserialize crate")?;

    // Extract module name from input filename
    let module_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("hlx_module");

    // Generate header
    let header = ffi::generate_header(&krate, module_name)
        .context("No FFI exports found - use #[export] or #[no_mangle] attributes")?;

    // Write to output or stdout
    if let Some(path) = output {
        fs::write(path, &header)
            .with_context(|| format!("Failed to write header to {}", path.display()))?;
        eprintln!("Header generated: {}", path.display());
    } else {
        print!("{}", header);
    }

    Ok(())
}

fn generate_python(input: &PathBuf, output: Option<&PathBuf>, lib_name: Option<&str>) -> Result<()> {
    use hlx_core::ffi;

    // Read and deserialize crate
    let bytes = fs::read(input)
        .with_context(|| format!("Failed to read crate: {}", input.display()))?;
    let krate: HlxCrate = bincode::deserialize(&bytes)
        .context("Failed to deserialize crate")?;

    // Extract module name from input filename
    let module_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("hlx_module");

    // Determine library name (for loading .so/.dll/.dylib)
    let lib = lib_name.unwrap_or(module_name);

    // Generate Python wrapper
    let wrapper = ffi::generate_python_wrapper(&krate, module_name, lib)
        .context("No FFI exports found - use #[export] or #[no_mangle] attributes")?;

    // Write to output or stdout
    if let Some(path) = output {
        fs::write(path, &wrapper)
            .with_context(|| format!("Failed to write Python wrapper to {}", path.display()))?;
        eprintln!("Python wrapper generated: {}", path.display());
    } else {
        print!("{}", wrapper);
    }

    Ok(())
}

fn generate_rust(input: &PathBuf, output: Option<&PathBuf>, lib_name: Option<&str>, gen_cargo_toml: bool) -> Result<()> {
    use hlx_core::ffi;

    // Read and deserialize crate
    let bytes = fs::read(input)
        .with_context(|| format!("Failed to read crate: {}", input.display()))?;
    let krate: HlxCrate = bincode::deserialize(&bytes)
        .context("Failed to deserialize crate")?;

    // Extract module name from input filename
    let module_name = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("hlx_module");

    // Determine library name (for linking .so/.dll/.dylib)
    let lib = lib_name.unwrap_or(module_name);

    // Generate Rust wrapper
    let wrapper = ffi::generate_rust_wrapper(&krate, module_name, lib)
        .context("No FFI exports found - use #[export] or #[no_mangle] attributes")?;

    // Write to output or stdout
    if let Some(path) = output {
        fs::write(path, &wrapper)
            .with_context(|| format!("Failed to write Rust wrapper to {}", path.display()))?;
        eprintln!("Rust wrapper generated: {}", path.display());

        // Generate Cargo.toml if requested
        if gen_cargo_toml {
            let cargo_toml = ffi::generate_cargo_toml(module_name, lib);
            let toml_path = path.parent()
                .unwrap_or_else(|| std::path::Path::new("."))
                .join("Cargo.toml");
            fs::write(&toml_path, cargo_toml)
                .with_context(|| format!("Failed to write Cargo.toml to {}", toml_path.display()))?;
            eprintln!("Cargo.toml generated: {}", toml_path.display());
        }
    } else {
        print!("{}", wrapper);
    }

    Ok(())
}
