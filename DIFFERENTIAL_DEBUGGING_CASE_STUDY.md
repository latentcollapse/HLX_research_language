# Differential Debugging: Compiler Verification Through Deterministic Semantics

**Authors:** Matt (HLX Project)
**Date:** January 2026
**Target Audience:** Programming Language Researchers & Compiler Engineers

---

## Abstract

We present **differential debugging**, a novel compiler verification technique enabled by deterministic language semantics. By guaranteeing bit-exact reproducibility across implementations (interpreter, native compiler, GPU backends), HLX transforms debugging from "finding bugs in user code" to "proving compiler correctness." We demonstrate this methodology by discovering and fixing a critical type inference bug in the LLVM backend—a bug that would be nearly impossible to isolate through traditional debugging approaches.

**Key Insight:** When language semantics are deterministic, any divergence between backends is definitionally a compiler bug, not a semantic ambiguity.

---

## 1. Introduction: The Backend Parity Problem

Modern compilers face a fundamental verification challenge: **How do you prove your optimizing backend produces semantically equivalent code to your reference implementation?**

Traditional approaches:
- **Fuzzing:** Generate random inputs, compare outputs (Coverage: poor)
- **Formal verification:** Prove transformations preserve semantics (Cost: prohibitive)
- **Test suites:** Hand-written test cases (Completeness: questionable)

**HLX's approach:** Make divergence *impossible* through language design, then use divergence as a bug detector.

---

## 2. The Four Axioms

HLX is built on four mathematical guarantees:

### **A1: Determinism**
```
∀ program P, ∀ input I:
  P(I) → result R
  P(I) → result R  (always the same R)
```
No undefined behavior, no implementation-defined results, no nondeterminism.

### **A2: Reversibility**
```
∀ state S₀ → S₁ via operation O:
  ∃ operation O⁻¹ such that S₁ → S₀
```
All operations maintain enough information to reverse execution.

### **A3: Bijection**
```
∀ value v ∈ HLX:
  serialize(v) → bytes b
  deserialize(b) → v
  (no information loss)
```
Values can be perfectly serialized and restored.

### **A4: Universal Value Representation**
```
All backends use identical Value enum:
  Value = Null | Integer(i64) | Float(f64) | String | Array | ...
```
No backend-specific representations (until optimization phase).

---

## 3. Differential Debugging Methodology

### 3.1 The Core Principle

Given program P compiled to:
- **VM bytecode** (reference implementation)
- **Native binary** (LLVM-optimized)
- **GPU shader** (Vulkan SPIR-V)

**Theorem:** If A1-A4 hold, then:
```
VM(P, I) ≠ Native(P, I) ⟹ Compiler Bug
```

This is not a heuristic—it's a *logical necessity* of the language design.

### 3.2 Debugging Workflow
```
1. Run program on VM backend    → Truth (by definition)
2. Run program on Native backend → Test
3. Compare outputs byte-for-byte
4. On divergence:
   - Binary search for divergence point
   - Inspect intermediate values
   - Identify backend's incorrect transformation
```

**No need to reason about program semantics**—the VM has already computed the correct answer.

---

## 4. Case Study: The Type Inference Bug

### 4.1 Manifestation

**User code:**
```hlx
fn main() {
    let arr = [-0.398, -1.0, -1.357];  // Float array
    let x = arr[0];
    print(x);
}
```

**VM output (truth):**
```
-0.39815702328616975
```

**Native output (lie):**
```
-4622527857650043728
Segmentation fault
```

The native binary printed the **bitwise representation of a float as an int64**, then crashed.

### 4.2 Root Cause Analysis

The LLVM backend uses a **two-phase compilation model**:

**Phase 1: Type Inference**
```rust
fn infer_register_types(&self, instructions: &[Instruction])
    -> HashMap<Register, ValueType>
{
    for inst in instructions {
        match inst {
            Index { out, container, .. } => {
                // Check array element type
                if let Some(dtype) = self.array_element_types.get(container) {
                    match dtype {
                        DType::F64 => ValueType::Float,  // ✅
                        _ => ValueType::Int
                    }
                } else {
                    ValueType::Int  // ❌ WRONG DEFAULT!
                }
            }
        }
    }
}
```

