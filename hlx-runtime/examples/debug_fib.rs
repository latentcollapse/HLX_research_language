use hlx_runtime::{Compiler, Vm};

fn main() {
    let source = r#"
        program test {
            fn fib(n: i64) -> i64 {
                if n < 2 {
                    return n
                }
                return fib(n - 1) + fib(n - 2)
            }
            
            fn main() -> i64 {
                return fib(6)
            }
        }
    "#;

    println!("Compiling...");
    let (bc, funcs) = Compiler::compile(source).unwrap();

    println!("Functions: {:?}", funcs);
    println!("Constants: {:?}", bc.constants);

    println!("\nBytecode ({} bytes):", bc.code.len());

    let mut i = 0;
    while i < bc.code.len() {
        let op = bc.code[i];
        print!("{:04}: {:3} ", i, op);
        match op {
            1 => {
                let dst = bc.code[i + 1];
                let idx = u32::from_le_bytes([
                    bc.code[i + 2],
                    bc.code[i + 3],
                    bc.code[i + 4],
                    bc.code[i + 5],
                ]);
                let val = &bc.constants[idx as usize];
                println!("CONST r{} = {:?}", dst, val);
                i += 6;
            }
            2 => {
                let dst = bc.code[i + 1];
                let src = bc.code[i + 2];
                println!("MOVE r{} = r{}", dst, src);
                i += 3;
            }
            10 => {
                let dst = bc.code[i + 1];
                let a = bc.code[i + 2];
                let b = bc.code[i + 3];
                println!("ADD r{} = r{} + r{}", dst, a, b);
                i += 4;
            }
            11 => {
                let dst = bc.code[i + 1];
                let a = bc.code[i + 2];
                let b = bc.code[i + 3];
                println!("SUB r{} = r{} - r{}", dst, a, b);
                i += 4;
            }
            22 => {
                let dst = bc.code[i + 1];
                let a = bc.code[i + 2];
                let b = bc.code[i + 3];
                println!("LT r{} = r{} < r{}", dst, a, b);
                i += 4;
            }
            40 => {
                let target = u32::from_le_bytes([
                    bc.code[i + 1],
                    bc.code[i + 2],
                    bc.code[i + 3],
                    bc.code[i + 4],
                ]);
                println!("JUMP {}", target);
                i += 5;
            }
            41 => {
                let cond = bc.code[i + 1];
                let target = u32::from_le_bytes([
                    bc.code[i + 2],
                    bc.code[i + 3],
                    bc.code[i + 4],
                    bc.code[i + 5],
                ]);
                println!("JUMP_IF r{} -> {}", cond, target);
                i += 6;
            }
            42 => {
                let cond = bc.code[i + 1];
                let target = u32::from_le_bytes([
                    bc.code[i + 2],
                    bc.code[i + 3],
                    bc.code[i + 4],
                    bc.code[i + 5],
                ]);
                println!("JUMP_IF_NOT r{} -> {}", cond, target);
                i += 6;
            }
            50 => {
                let name_idx = u32::from_le_bytes([
                    bc.code[i + 1],
                    bc.code[i + 2],
                    bc.code[i + 3],
                    bc.code[i + 4],
                ]);
                let argc = bc.code[i + 5];
                let dst = bc.code[i + 6];
                let name = bc
                    .strings
                    .get(name_idx as usize)
                    .map(|s| s.as_str())
                    .unwrap_or("?");
                println!("CALL \"{}\"({} args) -> r{}", name, argc, dst);
                i += 7;
            }
            51 => {
                println!("RETURN");
                i += 1;
            }
            52 => {
                println!("HALT");
                i += 1;
            }
            _ => {
                println!("??? (opcode {})", op);
                i += 1;
            }
        }
    }

    println!("\nRunning fib(6)...");
    let mut vm = Vm::new().with_max_steps(100000);
    vm.load_functions(&funcs);
    let result = vm.run(&bc).unwrap();

    println!("\nResult: {:?} (expected: 8)", result);
}
