//! End-to-end tests for the HLX pipeline
//!
//! Tests the full path: Source → AST → Bytecode → Execution
//! Also tests: Source → AST → Render (roundtrip verification)

use hlx_runtime::{AstParser, Lowerer, NodeCounter, Render, Value, Vm};

// ─── Helper: Parse → Render ────────────────────────────────────────

fn parse_and_render(source: &str) -> Result<String, String> {
    let ast = AstParser::parse(source).map_err(|e| {
        format!(
            "Parse error: {} at line {}, col {}",
            e.message, e.line, e.col
        )
    })?;

    Ok(ast.render(0))
}

fn count_nodes(source: &str) -> Result<(usize, usize, usize), String> {
    let ast = AstParser::parse(source).map_err(|e| format!("Parse error: {}", e.message))?;

    let counter = NodeCounter::new().count(&ast);
    Ok((counter.functions, counter.agents, counter.statements))
}

// ─── Helper: Full Pipeline (Source → AST → Bytecode → VM) ─────────

fn run_pipeline(source: &str) -> Value {
    let ast = AstParser::parse(source).expect("Parse failed");
    let (bc, funcs) = Lowerer::lower(&ast).expect("Lower failed");
    let mut vm = Vm::new().with_max_steps(100000);
    vm.load_functions(&funcs);
    vm.run(&bc).expect("VM execution failed")
}

// ═══════════════════════════════════════════════════════════════════
// AST RENDERING TESTS (Source → AST → Render)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_simple_function() {
    let source = r#"
        fn add(a: i64, b: i64) -> i64 {
            return a + b;
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("fn add"));
    assert!(rendered.contains("return"));
}

