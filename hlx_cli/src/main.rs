//! HLX Command-Line Interface
//!
//! ```bash
//! # Compile HLXL to capsule
//! hlx compile program.hlxl -o program.lcb
//!
//! # Run a capsule
//! hlx run program.lcb
//!
//! # Transliterate between forms
//! hlx translate --from hlxl --to hlx program.hlxl
//!
//! # Inspect a capsule
//! hlx inspect program.lcb
//! ```

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::fs;

use hlx_core::{Capsule, Value, lcb};
use hlx_compiler::{HlxlParser, HlxlEmitter, RunicEmitter, parser::Parser as ParseTrait, Emitter, lower};
use hlx_runtime::{execute, RuntimeConfig};

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
    /// Compile HLXL/HLX source to LC-B capsule
    Compile {
        /// Input source file
        input: PathBuf,
        
        /// Output capsule file
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Input format (hlxl or hlx)
        #[arg(long, default_value = "hlxl")]
        format: String,
    },
    
    /// Run a capsule or source file
    Run {
        /// Input file (capsule or source)
        input: PathBuf,
        
        /// Force CPU backend
        #[arg(long)]
        cpu: bool,
    },
    
    /// Transliterate between HLXL and HLX
    Translate {
        /// Input source file
        input: PathBuf,
        
        /// Output file (stdout if not specified)
        #[arg(short, long)]
        output: Option<PathBuf>,
        
        /// Source format
        #[arg(long, default_value = "hlxl")]
        from: String,
        
        /// Target format
        #[arg(long, default_value = "hlx")]
        to: String,
    },
    
    /// Inspect a capsule
    Inspect {
        /// Capsule file to inspect
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
    
    /// Run smoke tests
    Test,
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
        Commands::Run { input, cpu } => {
            run(&input, cpu)?;
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
        Commands::Test => {
            run_tests()?;
        }
    }
    
    Ok(())
}

fn compile(input: &PathBuf, output: Option<PathBuf>, format: &str) -> Result<()> {
    let source = fs::read_to_string(input)
        .context("Failed to read input file")?;
    
    // Parse source
    let ast = match format {
        "hlxl" => {
            let parser = HlxlParser::new();
            parser.parse(&source).context("Parse error")?
        }
        "hlx" => {
            let hlxl = hlx_compiler::runic::transliterate_to_hlxl(&source)?;
            let parser = HlxlParser::new();
            parser.parse(&hlxl).context("Parse error")?
        }
        _ => anyhow::bail!("Unknown format: {}", format),
    };
    
    // Lower to capsule
    let capsule = lower::lower_to_capsule(&ast)
        .context("Lowering failed")?;
    
    // Determine output path
    let output_path = output.unwrap_or_else(|| {
        let mut p = input.clone();
        p.set_extension("lcb");
        p
    });
    
    // Serialize capsule
    let bytes = bincode::serialize(&capsule)
        .context("Serialization failed")?;
    
    fs::write(&output_path, &bytes)
        .context("Failed to write output")?;
    
    println!("Compiled {} -> {}", input.display(), output_path.display());
    println!("  Instructions: {}", capsule.len());
    println!("  Hash: {}", hex::encode(&capsule.hash[..8]));
    
    Ok(())
}

fn run(input: &PathBuf, cpu_only: bool) -> Result<()> {
    let config = if cpu_only {
        RuntimeConfig::cpu_only()
    } else {
        RuntimeConfig::default()
    };
    
    // Check if input is a capsule or source
    let capsule = if input.extension().map(|e| e == "lcb").unwrap_or(false) {
        // Load capsule
        let bytes = fs::read(input).context("Failed to read capsule")?;
        bincode::deserialize(&bytes).context("Failed to deserialize capsule")?
    } else {
        // Compile source first
        let source = fs::read_to_string(input).context("Failed to read source")?;
        let parser = HlxlParser::new();
        let ast = parser.parse(&source).context("Parse error")?;
        lower::lower_to_capsule(&ast).context("Lowering failed")?
    };
    
    // Execute
    let result = hlx_runtime::execute_with_config(&capsule, &config)
        .context("Execution failed")?;
    
    println!("{}", result);
    
    Ok(())
}

