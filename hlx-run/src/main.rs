//! hlx-run — Execute HLX programs
//!
//! Usage:
//!   hlx-run <program.hlx> [args...]
//!   hlx-run <program.hlx> --func <function_name> [args...]
//!
//! Reads an HLX source file, compiles to bytecode, and executes.

use anyhow::{Context, Result};
use clap::Parser;
use std::fs;
use std::io::{self, Write};

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
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.repl {
        run_repl(args.verbose, args.max_steps)?;
        return Ok(());
    }

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

    // Compile using canonical pipeline: AstParser -> Lowerer
    print!("Compiling... ");
    std::io::stdout().flush()?;

    use hlx_runtime::{AstParser, Lowerer, Vm};

    // Phase 1: Parse source to AST
    let program = AstParser::parse(&source)
        .map_err(|e| anyhow::anyhow!("Parse error at line {}: {}", e.line, e.message))?;

    // Phase 2: Lower AST to bytecode (associated function, not method)
    let (bytecode, functions) =
        Lowerer::lower(&program).map_err(|e| anyhow::anyhow!("Lower error: {}", e.message))?;

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

    // Register functions with VM
    for (name, (start_pc, params)) in &functions {
        vm.register_function(name, *start_pc as usize, *params as usize);
    }

    let result = vm
        .run(&bytecode)
        .map_err(|e| anyhow::anyhow!("Runtime error: {}", e.message))?;

    println!("✓");
    println!();
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
                            for (name, (start_pc, params)) in &functions {
                                vm.register_function(name, *start_pc as usize, *params as usize);
                            }

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
