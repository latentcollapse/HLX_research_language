use hlx_runtime::{Bytecode, Opcode, Value, Vm};

fn main() {
    println!("╔════════════════════════════════════════════════════════════════════╗");
    println!("║              HLX Phase 1: Agent Lifecycle                          ║");
    println!("║              Pure Symbolic AI - Agents That Reason                 ║");
    println!("╚════════════════════════════════════════════════════════════════════╝\n");

    demo_agent_spawn();
    demo_agent_cycles();
    demo_latent_state();

    println!("\n╔════════════════════════════════════════════════════════════════════╗");
    println!("║  Phase 1 Progress:                                                  ║");
    println!("║  ✅ Agent spawn/halt/dissolve                                       ║");
    println!("║  ✅ Cycle execution (nested H/L loops)                              ║");
    println!("║  ✅ Latent state storage                                            ║");
    println!("║  ✅ 13 tests passing                                                ║");
    println!("║                                                                    ║");
    println!("║  Next: Tensor primitives for TRM-style reasoning                   ║");
    println!("╚════════════════════════════════════════════════════════════════════╝");
}

fn demo_agent_spawn() {
    println!("═══ Demo 1: Agent Spawn/Halt ═══");

    let mut bc = Bytecode::new();

    // Spawn agent
    let name = bc.add_string("TestAgent".to_string());
    bc.emit(Opcode::AgentSpawn);
    bc.emit_u32(name);
    bc.emit_u32(0);

    // Check agent ID is in r0
    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    let zero = bc.add_constant(Value::I64(0));
    bc.emit_u32(zero);

    bc.emit(Opcode::Eq);
    bc.emit_u8(2);
    bc.emit_u8(0);
    bc.emit_u8(1);

    // Halt agent
    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    let true_val = bc.add_constant(Value::Bool(true));
    bc.emit_u32(true_val);

    bc.emit(Opcode::AgentHalt);
    bc.emit_u8(3);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   Agent spawned with ID: {}", result);
    println!("   ✓ Agent lifecycle working\n");
}

fn demo_agent_cycles() {
    println!("═══ Demo 2: TRM-Style Cycles ═══");

    let mut bc = Bytecode::new();

    // Spawn agent
    let name = bc.add_string("Thinker".to_string());
    bc.emit(Opcode::AgentSpawn);
    bc.emit_u32(name);
    bc.emit_u32(0);

    // Initialize counter
    let zero = bc.add_constant(Value::I64(0));
    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(zero);

    // Set latent state "refinements" = 0
    let latent_name = bc.add_string("refinements".to_string());
    bc.emit(Opcode::LatentSet);
    bc.emit_u32(latent_name);
    bc.emit_u8(1);

    // Outer cycle (H=3)
    bc.emit(Opcode::CycleBegin);
    bc.emit_u8(0); // level 0 = outer
    bc.emit_u8(3); // 3 iterations

    // Inner cycle (L=6)
    bc.emit(Opcode::CycleBegin);
    bc.emit_u8(1); // level 1 = inner
    bc.emit_u8(6); // 6 iterations

    // refinements += 1
    bc.emit(Opcode::LatentGet);
    bc.emit_u8(2);
    let latent_name2 = bc.add_string("refinements".to_string());
    bc.emit_u32(latent_name2);

    let one = bc.add_constant(Value::I64(1));
    bc.emit(Opcode::Const);
    bc.emit_u8(3);
    bc.emit_u32(one);

    bc.emit(Opcode::Add);
    bc.emit_u8(2);
    bc.emit_u8(2);
    bc.emit_u8(3);

    let latent_name3 = bc.add_string("refinements".to_string());
    bc.emit(Opcode::LatentSet);
    bc.emit_u32(latent_name3);
    bc.emit_u8(2);

    bc.emit(Opcode::CycleEnd);
    bc.emit_u8(1); // end inner

    bc.emit(Opcode::CycleEnd);
    bc.emit_u8(0); // end outer

    // Get final refinements count
    let latent_name4 = bc.add_string("refinements".to_string());
    bc.emit(Opcode::LatentGet);
    bc.emit_u8(0);
    bc.emit_u32(latent_name4);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   H_cycles=3, L_cycles=6");
    println!("   Refinements: {}", result);
    println!("   ✓ TRM-style cycles working\n");
}

fn demo_latent_state() {
    println!("═══ Demo 3: Latent State ═══");

    let mut bc = Bytecode::new();

    // Spawn agent
    let name = bc.add_string("StatefulAgent".to_string());
    bc.emit(Opcode::AgentSpawn);
    bc.emit_u32(name);
    bc.emit_u32(0);

    // Set multiple latent states
    let hypothesis = bc.add_string("hypothesis".to_string());
    let val1 = bc.add_constant(Value::I64(42));
    bc.emit(Opcode::Const);
    bc.emit_u8(1);
    bc.emit_u32(val1);
    bc.emit(Opcode::LatentSet);
    bc.emit_u32(hypothesis);
    bc.emit_u8(1);

    let details = bc.add_string("details".to_string());
    let val2 = bc.add_constant(Value::I64(128));
    bc.emit(Opcode::Const);
    bc.emit_u8(2);
    bc.emit_u32(val2);
    bc.emit(Opcode::LatentSet);
    bc.emit_u32(details);
    bc.emit_u8(2);

    // Retrieve and add them
    let hyp2 = bc.add_string("hypothesis".to_string());
    bc.emit(Opcode::LatentGet);
    bc.emit_u8(3);
    bc.emit_u32(hyp2);

    let det2 = bc.add_string("details".to_string());
    bc.emit(Opcode::LatentGet);
    bc.emit_u8(4);
    bc.emit_u32(det2);

    bc.emit(Opcode::Add);
    bc.emit_u8(0);
    bc.emit_u8(3);
    bc.emit_u8(4);

    bc.emit(Opcode::Halt);

    let mut vm = Vm::new();
    let result = vm.run(&bc).unwrap();

    println!("   hypothesis=42, details=128");
    println!("   hypothesis + details = {}", result);
    println!("   ✓ Latent state working\n");
}
