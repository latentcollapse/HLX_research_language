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

use hlx_core::{HlxCrate, Value, lcb};
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

    /// Compile to native code using LLVM backend
    CompileNative {
        /// Input source file
        input: PathBuf,

        /// Output file (.o object file or .s assembly)
        #[arg(short, long)]
        output: Option<PathBuf>,

        /// Target triple (e.g., "x86_64-unknown-none-elf" for bare metal)
        #[arg(long)]
        target: Option<String>,

        /// Emit assembly instead of object file
        #[arg(long)]
        asm: bool,

        /// Print LLVM IR to stderr
        #[arg(long)]
        print_ir: bool,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    
    // Setup logging
    if cli.verbose {
        tracing_subscriber::fmt()
            .with_max_level(tracing::Level::DEBUG)
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
        Commands::CompileNative { input, output, target, asm, print_ir } => {
            compile_native(&input, output, target.as_deref(), asm, print_ir)?;
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
        }
        
        println!("\nInstructions:");
        for (i, inst) in krate.instructions.iter().enumerate() {
            println!("  {:4}: {:?}", i, inst);
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
        CodeGen::with_target(&context, "hlx_program", Some(target_triple))
    } else {
        println!("Compiling for host target");
        CodeGen::new(&context, "hlx_program")
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
