# Native HLX Runtime Implementation Plan - Detailed

**Goal**: Build a bytecode interpreter written in HLX that can execute HLX bytecode without RustD

**Criticality**: HIGHEST - This is the bottleneck for all subsequent phases

**Complexity**: Very High (~1500 LOC, 40+ instruction handlers, register allocation, call stacks)

---

## Architecture Design

### 1. VM State Structure (Using Contracts)

```hlx
// Contract ID 200: VM State
type VM = {200:{
    @0: bytecode,        // [u8] - raw bytecode
    @1: pc,              // i64 - program counter
    @2: registers,       // [i64] - register file (r0, r1, ..., r63)
    @3: call_stack,      // [[i64]] - function call frames
    @4: halted,          // i64 - 0=running, 1=halted
    @5: return_value,    // i64 - last return value
    @6: inst_count,      // i64 - total instruction count
    @7: handle_table,    // [i64] - collapsed value storage
    @8: next_handle,     // i64 - next handle ID
    @9: loop_stack       // [[i64]] - loop tracking for break/continue
}};

// Contract ID 201: Instruction
type Instruction = {201:{
    @0: opcode,          // i64 - instruction type
    @1: operand1,        // i64 - first operand
    @2: operand2,        // i64 - second operand
    @3: operand3,        // i64 - third operand
    @4: operand4         // i64 - fourth operand
}};

// Contract ID 202: Call Frame
type CallFrame = {202:{
    @0: return_pc,       // i64 - return address
    @1: saved_regs,      // [i64] - saved registers
    @2: local_count      // i64 - local variable count
}};
```

### 2. Instruction Format

**Bytecode Layout**:
```
[Opcode:u8][Operand1:u32][Operand2:u32][Operand3:u32][Operand4:u32]
```

**Variable-length instructions** (opcodes determine operand count):
- 1 operand: CONSTANT, MOVE, RETURN, COLLAPSE, RESOLVE
- 2 operands: UNARY, COLLAPSE, RESOLVE
- 3 operands: BINOP, INDEX, CONTRACT_GET, CONTRACT_SET
- 4+ operands: CALL (variable args), CONTRACT_CREATE (variable fields)

---

## Implementation Roadmap

### Phase 4.1: VM State Initialization

**File**: `/home/matt/hlx-compiler/hlx/hlx_runtime/hlx_vm.hlx`

```hlx
module hlx_runtime {

    fn init_vm(bytecode: [i64]) -> [i64] {
        let registers = [];
        let i = 0;
        loop(i < 64, 64) {
            registers = push(registers, 0);
            i = i + 1;
        }

        let vm = {200:{
            @0: bytecode,
            @1: 0,              // pc = 0
            @2: registers,      // 64 zero registers
            @3: [],             // empty call stack
            @4: 0,              // halted = false
            @5: 0,              // return_value = 0
            @6: array_len(bytecode),
            @7: [],             // handle table = []
            @8: 0,              // next_handle = 0
            @9: []              // loop_stack = []
        }};

        return vm;
    }

    // Accessors (note: using numbers instead of names for indices)
    fn vm_bytecode(vm: [i64]) -> [i64] { return int_to_ptr(get_at(vm, 0)); }
    fn vm_pc(vm: [i64]) -> i64 { return get_at(vm, 1); }
    fn vm_registers(vm: [i64]) -> [i64] { return int_to_ptr(get_at(vm, 2)); }
    fn vm_call_stack(vm: [i64]) -> [i64] { return int_to_ptr(get_at(vm, 3)); }
    fn vm_halted(vm: [i64]) -> i64 { return get_at(vm, 4); }
    fn vm_return_value(vm: [i64]) -> i64 { return get_at(vm, 5); }
    fn vm_handle_table(vm: [i64]) -> [i64] { return int_to_ptr(get_at(vm, 7)); }

    // Mutators
    fn set_vm_pc(vm: [i64], pc: i64) -> [i64] { return set_at(vm, 1, pc); }
    fn set_vm_halted(vm: [i64], halted: i64) -> [i64] { return set_at(vm, 4, halted); }
    fn set_vm_return_value(vm: [i64], value: i64) -> [i64] { return set_at(vm, 5, value); }

    // Register access
    fn get_register(vm: [i64], reg: i64) -> i64 {
        let regs = vm_registers(vm);
        return get_at(regs, reg);
    }

    fn set_register(vm: [i64], reg: i64, value: i64) -> [i64] {
        let regs = vm_registers(vm);
        regs = set_at(regs, reg, value);
        return set_at(vm, 2, ptr_to_int(regs));
    }
}
```

### Phase 4.2: Instruction Decode & Execute Loop