fn translate(input: &PathBuf, output: Option<PathBuf>, from: &str, to: &str) -> Result<()> {
    let source = fs::read_to_string(input)
        .context("Failed to read input")?;
    
    // Parse
    let ast = match from {
        "hlxl" => {
            let parser = HlxlParser::new();
            parser.parse(&source).context("Parse error")?
        }
        "hlx" => {
            let hlxl = hlx_compiler::runic::transliterate_to_hlxl(&source)?;
            let parser = HlxlParser::new();
            parser.parse(&hlxl).context("Parse error")?
        }
        _ => anyhow::bail!("Unknown source format: {}", from),
    };
    
    // Emit
    let result = match to {
        "hlxl" => {
            let emitter = HlxlEmitter::new();
            emitter.emit(&ast)?
        }
        "hlx" => {
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
    let bytes = fs::read(input).context("Failed to read capsule")?;
    let capsule: Capsule = bincode::deserialize(&bytes)
        .context("Failed to deserialize capsule")?;
    
    if json {
        let info = serde_json::json!({
            "version": capsule.version,
            "hash": hex::encode(&capsule.hash),
            "instruction_count": capsule.len(),
            "is_valid": capsule.is_valid(),
            "metadata": capsule.metadata,
        });
        println!("{}", serde_json::to_string_pretty(&info)?);
    } else {
        println!("Capsule: {}", input.display());
        println!("  Version: {}", capsule.version);
        println!("  Hash: {}", hex::encode(&capsule.hash));
        println!("  Instructions: {}", capsule.len());
        println!("  Valid: {}", capsule.is_valid());
        
        if let Some(meta) = &capsule.metadata {
            if let Some(src) = &meta.source_file {
                println!("  Source: {}", src);
            }
            if let Some(ver) = &meta.compiler_version {
                println!("  Compiler: {}", ver);
            }
        }
        
        println!("\nInstructions:");
        for (i, inst) in capsule.instructions.iter().enumerate() {
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

fn run_tests() -> Result<()> {
    println!("Running HLX smoke tests...\n");
    
    // Test 1: Basic arithmetic
    print!("Test 1 (5 + 3 = 8): ");
    let capsule = Capsule::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(5) },
        hlx_core::Instruction::Constant { out: 1, val: Value::Integer(3) },
        hlx_core::Instruction::Add { out: 2, lhs: 0, rhs: 1 },
        hlx_core::Instruction::Return { val: 2 },
    ]);
    let result = execute(&capsule)?;
    assert_eq!(result, Value::Integer(8));
    println!("✓ PASS");
    
    // Test 2: Determinism
    print!("Test 2 (Determinism): ");
    let cap1 = Capsule::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(42) },
    ]);
    let cap2 = Capsule::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::Integer(42) },
    ]);
    assert_eq!(cap1.hash, cap2.hash);
    println!("✓ PASS");
    
    // Test 3: LC-B roundtrip
    print!("Test 3 (LC-B roundtrip): ");
    let value = Value::Object(vec![
        ("name".to_string(), Value::String("Alice".to_string())),
        ("age".to_string(), Value::Integer(30)),
    ].into_iter().collect());
    let encoded = lcb::encode(&value)?;
    let decoded = lcb::decode(&encoded)?;
    assert_eq!(value, decoded);
    println!("✓ PASS");
    
    // Test 4: CAS roundtrip
    print!("Test 4 (CAS roundtrip): ");
    let capsule = Capsule::new(vec![
        hlx_core::Instruction::Constant { out: 0, val: Value::String("test".to_string()) },
        hlx_core::Instruction::Collapse { handle_out: 1, val: 0 },
        hlx_core::Instruction::Resolve { val_out: 2, handle: 1 },
        hlx_core::Instruction::Return { val: 2 },
    ]);
    let result = execute(&capsule)?;
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
            Value::Array(values?)
        }
        serde_json::Value::Object(obj) => {
            let mut map = std::collections::BTreeMap::new();
            for (k, v) in obj {
                map.insert(k.clone(), json_to_value(v)?);
            }
            Value::Object(map)
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
