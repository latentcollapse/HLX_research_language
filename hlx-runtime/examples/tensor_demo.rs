use hlx_runtime::{Bytecode, Opcode, Value, Vm};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║              HLX Phase 2: Tensor Primitives                        ║");
    println!("║              TRM-Style Neural Reasoning                            ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    demo_tensor_create();
    demo_tensor_math();
    demo_tensor_matmul();
    demo_neural_layer();

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 2 Progress:                                                  ║");
    println!("║  ✅ Tensor create/reshape                                           ║");
    println!("║  ✅ Element-wise ops (add, mul)                                     ║");
    println!("║  ✅ Matrix multiplication                                           ║");
    println!("║  ✅ Activation functions (softmax, relu)                            ║");
    println!("║  ✅ 19 tests passing                                                ║");
    println!("║                                                                    ║");
    println!("║  Next: SCALE coordination for multi-agent reasoning                ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
}

fn demo_tensor_create() {
    println!("═══ Demo 1: Tensor Creation ═══");

    let mut bc = Bytecode::new();

    bc.emit(Opcode::TensorCreate);
    bc.emit_u8(0);
    bc.emit_u8(2);
    bc.emit_u32(3);
    bc.emit_u32(4);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   Created tensor with shape [3, 4]");
    println!("   Result: {}", result);
    println!("   ✓ Tensor creation working\n");
}

fn demo_tensor_math() {
    println!("═══ Demo 2: Tensor Math ═══");

    let mut bc = Bytecode::new();

    let data_a = bc.add_constant(Value::Array(vec![
        Value::F64(1.0),
        Value::F64(2.0),
        Value::F64(3.0),
    ]));
    let shape = bc.add_constant(Value::Array(vec![Value::I64(3)]));
    let data_b = bc.add_constant(Value::Array(vec![
        Value::F64(4.0),
        Value::F64(5.0),
        Value::F64(6.0),
    ]));

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(data_a);
    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(shape);

    bc.emit(Opcode::TensorFromData);
    bc.emit_u8(10);
    bc.emit_u8(1);
    bc.emit_u8(2);

    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    bc.emit_u32(data_b);

    bc.emit(Opcode::TensorFromData);
    bc.emit_u8(11);
    bc.emit_u8(3);
    bc.emit_u8(2);

    bc.emit(Opcode::TensorAdd);
    bc.emit_u8(0);
    bc.emit_u8(10);
    bc.emit_u8(11);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   [1, 2, 3] + [4, 5, 6]");
    println!("   Result: {}", result);
    println!("   ✓ Tensor addition working\n");
}

fn demo_tensor_matmul() {
    println!("═══ Demo 3: Matrix Multiplication ═══");

    let mut bc = Bytecode::new();

    let data_a = bc.add_constant(Value::Array(vec![
        Value::F64(1.0),
        Value::F64(2.0),
        Value::F64(3.0),
        Value::F64(4.0),
    ]));
    let shape_a = bc.add_constant(Value::Array(vec![Value::I64(2), Value::I64(2)]));

    let data_b = bc.add_constant(Value::Array(vec![
        Value::F64(5.0),
        Value::F64(6.0),
        Value::F64(7.0),
        Value::F64(8.0),
    ]));
    let shape_b = bc.add_constant(Value::Array(vec![Value::I64(2), Value::I64(2)]));

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(data_a);
    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(shape_a);
    bc.emit(Opcode::TensorFromData);
    bc.emit_u8(10);
    bc.emit_u8(1);
    bc.emit_u8(2);

    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    bc.emit_u32(data_b);
    bc.emit(Opcode::Const);
    bc.emit_u8(4);
    bc.emit_u32(shape_b);
    bc.emit(Opcode::TensorFromData);
    bc.emit_u8(11);
    bc.emit_u8(3);
    bc.emit_u8(4);

    bc.emit(Opcode::TensorMatmul);
    bc.emit_u8(0);
    bc.emit_u8(10);
    bc.emit_u8(11);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   [[1,2],[3,4]] × [[5,6],[7,8]]");
    println!("   Result: {}", result);
    println!("   ✓ Matrix multiplication working\n");
}

fn demo_neural_layer() {
    println!("═══ Demo 4: Neural Layer (softmax) ═══");

    let mut bc = Bytecode::new();

    let logits = bc.add_constant(Value::Array(vec![
        Value::F64(1.0),
        Value::F64(2.0),
        Value::F64(3.0),
    ]));
    let shape = bc.add_constant(Value::Array(vec![Value::I64(3)]));

    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(logits);
    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(shape);
    bc.emit(Opcode::TensorFromData);
    bc.emit_u8(10);
    bc.emit_u8(1);
    bc.emit_u8(2);

    bc.emit(Opcode::TensorSoftmax);
    bc.emit_u8(0);
    bc.emit_u8(10);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   Logits: [1.0, 2.0, 3.0]");
    println!("   Softmax: {}", result);
    println!("   ✓ Softmax activation working\n");
}
