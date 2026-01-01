# HLX Language Family Specification

## Overview

HLX is a deterministic programming language ecosystem with multiple syntactic forms:

- **HLXL** - Human-readable ASCII form (high-level language)
- **HLX** - Compact runic/glyph form (efficient serialization)
- **LC-B** - Binary wire format (deterministic encoding)
- **LC-R** - Runic wire format (compact text representation)
- **LC-T** - Pedagogical text format (learning/debugging)

All forms are **bijective** (1:1 correspondence) and **deterministic** (reproducible across all hardware).

---

## Quick Reference

### Language Hierarchy

```
HLXL (ASCII)          ← Human-readable, easy to learn
    ↓
HLX (Runic)           ← Compact, efficient
    ↓
LC-B (Binary)         ← Wire format, deterministic
LC-R (Runic)          ← Runic wire format
LC-T (Text)           ← Pedagogical format
```

### Core Concepts

| Concept | Example | Purpose |
|---------|---------|---------|
| **Literal** | `42`, `3.14`, `"hello"` | Constant values |
| **Variable** | `x`, `name` | Named storage |
| **Binding** | `let x = 10` | Create and assign |
| **Array** | `[1, 2, 3]` | Ordered collection |
| **Object** | `{a: 1, b: 2}` | Key-value mapping |
| **Contract** | `@14 {field: value}` | Type-tagged structure |
| **Handle** | `&h_sha256_hash` | Content-addressed reference |

---

## The Four Language Forms

### 1. HLXL - Human Readable (ASCII)

**Best for:** Learning, development, readability

```hlxl
let x = 42
let name = "AMD"
let arr = [1, 2, 3]
let obj = {name: "HLX", version: "1.1.0"}
print(name)
let result = x + 10
```

**Key features:**
- Familiar syntax (similar to Python/JavaScript)
- Full Unicode support
- Comments not supported (for determinism)
- Clear operator precedence

**See:** [HLXL_SPECIFICATION.md](HLXL_SPECIFICATION.md)

---

### 2. HLX - Compact Runic

**Best for:** Efficient storage, wire transmission

```
ⓗ_42
ⓗ"AMD"
ⓗ[1 2 3]
ⓗ{name:"HLX" ver:"1.1"}
```

**Key features:**
- Compact Unicode glyph encoding
- Same semantics as HLXL
- 30-40% smaller than ASCII
- Deterministic glyph mapping

**See:** [HLX_SPECIFICATION.md](HLX_SPECIFICATION.md)

---

### 3. LC-B - Binary Wire Format

**Best for:** Network transmission, performance

```
[MAGIC] [TYPE] [LENGTH] [DATA] [SHA256 SIGNATURE]
```

**Key features:**
- LEB128 variable-length integers
- Deterministic field ordering
- SHA-256 signatures for integrity
- Zero-copy deserialization

