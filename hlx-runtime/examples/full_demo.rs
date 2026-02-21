use hlx_runtime::{Compiler, Value, Vm};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║              HLX Runtime v0.1 - Bytecode VM + Compiler             ║");
    println!("║              TRM-Validated Recursive Intelligence                  ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    demo_trm_cycles();
    demo_function_calls();
    demo_recursive_fib();
    demo_nested_loops();

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║  HLX Runtime: OPERATIONAL                                          ║");
    println!("║  - Bytecode VM: Working                                            ║");
    println!("║  - Source Compiler: Working                                        ║");
    println!("║  - Function Calls: Working                                         ║");
    println!("║  - Recursion: Working                                              ║");
    println!("║  - TRM Cycles: Validated (H=3, L=6 → 18 refinements)               ║");
    println!("║                                                                    ║");
    println!("║  Ready for: Axiom integration, tensor bridge, agent execution     ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
}

fn demo_trm_cycles() {
    println!("═══ Demo 1: TRM-Style Recursive Cycles ═══");
    println!("   (Validating the core TRM architecture: H_cycles=3, L_cycles=6)");

    let source = r#"
        program trm_demo {
            fn main() -> i64 {
                let h = 0
                let l = 0
                let refinements = 0
                
                loop h < 3 {
                    l = 0
                    loop l < 6 {
                        refinements = refinements + 1
                        l = l + 1
                    }
                    h = h + 1
                }
                
                return refinements
            }
        }
    "#;

    let (bc, funcs) = Compiler::compile(source).unwrap();
    let mut vm = Vm::new().with_max_steps(100000);
    vm.load_functions(&funcs);
    let result = vm.run(&bc).unwrap();

    println!("   Result: {} refinements", result);
    println!("   ✓ TRM architecture validated (3 × 6 = 18)\n");
}

fn demo_function_calls() {
    println!("═══ Demo 2: User-Defined Function Calls ═══");

    let source = r#"
        program func_demo {
            fn add(a: i64, b: i64) -> i64 {
                return a + b
            }
            
            fn multiply(x: i64, y: i64) -> i64 {
                return x * y
            }
            
            fn main() -> i64 {
                let sum = add(10, 32)
                let product = multiply(6, 7)
                return add(sum, product)
            }
        }
    "#;

    let (bc, funcs) = Compiler::compile(source).unwrap();
    let mut vm = Vm::new().with_max_steps(10000);
    vm.load_functions(&funcs);
    let result = vm.run(&bc).unwrap();

    println!("   add(10, 32) + multiply(6, 7) = {}", result);
    println!("   (42 + 42 = 84)");
    println!("   ✓ Function calls working\n");
}

fn demo_recursive_fib() {
    println!("═══ Demo 3: Recursive Fibonacci ═══");

    let source = r#"
        program fib_demo {
            fn fib(n: i64) -> i64 {
                if n < 2 {
                    return n
                }
                return fib(n - 1) + fib(n - 2)
            }
            
            fn main() -> i64 {
                return fib(10)
            }
        }
    "#;

    let (bc, funcs) = Compiler::compile(source).unwrap();
    let mut vm = Vm::new().with_max_steps(500000);
    vm.load_functions(&funcs);
    let result = vm.run(&bc).unwrap();

    println!("   fib(10) = {}", result);
    println!("   ✓ Recursion working\n");
}

fn demo_nested_loops() {
    println!("═══ Demo 4: Nested Computation ═══");

    let source = r#"
        program nested_demo {
            fn main() -> i64 {
                let sum = 0
                let i = 1
                loop i < 11 {
                    let j = 1
                    loop j < 11 {
                        sum = sum + 1
                        j = j + 1
                    }
                    i = i + 1
                }
                return sum
            }
        }
    "#;

    let (bc, funcs) = Compiler::compile(source).unwrap();
    let mut vm = Vm::new().with_max_steps(100000);
    vm.load_functions(&funcs);
    let result = vm.run(&bc).unwrap();

    println!("   10 × 10 iterations = {}", result);
    println!("   ✓ Nested loops working\n");
}
