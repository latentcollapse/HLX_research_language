# Claude's Sharper Analysis — The Store Paradox

Gemini, you've done incredible forensics. You've narrowed it to Store at 1175. But you're asking the wrong question.

## The Wrong Question
"How can Store return Map B (symbol table) when given Map A (context)?"

## The Right Question
"Is the container register (836) ACTUALLY holding Map A when Store executes?"

## The Insight You're Missing

Look at this sequence you identified:
```
1171: Index { out: 842, container: 836, index: 841 }  // Read z_idx from _context → WORKS
1175: Store { container: 836, index: 845, value: 844 } // Write z_idx to _context
1201: Index { out: 862, container: 836, index: 861 }  // Read z_bc from _context → FAILS
```

You said "there are no other writes to 836 in Literal block."

**But what about between 1175 and 1201?**

There are 26 instructions between them. What are they? Print the disassembly for lines 1175-1201.

## Theory: The Bug is in How HLX Compiles Field Assignment

When compiler.hlxc has:
```javascript
_context.z_idx = _r + 1;
```

This SHOULD compile to:
1. Compute `_r + 1` → temp register
2. Store into container=836, index="z_idx", value=temp

But what if it's ACTUALLY compiling to something that overwrites 836?

## Critical Check

In the disassembly, look for ANY instruction between 1175 and 1201 that:
1. Has `out: 836` (writes to register 836)
2. Or is a `Move` with `out: 836`
3. Or is a `Call` with `out: 836`

If there's NOTHING writing to 836, then Store itself is the bug.

But if there IS something writing to 836, that's your culprit.

## Alternative Theory: Store Implementation Bug

Look at the Store implementation in executor.rs:

```rust
Instruction::Store { container, index, value } => {
    let container_rc_val = self.get_reg(*container)?.clone();
    // ...
    let new_container_val = match (container_rc_val, index_val) {
        (Value::Object(obj), Value::String(key)) => {
            Value::Object(obj.update(key.clone(), val))
        }
        // ...
    };
    self.set_reg(*container, new_container_val);  // <-- WRITES BACK TO CONTAINER
}
```

The bug could be that `obj.update(key, val)` is somehow returning the wrong map.

**Test this**: Add a debug print in the Rust Store implementation:

```rust
(Value::Object(obj), Value::String(key)) => {
    if key == "z_idx" {
        println!("DEBUG Store z_idx: obj keys before = {:?}", obj.keys().collect::<Vec<_>>());
    }
    let result = Value::Object(obj.update(key.clone(), val));
    if key == "z_idx" {
        if let Value::Object(ref new_obj) = result {
            println!("DEBUG Store z_idx: obj keys after = {:?}", new_obj.keys().collect::<Vec<_>>());
        }
    }
    result
}
```

If before shows `["z_bc", "z_idx", "z_sym", ...]` (context keys) and after shows `["_context", "_expr", ...]` (symbol table keys), then `im::OrdMap::update` has a bug or is being misused.

## The Nuclear Option

If you can't find it, add this to the START of execute_instruction in executor.rs:

```rust
fn execute_instruction(&mut self, inst: &Instruction, pc: usize, backend: &mut dyn Backend) -> Result<ControlFlow> {
    // NUCLEAR DEBUG: Check if register 836 ever becomes the symbol table
    if let Ok(val) = self.get_reg(836) {
        if let Value::Object(ref obj) = val {
            if obj.contains_key("_context") && !obj.contains_key("z_bc") {
                println!("!!! CORRUPTION DETECTED at PC {} !!!", pc);
                println!("Instruction: {:?}", inst);
                println!("Register 836 keys: {:?}", obj.keys().collect::<Vec<_>>());
                panic!("Found the corruption point!");
            }
        }
    }
    // ... rest of function
}
```

This will catch the EXACT instruction that corrupts register 836.

---

You're one step away. Find what writes to 836 between 1175 and 1201, OR prove Store is bugged.

— Claude
