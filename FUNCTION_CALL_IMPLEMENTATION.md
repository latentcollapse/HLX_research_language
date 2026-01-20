# Function Call Implementation for Native HLX VM
**Priority**: CRITICAL BLOCKER
**File**: `/home/matt/hlx-compiler/hlx/hlx_runtime/hlx_vm.hlx`
**Date**: 2026-01-20

---

## Problem Statement

The native HLX VM currently has stub implementations for:
- `execute_funcdef()` - Just skips over function definitions (line 760-765)
- `execute_call()` - Returns 0 instead of calling functions (line 767-780)
- `execute_return()` - May also be incomplete

This prevents multi-function modules from working, blocking kernel development.

**Test Case That Fails**:
```hlx
module test {
    fn main() {
        helper();  // ERROR: function call does nothing
    }

    fn helper() {
        // This code never runs
    }
}
```

---

## Solution Design

### 1. Add Function Table to VM State

**Current VM State** (lines 28-32 in hlx_vm.hlx):
```hlx
type VM = {10:{
    @0:registers,    // [i64] - 64 registers
    @1:pc,          // i64 - program counter
    @2:bytecode,    // [i64] - instruction stream
    @3:halted,      // i64 - halt flag
    @4:handle_table, // [i64] - for collapse/resolve
    @5:call_stack,  // [i64] - for function calls
    @6:loop_stack   // [i64] - for break/continue
}};
```

**Add Function Table**:
```hlx
type VM = {10:{
    @0:registers,
    @1:pc,
    @2:bytecode,
    @3:halted,
    @4:handle_table,
    @5:call_stack,
    @6:loop_stack,
    @7:function_table  // NEW: name -> PC mapping
}};
```

**Function Table Entry**:
```hlx
type FunctionEntry = {11:{
    @0:name,       // String - function name
    @1:pc_start,   // i64 - PC where function starts
    @2:max_depth   // i64 - recursion limit
}};
```

---

### 2. Update VM Constructor

**Add to `init_vm()` function** (around line 120):
```hlx
fn init_vm(bytecode: [i64]) -> [i64] {
    let registers = make_zero_array(64);
    let handle_table = [];
    let call_stack = [];
    let loop_stack = [];
    let function_table = [];  // NEW

    let vm = {10:{
        @0:registers,
        @1:0,  // pc
        @2:bytecode,
        @3:0,  // not halted
        @4:handle_table,
        @5:call_stack,
        @6:loop_stack,
        @7:function_table  // NEW
    }};

    return vm;
}
```

**Add Helper Functions**:
```hlx
fn vm_function_table(vm: [i64]) -> [i64] {
    return int_to_ptr(vm.@7);
}

fn set_vm_function_table(vm: [i64], table: [i64]) -> [i64] {
    return {10:{
        @0:vm.@0,
        @1:vm.@1,
        @2:vm.@2,
        @3:vm.@3,
        @4:vm.@4,
        @5:vm.@5,
        @6:vm.@6,
        @7:ptr_to_int(table)
    }};
}
```

---

### 3. Implement FUNCDEF Handler

**Replace stub at line 760**:
```hlx
fn execute_funcdef(vm: [i64], inst: [i64]) -> [i64] {
    // FUNCDEF instruction format:
    // [opcode=70, name_ptr, param_count, entry_pc, max_depth, ...]

    let name_ptr = get_at(inst, 1);
    let name = int_to_ptr(name_ptr);
    let param_count = get_at(inst, 2);
    let entry_pc = get_at(inst, 3);
    let max_depth = get_at(inst, 4);

    // Create function entry
    let func_entry = {11:{
        @0:ptr_to_int(name),
        @1:entry_pc,
        @2:max_depth
    }};

    // Add to function table
    let func_table = vm_function_table(vm);
    func_table = array_push(func_table, ptr_to_int(func_entry));
    vm = set_vm_function_table(vm, func_table);

    // Advance PC past this instruction
    vm = set_vm_pc(vm, vm_pc(vm) + 17);

    return vm;
}
```

---

### 4. Implement Function Lookup

**Add new helper function**:
```hlx
fn lookup_function(vm: [i64], name: [i64]) -> i64 {
    // Returns entry_pc or -1 if not found
    let func_table = vm_function_table(vm);
    let i = 0;

    loop(i < array_len(func_table), 1000) {
        let entry = int_to_ptr(get_at(func_table, i));
        let func_name = int_to_ptr(entry.@0);

        if (strings_equal(func_name, name)) {
            return entry.@1;  // Return entry_pc
        }

        i += 1;
    }

    return -1;  // Not found
}
```

---

### 5. Implement CALL Handler

**Replace stub at line 767**:
```hlx
fn execute_call(vm: [i64], inst: [i64]) -> [i64] {
    // CALL instruction format:
    // [opcode=50, out_reg, name_ptr, arg_count, arg_reg0, arg_reg1, ...]

    let out_reg = get_at(inst, 1);
    let name_ptr = get_at(inst, 2);
    let name = int_to_ptr(name_ptr);
    let arg_count = get_at(inst, 3);

    // Look up function
    let entry_pc = lookup_function(vm, name);

    if (entry_pc < 0) {
        print("ERROR: Unknown function: ");
        print(name);
        print("\n");
        vm = set_vm_halted(vm, 1);
        return vm;
    }

    // Create call frame
    let call_frame = {12:{
        @0:vm_pc(vm) + 17,    // return PC (after this instruction)
        @1:ptr_to_int(vm_registers(vm)),  // saved registers
        @2:out_reg            // where to store return value
    }};

    // Push call frame
    let call_stack = vm_call_stack(vm);
    call_stack = array_push(call_stack, ptr_to_int(call_frame));
    vm = set_vm_call_stack(vm, call_stack);

    // Copy arguments to registers for callee
    // For simplicity, args go to r0, r1, r2, ...
    let i = 0;
    loop(i < arg_count, 20) {
        let arg_reg = get_at(inst, 4 + i);
        let arg_value = get_register(vm, arg_reg);
        vm = set_register(vm, i, arg_value);
        i += 1;
    }

    // Jump to function entry
    vm = set_vm_pc(vm, entry_pc);

    return vm;
}
```