**See:** [WIRE_FORMATS.md](WIRE_FORMATS.md#lc-b-binary-format)

---

### 4. LC-R & LC-T - Runic & Pedagogical Formats

**Best for:** Debugging, learning, inspection

```
LC-R: ⓡ123 ⓡ"text" ⓡ[1 2 3]
LC-T: [INT(123)] [STR("text")] [ARRAY(1 2 3)]
```

**Key features:**
- Human-inspectable
- Full type information visible
- Useful for debugging contracts
- Not for production serialization

**See:** [WIRE_FORMATS.md](WIRE_FORMATS.md)

---

## Type System

HLX supports 7 fundamental types:

| Type | Examples | Notes |
|------|----------|-------|
| **Null** | `null` | Represents absence |
| **Boolean** | `true`, `false` | Logical values |
| **Integer** | `-17`, `0`, `42`, `2^62` | 64-bit signed |
| **Float** | `3.14`, `-0.5`, `1e-6` | IEEE 754 (no NaN/Inf) |
| **String** | `"hello"`, `"café"` | UTF-8, NFC normalized |
| **Array** | `[1, 2, 3]` | Heterogeneous, ordered |
| **Object** | `{key: value}` | Key-value mapping, sorted keys |

**Special type:** **Contract** - Tagged value with schema (see CONTRACT_SYSTEM.md)

---

## Determinism Guarantees

HLX provides **4 core axioms**:

### A1: Determinism
```
encode(value) = encode(value)   // Always identical output
```
Same input produces byte-identical output across all runs and hardware.

### A2: Reversibility
```
decode(encode(value)) = value   // Perfect round-trip
```
No information loss during encoding/decoding cycle.

### A3: Bijection
```
encode: Value → Binary (1:1 correspondence)
decode: Binary → Value (1:1 correspondence)
```
Every value has exactly one encoding, every encoding has exactly one value.

### A4: Universal Value
```
All types lower to HLX-Lite (minimal subset)
```
Complex types reduce to fundamental types without special cases.

---

## Quick Start Examples

### Example 1: Basic Arithmetic (HLXL)

```hlxl
let a = 10
let b = 20
let sum = a + b
let product = a * b

print(sum)
print(product)
```

Output: `30`, `200`

---

### Example 2: Data Structure (HLXL)

```hlxl
let person = {
  name: "Alice",
  age: 30,
  skills: ["Rust", "Vulkan", "HLX"]
}

print(person)
```

Output: `{age: 30, name: "Alice", skills: ["Rust", "Vulkan", "HLX"]}`

*(Note: Keys automatically sorted)*

---

### Example 3: Contract with Schema (HLXL)

```hlxl
let user_contract = @14 {
  @0: "alice",
  @1: 30,
  @2: true
}
```

Contract 14 defines: field @0 is string, @1 is int, @2 is bool.

---

## Language Forms Comparison

| Feature | HLXL | HLX | LC-B | LC-T |
|---------|------|-----|------|------|
| **Readability** | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐ | ⭐⭐⭐⭐ |
| **Compactness** | ⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| **Production** | ✓ | ✓ | ✓ | ✗ |
| **Learning** | ✓ | ~ | ✗ | ✓ |
| **Wire Format** | ✗ | ~ | ✓ | ✗ |
| **Deterministic** | ✓ | ✓ | ✓ | ✓ |

---

## Operators & Built-ins

### Arithmetic

```hlxl
let sum = 10 + 5        // 15
let diff = 10 - 5       // 5
let prod = 10 * 5       // 50
let div = 10 / 5        // 2.0
```

### Comparison

```hlxl
let eq = 10 == 10       // true
let ne = 10 != 5        // true
let lt = 5 < 10         // true
let gt = 10 > 5         // true
let le = 5 <= 10        // true
let ge = 10 >= 5        // true
```

### Logical

```hlxl
let and_result = true and false      // false
let or_result = true or false        // true
let not_result = not true            // false
```

### String Operations

```hlxl
let greeting = "Hello" + " " + "World"  // "Hello World"
```

### Array Operations

```hlxl
let arr = [1, 2, 3]
let first = arr[0]      // 1
let length = arr        // Full array returned
```

### Built-in Functions

```hlxl
print(value)            // Output value
type(value)             // Get type name (returns string)
```

---

## Latent Space Operations (LS-only runtimes)

If using HLXL-LS or HLX-LS runtime:

```hlxl
let collapsed = collapse(value)     // Collapse to latent representation
let resolved = resolve(collapsed)   // Resolve back to value
let snapshot = snapshot()           // Capture environment state
```

These enable advanced meta-programming. See runtimes documentation.

---

## Content-Addressed Storage (CAS)

HLX supports content-addressed storage:

```hlxl
let handle = store(value)           // Returns &h_sha256_hash
let retrieved = retrieve(handle)    // Get value by hash
```

**Handle format:** `&h_XXXXXXX...` (SHA-256 prefix)

---

## Error Handling

HLX provides specific error codes for determinism violations:

| Error | Meaning |
|-------|---------|
| `E_FLOAT_SPECIAL` | NaN or Infinity encountered |
| `E_DEPTH_EXCEEDED` | Nesting too deep (>64 levels) |
| `E_FIELD_ORDER` | Object keys not sorted |
| `E_HANDLE_NOT_FOUND` | CAS handle doesn't exist |
| `E_LC_PARSE` | Binary format invalid |
| `E_PARSE_ERROR` | Syntax error |

All errors are deterministic (same input always fails same way).

---

## Constraints & Limitations

### Numeric Constraints
- **Integers:** -2^63 to 2^63-1 (64-bit signed)
- **Floats:** IEEE 754 (no NaN, Inf, -0 normalized to +0)
- **Nesting:** Max 64 levels deep (arrays/objects)

### String Constraints
- **Encoding:** UTF-8
- **Normalization:** NFC (composed form)
- **Whitespace:** Trailing whitespace trimmed for determinism

### Key Constraints
- **Sorting:** Object keys always sorted lexicographically
- **Uniqueness:** Duplicate keys not allowed

---

## Inter-Language Conversion

### HLXL → HLX
```python
from hlx_runtime.hlxl_runtime import HLXLRuntime
from hlx_runtime.lc_codec import encode_lcb

runtime = HLXLRuntime()
value = runtime.execute('let x = [1, 2, 3]')
hlx_bytes = encode_lcb(value)
```

### HLX → HLXL
```python
from hlx_runtime.lc_codec import decode_lcb
from hlx_runtime.hlxl_runtime import value_to_hlxl

bytes_data = b'...'
value = decode_lcb(bytes_data)
hlxl_text = value_to_hlxl(value)
```

### All formats ↔ LC-B (canonical)
All formats convert through LC-B:
```
HLXL → LC-B ← HLX
       ↓
LC-R (runic view)
LC-T (pedagogical view)
```

---

## Next Steps

1. **Learning HLXL:** See [HLXL_SPECIFICATION.md](HLXL_SPECIFICATION.md)
2. **Learning HLX:** See [HLX_SPECIFICATION.md](HLX_SPECIFICATION.md)
3. **Wire formats:** See [WIRE_FORMATS.md](WIRE_FORMATS.md)
4. **Determinism:** See [CONTRACT_SYSTEM.md](CONTRACT_SYSTEM.md)
5. **Type details:** See [TYPE_SYSTEM.md](TYPE_SYSTEM.md)

---

## Key Design Philosophy

HLX is built on **5 principles**:

1. **Determinism First** - No non-deterministic operations
2. **Correctness over Speed** - Better to be right than fast
3. **Portability** - Works identically across all hardware
4. **Simplicity** - Minimal language, maximum expressiveness
5. **Auditability** - All operations verifiable and reproducible

These principles enable:
- Cryptographic verification of computation
- Hardware-independent reproducibility
- Bit-exact determinism across platforms
- Confidence in scientific results
- Production ML training without surprises

---

**Status:** Production-ready v1.1.0
**Repository:** https://github.com/latentcollapse/hlx-compiler
**Runtimes:** 4 complete implementations (HLXL, HLXL-LS, HLX, HLX-LS)
**Tests:** 433 passing across all components