```hlx
    fn execute_crate(bytecode: [i64]) -> i64 {
        let vm = init_vm(bytecode);

        loop(vm_halted(vm) == 0, 1000000) {  // Max iterations
            if vm_pc(vm) >= array_len(bytecode) {
                break;
            }

            let inst = decode_instruction(vm);
            vm = execute_instruction(vm, inst);
        }

        return vm_return_value(vm);
    }

    fn decode_instruction(vm: [i64]) -> [i64] {
        let bytecode = vm_bytecode(vm);
        let pc = vm_pc(vm);

        if pc >= array_len(bytecode) {
            return [0, 0, 0, 0, 0];  // Invalid instruction
        }

        let opcode = get_at(bytecode, pc);

        // Extract operands based on opcode
        // (simplified: assume all opcodes have 4 operands for now)
        let op1 = if (pc + 1 < array_len(bytecode)) { get_at(bytecode, pc + 1); } else { 0; };
        let op2 = if (pc + 2 < array_len(bytecode)) { get_at(bytecode, pc + 2); } else { 0; };
        let op3 = if (pc + 3 < array_len(bytecode)) { get_at(bytecode, pc + 3); } else { 0; };
        let op4 = if (pc + 4 < array_len(bytecode)) { get_at(bytecode, pc + 4); } else { 0; };

        let inst = {201:{
            @0: opcode,
            @1: op1,
            @2: op2,
            @3: op3,
            @4: op4
        }};

        return inst;
    }

    fn execute_instruction(vm: [i64], inst: [i64]) -> [i64] {
        let opcode = get_at(inst, 0);

        switch opcode {
            1 => { vm = execute_constant(vm, inst); },
            2 => { vm = execute_move(vm, inst); },
            10 => { vm = execute_add(vm, inst); },
            11 => { vm = execute_sub(vm, inst); },
            // ... 40+ more cases
            51 => { vm = execute_return(vm, inst); },
            _ => {
                print("Unknown instruction: ");
                print_int(opcode);
                print("\n");
                vm = set_vm_halted(vm, 1);
            },
        }

        return vm;
    }
```

### Phase 4.3: Core Instruction Handlers

**Arithmetic Instructions**:
```hlx
    fn execute_add(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let lhs_reg = get_at(inst, 2);
        let rhs_reg = get_at(inst, 3);

        let lhs = get_register(vm, lhs_reg);
        let rhs = get_register(vm, rhs_reg);
        let result = lhs + rhs;

        vm = set_register(vm, out, result);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);  // Advance PC
        return vm;
    }

    fn execute_sub(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let lhs_reg = get_at(inst, 2);
        let rhs_reg = get_at(inst, 3);

        let lhs = get_register(vm, lhs_reg);
        let rhs = get_register(vm, rhs_reg);
        let result = lhs - rhs;

        vm = set_register(vm, out, result);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }

    fn execute_mul(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let lhs_reg = get_at(inst, 2);
        let rhs_reg = get_at(inst, 3);

        let lhs = get_register(vm, lhs_reg);
        let rhs = get_register(vm, rhs_reg);
        let result = lhs * rhs;

        vm = set_register(vm, out, result);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }

    // ... DIV, MOD similar pattern
```

**Comparison Instructions**:
```hlx
    fn execute_eq(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let lhs_reg = get_at(inst, 2);
        let rhs_reg = get_at(inst, 3);

        let lhs = get_register(vm, lhs_reg);
        let rhs = get_register(vm, rhs_reg);
        let result = if (lhs == rhs) { 1; } else { 0; };

        vm = set_register(vm, out, result);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }

    fn execute_lt(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let lhs_reg = get_at(inst, 2);
        let rhs_reg = get_at(inst, 3);

        let lhs = get_register(vm, lhs_reg);
        let rhs = get_register(vm, rhs_reg);
        let result = if (lhs < rhs) { 1; } else { 0; };

        vm = set_register(vm, out, result);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }

    // ... GT, LE, GE, NE similar
```

**Constant Loading**:
```hlx
    fn execute_constant(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let value = get_at(inst, 2);
        let is_string = get_at(inst, 3);

        if is_string == 1 {
            // String constant (value is pointer)
            // For now, treat as i64
        }

        vm = set_register(vm, out, value);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }

    fn execute_move(vm: [i64], inst: [i64]) -> [i64] {
        let out = get_at(inst, 1);
        let src = get_at(inst, 2);

        let value = get_register(vm, src);
        vm = set_register(vm, out, value);
        vm = set_vm_pc(vm, vm_pc(vm) + 5);
        return vm;
    }
```

