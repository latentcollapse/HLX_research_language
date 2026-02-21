# HLX Runtime Specification
## Minimal substrate for HLX self-hosting

HLX needs to be independent of RustD. This spec defines the minimal runtime HLX requires.

---

## 1. Primitive Types

| Type | Size | Description |
|------|------|-------------|
| `i64` | 8 bytes | Signed integer |
| `f64` | 8 bytes | Float (NaN/Inf trapped in safe mode) |
| `bool` | 1 byte | Boolean |
| `String` | ptr + len | Immutable string |
| `[T]` | ptr + len | Dynamic array |
| `Tensor` | ptr + shape | N-dimensional array |

---

## 2. Required Builtins

### String Operations
```
strlen(s: String) -> i64
substring(s: String, start: i64, length: i64) -> String
concat(a: String, b: String) -> String
strcmp(a: String, b: String) -> i64  // -1, 0, 1
ord(s: String) -> i64                 // First char code
char(code: i64) -> String             // Single char string
```

### Array Operations
```
push(arr: [T], elem: T) -> [T]
get_at(arr: [T], idx: i64) -> T
set_at(arr: [T], idx: i64, val: T) -> [T]
array_len(arr: [T]) -> i64
```

### I/O
```
print(s: String) -> void
print_int(n: i64) -> void
read_line() -> String
```

### Control Flow
```
loop(condition: bool, max_iter: i64) -> void
barrier(name: String) -> void
```

### Memory
```
alloc(size: i64) -> ptr
free(ptr) -> void
```

---

## 3. Bytecode Format (LC-B)

HLX compiles to LC-B (Low-level Computational Bytecode):

```
Header (32 bytes):
  magic: [u8; 4]     // "LCB1"
  version: u32
  flags: u32
  entry: u32
  num_funcs: u32
  num_strings: u32
  num_tensors: u32
  checksum: [u8; 8]  // BLAKE3 hash

Function:
  name_offset: u32
  num_params: u32
  num_regs: u32
  code_offset: u32
  code_len: u32

Instruction (variable):
  opcode: u8
  operands: [u8; 3] or [u32; 2]
```

---

## 4. Instruction Set

### Value Operations
```
CONST    out, value     // Load constant
MOVE     out, src       // Copy register
LOAD     out, addr      // Load from memory
STORE    addr, val      // Store to memory
```

### Arithmetic
```
ADD      out, a, b
SUB      out, a, b
MUL      out, a, b
DIV      out, a, b      // Traps on div by zero
MOD      out, a, b
NEG      out, a
```

### Control Flow
```
JUMP     target
JUMP_IF  cond, target
CALL     func, args...
RETURN   value
HALT     condition, max_steps
```

### Array/String
```
PUSH     arr, elem
GET      arr, idx, out
SET      arr, idx, val
LEN      arr, out
SUBSTR   str, start, len, out
CONCAT   a, b, out
```

### Recursive Intelligence
```
AGENT_SPAWN   name, latent_count
CYCLE_BEGIN   level, count
CYCLE_END     level
LATENT_GET    name, out
LATENT_SET    name, val
BARRIER_SYNC  id, consensus_type
GOVERN_CHECK  effect, conscience[]
```

---

## 5. Implementation Options

### Option A: Minimal C Runtime
```c
// hlx_runtime.c
#include <stdint.h>
#include <string.h>

// String ops
int64_t hlx_strlen(const char* s) { return strlen(s); }
char* hlx_substring(const char* s, int64_t start, int64_t len);
// ... etc
```

Pros: Ubiquitous, no dependencies
Cons: Manual memory management

### Option B: Minimal Rust Runtime
```rust
// hlx_runtime/src/lib.rs
pub fn hlx_strlen(s: &str) -> i64 { s.len() as i64 }
pub fn hlx_substring(s: &str, start: i64, len: i64) -> &str;
// ... etc
```

Pros: Safe, easy FFI
Cons: Rust dependency

### Option C: Self-Hosting
```hlx
// runtime.hlx - eventually implements itself
fn strlen(s: String) -> i64 {
    let len = 0;
    loop(ord(substring(s, len, 1)) != 0, 10000) {
        len = len + 1;
    }
    return len;
}
```

Pros: Pure HLX, maximum portability
Cons: Bootstrap complexity

---

## 6. Independence Path

```
Stage 0 (NOW):
  RustD runs HLX bootstrap ✓

Stage 1:
  Create hlx-runtime crate (minimal Rust)
  HLX -> hlx-runtime

Stage 2:
  hlx-runtime written in C
  HLX -> C runtime

Stage 3:
  HLX runtime written in HLX
  HLX -> HLX
```

---

## 7. Immediate Action Items

1. **Extract builtins** from RustD into `hlx-runtime/` crate
2. **Define LC-B formally** in a spec document
3. **Build HLX bytecode emitter** in the bootstrap
4. **Create minimal interpreter** that runs LC-B

---

## 8. Separation of Concerns

| Project | What It Is | Dependency |
|---------|-----------|------------|
| **RustD** | Rust library for determinism | None (pure Rust) |
| **Axiom** | Policy engine | None (can be HLX library later) |
| **HLX** | Language for recursive AI | Minimal runtime |

HLX is the ONLY language. Everything else enhances existing ecosystems.
