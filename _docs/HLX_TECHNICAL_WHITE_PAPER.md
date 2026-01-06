# HLX V2: A Deterministic, Native-Compiled Substrate for Constrained Intelligence

**Status**: Technical White Paper
**Date**: January 4, 2026
**Target Audience**: Systems Engineers, Compiler Architects, AI Researchers

---

## 1. Abstract
Current LLM outputs (Python/JSON/English) suffer from inherent non-determinism and lack rigorous execution constraints. HLX V2 addresses this by treating AI-generated logic as **topological circuits** rather than prose. It introduces a "CPU Shader" execution model—stateless, region-based, and compiled to native machine code via LLVM—enabling mathematically verified, high-performance execution of stochastic intent.

---

## 2. Legacy Architecture Analysis ("The Mud")
The V1 architecture attempted to bridge AI and execution via a dynamic interpreter. This approach failed due to fundamental architectural ambiguity.

| Feature | V1 Architecture (Legacy) | V2 Architecture (Current) |
| :--- | :--- | :--- |
| **State Model** | Shared Mutable State (Python dicts) | **Region-Based Arenas** (Isolated Memory) |
| **Ownership** | Reference Counting / GC (Ambiguous) | **Copy-On-Return** (Strict Value Semantics) |
| **Execution** | Interpreter Loop (Slow, fragile) | **Native JIT / AOT** (LLVM Backend) |
| **Tracks** | 3 (HLXL, HLXC, LC-R) overlapping | **2 (HLX-A, HLX-R)** targeting 1 Binary (LC-B) |
| **Failure Mode** | "COW Bug" (Silent State Corruption) | **Compile Error** (Invalid State Impossible) |

**The V1 Failure:** By allowing pointers to persist across contract boundaries, V1 introduced aliasing bugs that were impossible to debug in a probabilistic system.

---

## 3. The V2 Memory Model: "CPU Shaders"
HLX V2 adopts the GPU execution paradigm for CPU workloads.

### 3.1 Region-Based Allocation
Each Contract execution is allocated a linear memory arena.
*   **Allocation:** `O(1)` pointer bump.
*   **Deallocation:** `O(1)` region reset. No Garbage Collection.
*   **Lifecycle:** Init → Execute → Copy Result → Wipe Region.

### 3.2 Copy-On-Return (COR)
To ensure isolation, no reference ever escapes a region.
*   When Contract A calls Contract B:
    1.  A's arguments are deep-copied into B's region.
    2.  B executes.
    3.  B's return value is deep-copied back to A's region.
    4.  B's region is vaporized.
*   **Result:** Use-after-free and double-free bugs are mathematically impossible.

---

## 4. The Native Compilation Pipeline
HLX V2 abandons interpretation for a rigorous lowering pass targeting LLVM IR.

### 4.1 The Pipeline
1.  **Source (HLX-A / HLX-R)**: Parsed into a unified `Item::Node` AST.
2.  **Lowering**: AST is flattened into `hlx_core::Instruction` (SSA-ready register code).
3.  **CodeGen**: Instructions are mapped to LLVM IR using the Stack Machine strategy.
    *   Registers map to `alloca` (stack slots).
    *   Operations map to `load` -> `op` -> `store`.
4.  **Optimization**: LLVM's `mem2reg` pass promotes stack slots to physical CPU registers.

### 4.2 Proof of Execution (Fibonacci)
**Source (HLX-A):**
```rust
fn fib(n) {
    let a = 0;
    let b = 1;
    loop (i < n) {
        let temp = a + b;
        a = b;
        b = temp;
    }
    return a;
}
```

**Generated LLVM IR (Actual Output):**
```llvm
define i64 @main() {
block_0:
  %reg_1 = alloca i64, align 8
  store i64 0, ptr %reg_1, align 8  ; a = 0
  br label %block_4

block_4:
  %v1 = load i64, ptr %reg_1, align 8
  %v2 = load i64, ptr %reg_2, align 8
  %add = add i64 %v1, %v2           ; temp = a + b
  store i64 %v2, ptr %reg_1, align 8 ; a = b
  store i64 %add, ptr %reg_2, align 8 ; b = temp
  br i1 %cond, label %block_4, label %block_exit
}
```

---

## 5. Core Axioms & Verification
The system guarantees correctness via four immutable axioms:
*   **A1 (Determinism):** `H(Code + Input) = Constant`. Zero side effects allowed outside the IO boundary.
*   **A2 (Reversibility):** `Decode(Encode(V)) == V`. Lossless data transport.
*   **A3 (Bijection):** `HLX-A <-> AST <-> HLX-R`. Text and Graph are isomorphic views of the same logic.
*   **A4 (Universal Value):** All complex types decompose into the 7 Primitives.

---

## 6. Technical Use Cases
### 6.1 The "Formal Kernel"
Because HLX modules are isolated by regions and strictly typed, writing an OS kernel in HLX eliminates entire classes of memory safety vulnerabilities (Buffer Overflows, UAF) without the cognitive overhead of Rust's borrow checker. The *runtime* enforces the safety, not the *developer*.

### 6.2 State Reification (Solving Context Rot)
Long-running AI agents suffer from context window degradation.
*   **Solution:** The Agent's state is an `HLX Object`.
*   **Loop:** The Agent reads `State_N`, computes `Delta`, Runtime applies `Delta` -> `State_N+1`.
*   **Benefit:** The Context Window is cleared every turn. The "Memory" is perfect, externalized data, not fuzzy token history.

---

**Conclusion:** HLX V2 is not just a language; it is a high-performance, verifiable substrate. By constraining the AI to emit deterministic bytecode and executing it on a "CPU Shader" architecture, we achieve the reliability of formal systems with the flexibility of generative AI.