**Control Flow**:
```hlx
    fn execute_return(vm: [i64], inst: [i64]) -> [i64] {
        let val_reg = get_at(inst, 1);
        let value = get_register(vm, val_reg);

        vm = set_vm_return_value(vm, value);
        vm = set_vm_halted(vm, 1);
        return vm;
    }

    fn execute_jump(vm: [i64], inst: [i64]) -> [i64] {
        let target = get_at(inst, 1);
        vm = set_vm_pc(vm, target);
        return vm;
    }

    fn execute_if(vm: [i64], inst: [i64]) -> [i64] {
        let cond_reg = get_at(inst, 1);
        let then_pc = get_at(inst, 2);
        let else_pc = get_at(inst, 3);

        let cond = get_register(vm, cond_reg);
        if cond != 0 {
            vm = set_vm_pc(vm, then_pc);
        } else {
            vm = set_vm_pc(vm, else_pc);
        }
        return vm;
    }
```

### Phase 4.4-4.5: Additional Handlers

- Array operations: GET_ELEMENT, SET_ELEMENT
- Contract operations: CONTRACT_CREATE, CONTRACT_GET, CONTRACT_SET
- Handle operations: COLLAPSE, RESOLVE
- Function calls: CALL, FUNCDEF
- Loop control: LOOP, BREAK, CONTINUE
- Bitwise: BIT_AND, BIT_OR, BIT_XOR, SHL, SHR
- Logical: AND, OR
- Unary: NOT

Total: ~40 handlers, each ~10-20 lines (400-800 LOC)

### Phase 4.6: Testing Strategy

**Test 1**: Arithmetic operations
```hlx
let bytecode = [
    1, 0, 42, 0,           // r0 = 42
    1, 1, 100, 0,          // r1 = 100
    10, 2, 0, 1,           // r2 = r0 + r1 (142)
    51, 2                  // return r2
];
assert(execute_crate(bytecode) == 142);
```

**Test 2**: Conditional
```hlx
let bytecode = [
    1, 0, 50, 0,           // r0 = 50
    1, 1, 100, 0,          // r1 = 100
    22, 2, 0, 1,           // r2 = (r0 < r1) = 1
    40, 2, <then>, <else>, // if r2 goto then else
    // else: r0 = 0
    1, 0, 0, 0,
    41, <end>,             // jump end
    // then: r0 = 1
    1, 0, 1, 0,
    // end:
    51, 0                  // return r0
];
```

**Test 3**: Self-hosting compiler
```
./hlx_vm compiler.lcc source.hlx > output.lcc
```

---

## Implementation Schedule

**Session 1** (Today):
- [ ] 4.1: VM state design and initialization (1 hour)
- [ ] 4.2: Instruction decode and execute loop (1.5 hours)
- [ ] 4.3: Core 20 instructions (arithmetic, comparison, control) (2 hours)

**Session 2** (Follow-up):
- [ ] 4.3: Remaining 20 instructions (2-3 hours)
- [ ] 4.4-4.5: Function calls, arrays, contracts (2 hours)
- [ ] 4.6: Testing and debugging (1-2 hours)

**Total**: ~10-12 hours of focused implementation

---

## Success Criteria

1. ✅ VM initializes without errors
2. ✅ Decodes instructions correctly
3. ✅ Arithmetic operations produce correct results
4. ✅ Comparisons work correctly
5. ✅ Control flow (if/else) works
6. ✅ Function calls work (CALL/RETURN)
7. ✅ Arrays work (GET_ELEMENT/SET_ELEMENT)
8. ✅ Contracts work (CREATE/GET/SET)
9. ✅ Can execute Fibonacci(10) = 55 correctly
10. ✅ Can execute self-hosting compiler: `./hlx_vm compiler.lcc source.hlx`

---

## Risk Mitigation

| Risk | Mitigation |
|------|-----------|
| Instruction decoding too slow | Optimize with better parsing, consider bytecode caching |
| Register allocation limits | Start with 64 registers, expand if needed |
| Stack overflow on deep recursion | Track call stack depth, error on limits |
| Handle table grows unbounded | Implement GC for unused handles (Phase 2 enhancement) |
| PC tracking bug causes infinite loops | Add instruction count limit, halt on max iterations |
| String handling undefined | Treat strings as integers initially (pointers), enhance later |

---

## Key Design Decisions

1. **Contract-based VM State**: Allows clean modeling, but requires contract support (✅ Phase 1 done)
2. **Fixed operand sizes**: Simplifies decoding, wastes some bytes (acceptable for MVP)
3. **Limited registers**: 64 registers should be enough for reasonable programs
4. **No JIT**: Interpret bytecode directly (faster to implement)
5. **No GC**: Require explicit handle cleanup (acceptable for MVP)
6. **No string tables**: Treat strings as pointers (enhancement later)

---

## Next Steps After Phase 4

Once native runtime works:
- Phase 2: Add handle garbage collection
- Phase 5: Implement axiom validators
- Phase 6: Boot Axiom Kernel in QEMU
- **Final Goal**: Kernel running on native HLX (no RustD dependency)

---

Document prepared for Phase 4 implementation
Target completion: This session or next session
Criticality: HIGHEST - Blocks all downstream phases
