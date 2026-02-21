use hlx_runtime::{Bytecode, Opcode, Value, Vm};

fn main() {
    println!("╔════════════════════════════════════════════════════════════╗");
    println!("║            HLX Runtime v0.1 - Bytecode VM                  ║");
    println!("╚════════════════════════════════════════════════════════════╝\n");

    demo_basic_arithmetic();
    demo_loop();
    demo_recursive_cycle();
    demo_string_ops();

    println!("\n╔════════════════════════════════════════════════════════════╗");
    println!("║  HLX Runtime: OPERATIONAL                                  ║");
    println!("║  Bytecode VM: WORKING                                      ║");
    println!("║  Ready for: HLX compiler integration                       ║");
    println!("╚════════════════════════════════════════════════════════════╝");
}

fn demo_basic_arithmetic() {
    println!("═══ Demo 1: Basic Arithmetic ═══");

    let mut bc = Bytecode::new();

    let a = bc.add_constant(Value::I64(10));
    let b = bc.add_constant(Value::I64(32));

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(a);

    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(b);

    bc.emit(Opcode::Add);
    bc.emit_u8(0);
    bc.emit_u8(1);
    bc.emit_u8(2);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("  10 + 32 = {}", result);
    assert_eq!(result, Value::I64(42));
    println!("  ✓ Passed\n");
}

fn demo_loop() {
    println!("═══ Demo 2: Loop (sum 1 to 5) ═══");

    let mut bc = Bytecode::new();

    let zero = bc.add_constant(Value::I64(0));
    let one = bc.add_constant(Value::I64(1));
    let six = bc.add_constant(Value::I64(6));

    // r0 = sum (starts at 0)
    bc.emit(Opcode::Const);
    bc.emit_u8(0);
    bc.emit_u32(zero);

    // r1 = counter (starts at 1)
    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(one);

    // r2 = limit (6)
    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(six);

    // r3 = increment value (1)
    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    bc.emit_u32(one);

    let loop_start = bc.code.len();

    // r4 = r1 < r2
    bc.emit(Opcode::Lt);
    bc.emit_u8(4);
    bc.emit_u8(1);
    bc.emit_u8(2);

    // if not r4, jump to end
    bc.emit(Opcode::JumpIfNot);
    bc.emit_u8(4);
    let jump_end = bc.code.len();
    bc.emit_u32(0);

    // r0 = r0 + r1 (sum += counter)
    bc.emit(Opcode::Add);
    bc.emit_u8(0);
    bc.emit_u8(0);
    bc.emit_u8(1);

    // r1 = r1 + r3 (counter += 1)
    bc.emit(Opcode::Add);
    bc.emit_u8(1);
    bc.emit_u8(1);
    bc.emit_u8(3);

    // jump to loop start
    bc.emit(Opcode::Jump);
    bc.emit_u32(loop_start as u32);

    let loop_end = bc.code.len();
    let mut code = bc.code.clone();
    code[jump_end..jump_end + 4].copy_from_slice(&(loop_end as u32).to_le_bytes());
    bc.code = code;

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new().with_max_steps(10000);
    let result = vm.run(&bc).unwrap();

    println!("  sum(1..5) = {}", result);
    assert_eq!(result, Value::I64(15));
    println!("  ✓ Passed\n");
}

fn demo_recursive_cycle() {
    println!("═══ Demo 3: Recursive Cycle (TRM-style) ═══");

    let mut bc = Bytecode::new();

    let zero = bc.add_constant(Value::I64(0));
    let one = bc.add_constant(Value::I64(1));
    let three = bc.add_constant(Value::I64(3));
    let six = bc.add_constant(Value::I64(6));

    bc.emit(Opcode::Const);
    bc.emit_u8(0);
    bc.emit_u32(zero);

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(one);

    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(zero);

    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    bc.emit_u32(three);

    let outer_start = bc.code.len();

    bc.emit(Opcode::Lt);
    bc.emit_u8(4);
    bc.emit_u8(0);
    bc.emit_u8(3);

    bc.emit(Opcode::JumpIfNot);
    bc.emit_u8(4);
    let outer_end_jump = bc.code.len();
    bc.emit_u32(0);

    bc.emit(Opcode::Const);
    bc.emit_u8(5);
    bc.emit_u32(zero);

    bc.emit(Opcode::Const);
    bc.emit_u8(6);
    bc.emit_u32(six);

    let inner_start = bc.code.len();

    bc.emit(Opcode::Lt);
    bc.emit_u8(7);
    bc.emit_u8(5);
    bc.emit_u8(6);

    bc.emit(Opcode::JumpIfNot);
    bc.emit_u8(7);
    let inner_end_jump = bc.code.len();
    bc.emit_u32(0);

    bc.emit(Opcode::Add);
    bc.emit_u8(2);
    bc.emit_u8(2);
    bc.emit_u8(1);

    bc.emit(Opcode::Add);
    bc.emit_u8(5);
    bc.emit_u8(5);
    bc.emit_u8(1);

    bc.emit(Opcode::Jump);
    bc.emit_u32(inner_start as u32);

    let inner_end = bc.code.len();
    let mut code = bc.code.clone();
    code[inner_end_jump..inner_end_jump + 4].copy_from_slice(&(inner_end as u32).to_le_bytes());
    bc.code = code;

    bc.emit(Opcode::Add);
    bc.emit_u8(0);
    bc.emit_u8(0);
    bc.emit_u8(1);

    bc.emit(Opcode::Jump);
    bc.emit_u32(outer_start as u32);

    let outer_end = bc.code.len();
    let mut code = bc.code.clone();
    code[outer_end_jump..outer_end_jump + 4].copy_from_slice(&(outer_end as u32).to_le_bytes());
    bc.code = code;

    bc.emit(Opcode::Move);
    bc.emit_u8(0);
    bc.emit_u8(2);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new().with_max_steps(100000);
    let result = vm.run(&bc).unwrap();

    println!("  H_cycles=3, L_cycles=6, refinements = {}", result);
    assert_eq!(result, Value::I64(18));
    println!("  (3 outer × 6 inner = 18 refinements)");
    println!("  ✓ TRM-style cycles work\n");
}

fn demo_string_ops() {
    println!("═══ Demo 4: String Operations ═══");

    let mut bc = Bytecode::new();

    let hello = bc.add_constant(Value::String("Hello, ".to_string()));
    let world = bc.add_constant(Value::String("HLX!".to_string()));

    bc.emit(Opcode::Const);
    bc.emit_u8(0);
    bc.emit_u32(hello);

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(world);

    bc.emit(Opcode::Concat);
    bc.emit_u8(2);
    bc.emit_u8(0);
    bc.emit_u8(1);

    bc.emit(Opcode::StrLen);
    bc.emit_u8(3);
    bc.emit_u8(2);

    bc.emit(Opcode::Move);
    bc.emit_u8(0);
    bc.emit_u8(2);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("  Concatenated: \"{}\"", result);
    println!("  ✓ String ops work\n");
}
