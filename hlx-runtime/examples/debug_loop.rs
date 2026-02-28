use hlx_runtime::{Compiler, Vm};

fn main() {
    let source = r#"
        program test {
            fn main() -> i64 {
                let h = 0
                let l = 0
                let result = 0
                
                loop h < 3 {
                    l = 0
                    loop l < 6 {
                        result = result + 1
                        l = l + 1
                    }
                    h = h + 1
                }
                
                return result
            }
        }
    "#;

    println!("Compiling...");
    let (bc, funcs) = Compiler::compile(source).unwrap();

    println!("Functions: {:?}", funcs);
    println!("Constants: {:?}", bc.constants);
    println!("Strings: {:?}", bc.strings);
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
                println!("CONST r{} = const[{}]", dst, idx);
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

    println!("\nRunning...");
    let mut vm = Vm::new().with_max_steps(100000);
    let result = vm.run(&bc).unwrap();

    println!("\nResult: {:?}", result);
}