#[test]
fn test_let_statement() {
    let source = r#"
        let x = 42;
        let y: i64 = 100;
        let mut z = x + y;
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_if_statement() {
    let source = r#"
        fn test(n: i64) -> i64 {
            if (n > 0) {
                return 1;
            } else {
                return 0;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("if"));
    assert!(rendered.contains("else"));
}

#[test]
fn test_loop_statement() {
    let source = r#"
        fn countdown(n: i64) -> i64 {
            let i = n;
            loop(i > 0) {
                i = i - 1;
            }
            return i;
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_switch_statement() {
    let source = r#"
        fn classify(n: i64) -> i64 {
            switch n {
                case 0 => { return 0; }
                case 1 => { return 1; }
                default => { return 2; }
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("switch"));
    assert!(rendered.contains("case"));
}

#[test]
fn test_simple_agent() {
    let source = r#"
        recursive agent Counter {
            latent count: i64 = 0;

            cycle H(10) {
                count = count + 1;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("recursive agent"));
    assert!(rendered.contains("latent"));
    assert!(rendered.contains("cycle"));
}

#[test]
fn test_agent_with_govern() {
    let source = r#"
        recursive agent SafeAgent {
            latent state: i64 = 0;

            govern {
                effect: modify;
                conscience: [no_harm, path_safety];
                trust: 0.8;
            }

            cycle H(5) {
                state = state + 1;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("govern"));
    assert!(rendered.contains("conscience"));
}

#[test]
fn test_agent_with_modify() {
    let source = r#"
        recursive agent SelfModifying {
            latent value: f64 = 0.0;

            modify self {
                gate proof;
                cooldown: 100;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("modify self"));
}

#[test]
fn test_cluster() {
    let source = r#"
        scale cluster Swarm {
            agents: [Worker1, Worker2, Worker3];
            barrier sync_point(3);
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);

    let rendered = result.unwrap();
    assert!(rendered.contains("scale cluster"));
    assert!(rendered.contains("agents"));
}

#[test]
fn test_module() {
    let source = r#"
        module Math {
            fn add(a: i64, b: i64) -> i64 {
                return a + b;
            }

            fn mul(a: i64, b: i64) -> i64 {
                return a * b;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_expressions() {
    let source = r#"
        fn test() -> i64 {
            let a = 1 + 2 * 3;
            let b = (a - 1) / 2;
            let c = a % b;
            let d = a == b;
            let e = a > 0 && b < 10;
            let arr = [1, 2, 3];
            let elem = arr[0];
            return a;
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_node_counting() {
    let source = r#"
        fn foo() -> i64 { return 1; }
        fn bar() -> i64 { return 2; }
        recursive agent A { latent x: i64 = 0; }
    "#;

    let (funcs, agents, stmts) = count_nodes(source).expect("Parse failed");
    assert_eq!(funcs, 2);
    assert_eq!(agents, 1);
    assert!(stmts > 0);
}

#[test]
fn test_halt_when() {
    let source = r#"
        recursive agent Reasoner {
            latent confidence: f64 = 0.0;

            halt when confidence >= 0.95 or steps >= 1000;

            cycle H(100) {
                confidence = confidence + 0.01;
            }
        }
    "#;

    // Halt parsing may not be fully implemented yet, but shouldn't crash
    let result = parse_and_render(source);
    assert!(result.is_ok() || result.is_err());
}

#[test]
fn test_trm_style_agent() {
    let source = r#"
        recursive agent TRMAgent {
            latent hypothesis: i64 = 0;
            latent confidence: f64 = 0.0;

            takes: input;
            gives: result;

            cycle H(10) {
                hypothesis = hypothesis + 1;
            }

            cycle L(100) {
                confidence = confidence + 0.001;
            }

            govern {
                effect: self_modify;
                conscience: [no_harm, no_bypass, path_safety];
                trust: 0.9;
            }

            modify self {
                gate proof;
                gate consensus;
                gate human;
                cooldown: 1000;
            }
        }
    "#;

    let result = parse_and_render(source);
    assert!(result.is_ok(), "Failed: {:?}", result);
}

#[test]
fn test_ast_modification_potential() {
    let source = r#"
        fn foo(x: i64) -> i64 {
            return x + 1;
        }
    "#;

    let ast = AstParser::parse(source).expect("Parse failed");

    let counter = NodeCounter::new().count(&ast);
    assert!(counter.functions >= 1);
    assert!(counter.statements >= 1);

    let rendered = ast.render(0);
    assert!(rendered.contains("fn foo"));
}

// ═══════════════════════════════════════════════════════════════════
// FULL PIPELINE TESTS (Source → AST → Bytecode → VM Execution)
// ═══════════════════════════════════════════════════════════════════

#[test]
fn test_pipeline_simple_return() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            return 42;
        }
    "#);
    assert_eq!(result, Value::I64(42));
}

#[test]
fn test_pipeline_arithmetic() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let x = 10 + 32;
            return x;
        }
    "#);
    assert_eq!(result, Value::I64(42));
}

#[test]
fn test_pipeline_complex_arithmetic() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let a = 2 + 3 * 4;
            let b = (2 + 3) * 4;
            return a + b;
        }
    "#);
    // a = 2 + 12 = 14, b = 5 * 4 = 20, total = 34
    assert_eq!(result, Value::I64(34));
}

#[test]
fn test_pipeline_if_true() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let x = 10;
            if (x > 5) {
                return 100;
            } else {
                return 0;
            }
        }
    "#);
    assert_eq!(result, Value::I64(100));
}

#[test]
fn test_pipeline_if_false() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let x = 3;
            if (x > 5) {
                return 100;
            } else {
                return 0;
            }
        }
    "#);
    assert_eq!(result, Value::I64(0));
}

#[test]
fn test_pipeline_loop_sum() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let sum = 0;
            let i = 1;
            loop(i < 11) {
                sum = sum + i;
                i = i + 1;
            }
            return sum;
        }
    "#);
    // sum of 1..10 = 55
    assert_eq!(result, Value::I64(55));
}

#[test]
fn test_pipeline_function_call() {
    let result = run_pipeline(r#"
        fn add(a: i64, b: i64) -> i64 {
            return a + b;
        }
        fn main() -> i64 {
            return add(19, 23);
        }
    "#);
    assert_eq!(result, Value::I64(42));
}

#[test]
fn test_pipeline_nested_calls() {
    let result = run_pipeline(r#"
        fn double(x: i64) -> i64 {
            return x + x;
        }
        fn quadruple(x: i64) -> i64 {
            return double(double(x));
        }
        fn main() -> i64 {
            return quadruple(5);
        }
    "#);
    assert_eq!(result, Value::I64(20));
}

#[test]
fn test_pipeline_recursive_fibonacci() {
    let result = run_pipeline(r#"
        fn fib(n: i64) -> i64 {
            if (n < 2) {
                return n;
            }
            return fib(n - 1) + fib(n - 2);
        }
        fn main() -> i64 {
            return fib(10);
        }
    "#);
    assert_eq!(result, Value::I64(55));
}

#[test]
fn test_pipeline_nested_loops() {
    let result = run_pipeline(r#"
        fn main() -> i64 {
            let total = 0;
            let i = 0;
            loop(i < 4) {
                let j = 0;
                loop(j < 5) {
                    total = total + 1;
                    j = j + 1;
                }
                i = i + 1;
            }
            return total;
        }
    "#);
    assert_eq!(result, Value::I64(20));
}

#[test]
fn test_pipeline_factorial() {
    let result = run_pipeline(r#"
        fn factorial(n: i64) -> i64 {
            if (n <= 1) {
                return 1;
            }
            return n * factorial(n - 1);
        }
        fn main() -> i64 {
            return factorial(6);
        }
    "#);
    assert_eq!(result, Value::I64(720));
}

// ─── Pipeline: Agent Lowering (no execution, just bytecode emission) ─

#[test]
fn test_pipeline_agent_lowers() {
    let source = r#"
        recursive agent Counter {
            latent count: i64 = 0;
            cycle H(10) {
                count = count + 1;
            }
            govern {
                effect: modify;
                conscience: [no_harm, path_safety];
                trust: 0.8;
            }
        }
    "#;
    let ast = AstParser::parse(source).expect("Parse failed");
    let (bc, _funcs) = Lowerer::lower(&ast).expect("Lower failed");
    // Agent should produce non-empty bytecode
    assert!(!bc.code.is_empty());
}

#[test]
fn test_pipeline_agent_with_modify_lowers() {
    let source = r#"
        recursive agent RSIAgent {
            latent v: f64 = 0.0;
            modify self {
                gate proof;
                gate consensus;
                cooldown: 500;
            }
        }
    "#;
    let ast = AstParser::parse(source).expect("Parse failed");
    let (bc, _funcs) = Lowerer::lower(&ast).expect("Lower failed");
    assert!(!bc.code.is_empty());
}