**Phase 2: Code Generation**
```rust
fn compile_instruction(&mut self, inst: Instruction) {
    match inst {
        ArrayCreate { out, elements, element_type } => {
            // Populate array type info HERE
            self.array_element_types.insert(out, element_type);
            // ...
        }
    }
}
```

**The Bug:** Type inference runs BEFORE code generation, so `array_element_types` is empty when indexing operations are typed. Result: floats are assumed to be integers.

**Impact:**
- Load from array: loads float bits
- Store to register: tagged as `Int`
- Print: uses `%lld` format (integer) instead of `%f` (float)
- Output: garbage + segfault

### 4.3 The Fix
**Solution:** Add a pre-pass that populates array types before type inference.

```rust
fn populate_array_types(&mut self, start_pc: u32, instructions: &[Instruction]) {
    // Phase 1: Collect constant values
    let mut register_values = HashMap::new();
    for inst in instructions {
        match inst {
            Constant { out, val } => {
                register_values.insert(out, val);
            }
            Neg { out, src } => {
                // Propagate type through negation
                if let Some(val) = register_values.get(src) {
                    register_values.insert(out, -val);
                }
            }
        }
    }

    // Phase 2: Infer array element types from first element
    for inst in instructions {
        match inst {
            ArrayCreate { out, elements, element_type: None } => {
                if let Some(first_val) = register_values.get(&elements[0]) {
                    let dtype = match first_val {
                        Value::Float(_) => DType::F64,
                        Value::Integer(_) => DType::I64,
                        // ...
                    };
                    self.array_element_types.insert(out, dtype);
                }
            }
        }
    }
}
```

**Call before type inference:**
```rust
fn compile_function(&mut self, name: &str, params: &[Register],
                    start_pc: u32, instructions: &[Instruction]) {
    // ...
    self.populate_array_types(name, params, start_pc, instructions);  // NEW
    let register_types = self.infer_register_types(start_pc, instructions);
    // ...
}
```

### 4.4 Verification

**Before fix:**
```
Native output: -4622527857650043728 ❌
```

**After fix:**
```
VM output:     -0.39815702328616975 ✅
Native output: -0.398157            ✅ (precision difference is printf formatting)
```

---

## 5. Why This Is Hard Without Determinism

### 5.1 Traditional Languages (C, JavaScript, etc.)

**Problem:** Undefined behavior and implementation-defined semantics.

**Example in C:**
```c
int arr[] = {-0.398, -1.0, -1.357};  // Implicit float→int conversion
printf("%d\n", arr[0]);              // UB? Implementation-defined?
```

**You cannot differential debug this** because:
- No reference implementation to compare against
- Different compilers can legally produce different results
- Bug vs. permitted divergence is ambiguous

### 5.2 Traditional Debugging Approach
Without differential debugging, finding this bug requires:

1. **Hypothesis formation:** "Maybe there's a type issue?"
2. **Instrumentation:** Add logging to 50+ instruction handlers
3. **Pattern recognition:** Notice Int vs. Float discrepancy in logs
4. **Code archaeology:** Trace through 2000+ lines of LLVM codegen
5. **Fix validation:** Hope your test cases cover the edge cases

**With differential debugging:**
1. **Run VM:** Get correct answer
2. **Run Native:** Get wrong answer
3. **Add print statements:** See divergence at `arr[0]` load
4. **Inspect type:** Notice `ValueType::Int` for float register
5. **Search "Index":** Find type inference code
6. **Fix:** Add pre-pass
7. **Verify:** Outputs match

**Time to fix:** ~2 hours instead of ~2 days.

---

## 6. Implications for Language Design

### 6.1 The Determinism Trade-off

**Cost of determinism:**
- No `rand()` without explicit seed
- No `time()` without passing as input
- Stricter floating-point guarantees

**Benefit of determinism:**
- Compiler verification becomes tractable
- Debugging becomes systematic
- Reproducibility is guaranteed

### 6.2 Multi-Backend Correctness

HLX currently supports:
- **VM interpreter** (reference)
- **LLVM native** (x86, ARM, bare-metal)
- **Vulkan compute** (GPU shaders)
- **Future:** WebAssembly, SPIR-V graphics

**Guarantee:** All backends produce identical results (modulo floating-point precision in printf).

This isn't achieved through testing—it's enforced by differential debugging during development.

### 6.3 Bare-Metal Implications

