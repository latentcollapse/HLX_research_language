mod error;
mod lexer;
mod parser;
mod checker;
mod interpreter;
mod lcb;
mod trust;
mod conscience;
mod experimental;

use std::env;
use std::fs;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        eprintln!("Axiom Language v2.4.0 — Deterministic Effect-Typed Agent Language");
        eprintln!();
        eprintln!("Usage: axiom <file.axm> [options]");
        eprintln!("       axiom --project <axiom.project> [options]");
        eprintln!();
        eprintln!("Options:");
        eprintln!("  --parse-only    Only lex and parse, show AST summary");
        eprintln!("  --check-only    Lex, parse, and type-check only");
        eprintln!("  --dsf-only      Run DSF analysis only");
        eprintln!("  --project FILE  Use project manifest for module resolution");
        eprintln!("  --verbose       Show detailed output");
        eprintln!();
        eprintln!("Architecture:");
        eprintln!("  Lexer → Parser → TypeChecker → DSF → Interpreter");
        eprintln!("  LC-B wire format · Trust algebra · Conscience kernel");
        eprintln!("  SCALE coordination · Inference modes · Self-modification");
        process::exit(1);
    }

    let filename = &args[1];
    let parse_only = args.contains(&"--parse-only".to_string());
    let check_only = args.contains(&"--check-only".to_string());
    let dsf_only = args.contains(&"--dsf-only".to_string());
    let verbose = args.contains(&"--verbose".to_string());

    // Check for project manifest mode
    let project_file = args.iter().position(|a| a == "--project")
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str());

    // Load project manifest if specified
    if let Some(manifest_path) = project_file {
        if verbose {
            eprintln!("=== Loading Project Manifest: {} ===", manifest_path);
        }
        let manifest_content = match fs::read_to_string(manifest_path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Error reading manifest '{}': {}", manifest_path, e);
                process::exit(1);
            }
        };
        match experimental::module::parse_manifest(&manifest_content) {
            Ok(manifest) => {
                if verbose {
                    eprintln!("  Project: {} v{}", manifest.name, manifest.version);
                    eprintln!("  Axiom version: {}", manifest.axiom_version);
                    eprintln!("  Inference mode: {}", manifest.inference_mode);
                    eprintln!("  SCALE: max_agents={}, mode={}", manifest.scale_max_agents, manifest.scale_mode);
                    eprintln!("  Modules: {}", manifest.modules.len());
                    for (alias, path) in &manifest.modules {
                        eprintln!("    {} → {}", alias, path);
                    }
                }
            }
            Err(e) => {
                eprintln!("Manifest error: {}", e);
                process::exit(1);
            }
        }
    }

    // Read source file
    let source = match fs::read_to_string(filename) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error reading '{}': {}", filename, e);
            process::exit(1);
        }
    };

    // Phase 1: Lex
    if verbose {
        eprintln!("=== Lexing ===");
    }
    let mut lexer = lexer::Lexer::new(&source);
    let tokens = match lexer.tokenize() {
        Ok(tokens) => {
            if verbose {
                eprintln!("  {} tokens produced", tokens.len());
            }
            tokens
        }
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            process::exit(1);
        }
    };

    // Phase 2: Parse
    if verbose {
        eprintln!("=== Parsing ===");
    }
    let mut parser = parser::Parser::new(tokens);
    let program = match parser.parse_program() {
        Ok(program) => {
            if verbose {
                eprintln!("  Module '{}' parsed successfully", program.module.name);
                eprintln!("  {} items", program.module.items.len());
            }
            program
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    if parse_only {
        println!("Parse successful: module '{}'", program.module.name);
        print_module_summary(&program);
        process::exit(0);
    }

    // Phase 3: Type Check
    if verbose {
        eprintln!("=== Type Checking ===");
    }
    let mut checker = checker::TypeChecker::new();
    match checker.check_program(&program) {
        Ok(()) => {
            if verbose {
                eprintln!("  Type checking passed");
            }
        }
        Err(e) => {
            eprintln!("Type error: {}", e);
            process::exit(1);
        }
    }
    for warning in &checker.warnings {
        eprintln!("Warning: {}", warning);
    }

    if check_only {
        println!("Type check successful: module '{}'", program.module.name);
        process::exit(0);
    }

    // Phase 4: DSF Analysis (Dumb Shit Filter — Parts XII, XIII)
    if verbose {
        eprintln!("=== DSF Analysis ===");
    }
    let mut dsf = experimental::dsf::DsfAnalyzer::new();
    let dsf_result = dsf.analyze(&program);

    if verbose || dsf_only {
        eprint!("{}", dsf.summary());
    }

    if let Err(e) = dsf_result {
        eprintln!("DSF error: {}", e);
        process::exit(1);
    }

    if dsf_only {
        process::exit(0);
    }

    // Phase 5: Interpret (emulated runtime)
    if verbose {
        eprintln!("=== Executing (Emulated Runtime) ===");
        eprintln!("  Conscience kernel: active (4 genesis predicates)");
        eprintln!("  Trust tracking: active (4-level algebra)");
        eprintln!("  BLAKE3 content-addressing: active");
        eprintln!("  Checkpoint/rollback: active");
    }
    let mut interp = interpreter::Interpreter::new();
    match interp.run(&program) {
        Ok(result) => {
            // Print output
            for line in &interp.output {
                println!("{}", line);
            }
            if verbose {
                eprintln!("=== Execution Summary ===");
                eprintln!("  Epochs executed: {}", interp.epoch);
                eprintln!("  Intents logged: {}", interp.intent_log.len());
                for log in &interp.intent_log {
                    eprintln!("    [epoch {}] {} — {}", log.epoch, log.intent_name, log.verdict);
                }
            }
            match result {
                interpreter::value::Value::I64(code) => {
                    if verbose {
                        eprintln!("=== Exit code: {} ===", code);
                    }
                    process::exit(code as i32);
                }
                interpreter::value::Value::Void => {
                    if verbose {
                        eprintln!("=== Completed (void) ===");
                    }
                }
                other => {
                    if verbose {
                        eprintln!("=== Result: {} ===", other);
                    }
                }
            }
        }
        Err(e) => {
            // Print any output before the error
            for line in &interp.output {
                println!("{}", line);
            }
            eprintln!("Runtime error: {}", e);
            process::exit(1);
        }
    }
}

fn print_module_summary(program: &parser::ast::Program) {
    for item in &program.module.items {
        match item {
            parser::ast::Item::Import(i) => {
                println!("  import \"{}\"", i.path);
            }
            parser::ast::Item::Function(f) => {
                let exported = if f.exported { "export " } else { "" };
                let ret = match &f.return_type {
                    Some(t) => format!(" -> {:?}", t),
                    None => String::new(),
                };
                println!(
                    "  {}fn {}({} params){}",
                    exported,
                    f.name,
                    f.params.len(),
                    ret
                );
            }
            parser::ast::Item::Contract(c) => {
                if let Some(parts) = &c.composed_of {
                    println!("  contract {} = {}", c.name, parts.join(" + "));
                } else {
                    println!("  contract {} ({} fields)", c.name, c.fields.len());
                }
            }
            parser::ast::Item::Intent(i) => {
                if let Some(parts) = &i.composed_of {
                    println!("  intent {} = {}", i.name, parts.join(" >> "));
                } else {
                    println!(
                        "  intent {} (takes: {}, gives: {})",
                        i.name,
                        i.clauses.takes.len(),
                        i.clauses.gives.len()
                    );
                }
            }
            parser::ast::Item::Enum(e) => {
                println!("  enum {} ({} variants)", e.name, e.variants.len());
            }
            parser::ast::Item::TensorOp(t) => {
                println!("  tensor_op {}", t.name);
            }
        }
    }
}