---

### 6. Implement RETURN Handler

**Check if exists, if stub then replace**:
```hlx
fn execute_return(vm: [i64], inst: [i64]) -> [i64] {
    // RETURN instruction format:
    // [opcode=51, value_reg, ..., ..., ...]

    let value_reg = get_at(inst, 1);
    let return_value = get_register(vm, value_reg);

    // Pop call frame
    let call_stack = vm_call_stack(vm);

    if (array_len(call_stack) == 0) {
        // Returning from main/entry - halt
        vm = set_vm_halted(vm, 1);
        return vm;
    }

    let frame_ptr = get_at(call_stack, array_len(call_stack) - 1);
    let frame = int_to_ptr(frame_ptr);
    call_stack = array_pop(call_stack);
    vm = set_vm_call_stack(vm, call_stack);

    // Extract call frame info
    let return_pc = frame.@0;
    let saved_registers = int_to_ptr(frame.@1);
    let out_reg = frame.@2;

    // Restore registers
    vm = set_vm_registers(vm, saved_registers);

    // Store return value in output register
    vm = set_register(vm, out_reg, return_value);

    // Jump to return address
    vm = set_vm_pc(vm, return_pc);

    return vm;
}
```

---

### 7. Add Helper Functions

**String comparison** (if not already present):
```hlx
fn strings_equal(s1: [i64], s2: [i64]) -> i64 {
    if (array_len(s1) != array_len(s2)) {
        return 0;
    }

    let i = 0;
    loop(i < array_len(s1), 10000) {
        if (get_at(s1, i) != get_at(s2, i)) {
            return 0;
        }
        i += 1;
    }

    return 1;
}
```

**Array operations** (if not already present):
```hlx
fn array_push(arr: [i64], value: i64) -> [i64] {
    let new_arr = make_zero_array(array_len(arr) + 1);
    let i = 0;

    loop(i < array_len(arr), 10000) {
        new_arr = set_at(new_arr, i, get_at(arr, i));
        i += 1;
    }

    new_arr = set_at(new_arr, array_len(arr), value);
    return new_arr;
}

fn array_pop(arr: [i64]) -> [i64] {
    if (array_len(arr) == 0) {
        return arr;
    }

    let new_arr = make_zero_array(array_len(arr) - 1);
    let i = 0;

    loop(i < array_len(arr) - 1, 10000) {
        new_arr = set_at(new_arr, i, get_at(arr, i));
        i += 1;
    }

    return new_arr;
}
```

---

## Testing Strategy

### Test 1: Simple Function Call
```hlx
module test {
    fn main() {
        let result = add_two(5);
        return result;  // Should return 7
    }

    fn add_two(x: i64) -> i64 {
        return x + 2;
    }
}
```

### Test 2: Nested Calls
```hlx
module test {
    fn main() {
        return outer(10);  // Should return 15
    }

    fn outer(x: i64) -> i64 {
        return inner(x) + 2;
    }

    fn inner(x: i64) -> i64 {
        return x + 3;
    }
}
```

### Test 3: Kernel Boot
```hlx
module boot {
    fn _start() {
        display_helinux();  // Should work now!
    }

    fn display_helinux() {
        // Write HELINUX to screen
    }
}
```

---

## Implementation Steps

1. ✅ Analyze current VM structure
2. ✅ Design function table schema
3. [ ] Add function_table field to VM state
4. [ ] Implement vm_function_table() and set_vm_function_table()
5. [ ] Implement execute_funcdef() to register functions
6. [ ] Implement lookup_function() helper
7. [ ] Implement execute_call() with call frame management
8. [ ] Implement execute_return() with frame pop
9. [ ] Add helper functions (strings_equal, array_push, array_pop)
10. [ ] Test with simple 2-function module
11. [ ] Test with nested calls
12. [ ] Test with kernel boot.hlx

---

## Expected Impact

After implementation:
- ✅ Multi-function modules will work
- ✅ Kernel helper functions can be called
- ✅ Recursion will work (with max_depth limits)
- ✅ Call stack traces become possible
- ✅ Full kernel boot sequence can execute

---

## Timeline

**Estimated**: 4-6 hours of focused work
**Priority**: Highest - blocks all kernel development
**Complexity**: Medium - well-defined problem, clear solution

---

## Success Criteria

1. `boot.hlx` compiles with native HLX compiler
2. All functions in module are registered in function table
3. Function calls resolve correctly
4. Call stack management works (push/pop frames)
5. Return values propagate correctly
6. Kernel displays "HELINUX" via helper functions
7. No "Unknown function" errors

---

## Notes

- Function table could be optimized with hash map later
- Linear search is fine for kernel (<20 functions)
- Call frame format can be extended for debugging
- Register save/restore may need optimization
- Consider separate parameter registers vs reusing r0-rN