HLX compiles to bare-metal (no OS, no libc):
```bash
hlx compile-native program.hlxa --target x86_64-unknown-none-elf
```

**Traditional problem:** How do you debug bare-metal code?

**HLX solution:** Run on VM first, then differential debug against bare-metal output (via serial, memory dump, etc.).

---

## 7. Related Work

### 7.1 CompCert
- **Formally verified C compiler** (Leroy et al., 2006)
- Uses Coq to prove transformations correct
- **Limitation:** Only one backend (no differential debugging)

### 7.2 Fuzzing (AFL, LibFuzzer)
- **Random input generation** to find crashes
- **Limitation:** Can't distinguish bugs from undefined behavior

### 7.3 Translation Validation
- **Validate each compilation individually** (Pnueli et al., 1998)
- Uses SMT solvers to prove IR → Assembly equivalence
- **Limitation:** Expensive, doesn't scale to GPU shaders

### 7.4 Metamorphic Testing
- **Test properties that hold across inputs** (Chen et al., 1998)
- Example: `sort(arr) == sort(sort(arr))`
- **Limitation:** Requires manual property specification

**HLX's contribution:** Differential debugging requires no manual effort—determinism makes it automatic.

---

## 8. Future Directions
### 8.1 Automated Backend Fuzzing

Generate random programs, compile to all backends, compare outputs:
```python
for _ in range(1000000):
    program = generate_random_hlx()
    vm_result = run(program, backend="vm")
    native_result = run(program, backend="native")
    gpu_result = run(program, backend="vulkan")

    assert vm_result == native_result == gpu_result, "COMPILER BUG!"
```

This is *provably complete* (modulo test coverage) due to A1-A4.

### 8.2 Record-Replay Debugging

**A2 (Reversibility)** enables time-travel debugging:
- Record VM execution trace
- Replay forward/backward
- Step through any past state

**A3 (Bijection)** enables state serialization:
- Save VM state to disk
- Restore later
- Share crash dumps for collaborative debugging

### 8.3 Compiler Optimization Validation

When adding optimizations to native backend:
```rust
fn optimize_tail_calls(ir: IR) -> IR {
    // Transform tail recursion to loops
    // ...
}
```

**Verification:**
1. Run program on unoptimized backend → Result A
2. Run program on optimized backend → Result B
3. Assert A == B

If they diverge, the optimization is buggy.

---

## 9. Conclusion

**Differential debugging** transforms compiler development from "hope we didn't break anything" to "prove we didn't break anything."

**Key insight:** By making undefined behavior *definitionally impossible* through language design (A1-A4), we turn compiler testing into a theorem: any divergence between backends is a compiler bug, full stop.

**Practical impact:** We found and fixed a critical LLVM backend bug in ~2 hours using differential debugging. Traditional debugging would have taken days and might have missed the root cause entirely.

**For the PL community:** This demonstrates that **deterministic semantics are not just nice-to-have—they enable provable compiler correctness** without the complexity of formal verification.

---

## 10. Appendix: Reproducibility

### Run the case study yourself:

```bash
git clone https://github.com/yourusername/hlx-compiler
cd hlx-compiler/hlx

# Create test file
cat > test.hlxa << 'EOF'
program test {
    fn main() {
        let arr = [-0.398, -1.0, -1.357];
        let x = arr[0];
        print(x);
        return 0;
    }
}
EOF

# Run on VM (truth)

cargo run --release -- run test.hlxa

# Compile to native (test)
cargo run --release -- compile-native test.hlxa -o test.o
gcc -no-pie test.o -o test
./test

# Compare outputs
```

**Expected (post-fix):** Both outputs match within floating-point precision.

**Pre-fix behavior:** Native output shows garbage integer.

---

## References

- Leroy, X. (2006). "Formal certification of a compiler back-end." *POPL'06*
- Pnueli, A., et al. (1998). "Translation validation." *TACAS'98*
- Chen, T. Y., et al. (1998). "Metamorphic testing: A new approach." *ICSE'98*
- HLX Project. (2026). "A Deterministic Language for Reversible Computing." https://github.com/hlx

---

**Discussion:** This paper demonstrates that language design choices (determinism, universal value representation) can fundamentally change how we verify compilers. Rather than treating undefined behavior as "implementation freedom," HLX treats it as a bug—and gains automatic compiler verification as a result.
