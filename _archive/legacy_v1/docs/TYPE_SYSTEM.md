# Type System Specification

## Overview

HLX has a **simple but powerful type system** with 7 fundamental types:

| Type | Description | Examples | Size |
|------|-------------|----------|------|
| **Null** | Absence of value | `null` | 1 byte |
| **Boolean** | Logical value | `true`, `false` | 1 byte |
| **Integer** | 64-bit signed | `-17`, `0`, `42`, `2^62` | 8 bytes |
| **Float** | IEEE 754 double | `3.14`, `-0.5`, `1e-6` | 8 bytes |
| **String** | UTF-8 text | `"hello"`, `"café"` | Variable |
| **Array** | Ordered collection | `[1, "two", 3.0]` | Variable |
| **Object** | Key-value mapping | `{a: 1, b: 2}` | Variable |

Plus special type: **Contract** (tagged value with schema)

---

## Type Semantics

### Null

**Represents:** Absence of value, no value, nothing

```hlxl
let x = null
let empty = null

// Check if null
type(null) = "null"
```

**Semantics:**
- Unique value (only one null)
- Cannot be operated on
- Type is "null"

### Boolean

**Represents:** Logical true or false

```hlxl
let flag = true
let inactive = false

// Results of comparisons
let result = 10 > 5    // true
```

**Semantics:**
- Two distinct values
- Result of logical operations
- Type is "boolean"

### Integer

**Represents:** Exact whole numbers

```hlxl
let count = 42
let negative = -17
let zero = 0
let large = 9223372036854775807  // 2^63 - 1
let small = -9223372036854775808 // -2^63
```

**Range:** -2^63 to 2^63-1 (64-bit signed)

**Semantics:**
- Exact representation
- Supports arithmetic operations
- Type is "integer"
- No underflow/overflow checking (undefined at bounds)

**Examples:**
```hlxl
let x = 10 + 5           // 15 (integer)
let y = x * 2            // 30 (integer)
let z = 10 / 3           // 3.333... (float!) - always returns float
```

### Float

**Represents:** Approximate real numbers

```hlxl
let pi = 3.14159
let ratio = 1.5
let tiny = 1e-10
let large = 1e20
```

**Format:** IEEE 754 double precision

**Constraints:**
- **NaN NOT allowed** - Error: `E_FLOAT_SPECIAL`
- **Infinity NOT allowed** - Error: `E_FLOAT_SPECIAL`
- **-0.0 normalized to +0.0** - Determinism
- **Decimal point required** - Write `3.0` not `3`

**Semantics:**
- Approximate representation
- Precision: ~15-17 significant digits
- Type is "float"

**Examples:**
```hlxl
let a = 3.14            // Valid
let b = 0.0             // Valid
let c = -0.0            // Normalized to 0.0
let d = 1.0e-6          // Valid (scientific notation)

let invalid1 = 0/0      // NaN → Error!
let invalid2 = 1/0      // Infinity → Error!
```

### String

**Represents:** Ordered sequence of Unicode characters

```hlxl
let greeting = "Hello"
let empty = ""
let with_newline = "Line 1\nLine 2"
let unicode = "café"
let emoji = "🚀"
let japanese = "日本語"
```

**Encoding:** UTF-8

**Normalization:** NFC (composed form)

**Escape sequences:**
- `\n` - Newline
- `\t` - Tab
- `\"` - Double quote
- `\\` - Backslash
- (Others not supported)

**Semantics:**
- Immutable after creation
- Ordered sequence of characters
- Type is "string"
- Compared lexicographically

**Examples:**
```hlxl
let s1 = "hello"
let s2 = "hello"
s1 == s2                // true

let s3 = "Hello"
s1 == s3                // false (case-sensitive)

let multi = "Line 1\nLine 2"
type(multi) = "string"
```

### Array

**Represents:** Ordered, heterogeneous collection

```hlxl
let empty = []
let integers = [1, 2, 3, 4, 5]
let mixed = [1, "two", 3.0, true, null]
let nested = [[1, 2], [3, 4]]
let of_objects = [{x: 1}, {x: 2}]
```

**Features:**
- **Heterogeneous** - Any types mixed
- **Ordered** - Preserves sequence
- **Indexable** - Access by position

**Indexing:**
```hlxl
let arr = [10, 20, 30]
arr[0]                   // 10 (first element)
arr[1]                   // 20 (second element)
arr[2]                   // 30 (third element)
arr[3]                   // Error: out of bounds
arr[-1]                  // Error: negative index invalid
```

**Semantics:**
- Zero-indexed (first element at 0)
- Type is "array"
- Arrays are mutable (can reassign)

**Examples:**
```hlxl
let data = [1, 2, 3]
type(data) = "array"

let nested = [[1, 2], [3, 4]]
nested[0] = [1, 2]      // First nested array
```

### Object

**Represents:** Key-value mapping (dictionary, record, struct)

```hlxl
let empty = {}
let person = {name: "Alice", age: 30}
let mixed = {x: 1, label: "point", active: true}
let nested = {user: {id: 1, name: "Bob"}}
```

**Features:**
- **Key-value pairs**
- **Unordered conceptually** (but always sorted)
- **Accessible by key** - `obj[key]` or `obj.key`

**Field Access:**
```hlxl
let person = {name: "Alice", age: 30, city: "NYC"}
person.name              // "Alice"
person.age               // 30
person.city              // "NYC"
person.unknown           // Error: key doesn't exist
```

**Key Constraints:**
- Keys must be identifiers (alphanumeric + underscore)
- Keys are **case-sensitive**
- Keys are **always sorted** (determinism)
- No **duplicate keys**

**Semantics:**
- Keys sorted lexicographically
- Type is "object"
- Values can be any type

**Examples:**
```hlxl
let input = {z: 1, a: 2, m: 3}
// Automatically becomes: {a: 2, m: 3, z: 1}

let nested = {user: {id: 1, name: "Bob"}}
nested.user             // {id: 1, name: "Bob"}
nested.user.name        // "Bob"
```

### Contract

**Represents:** Type-tagged value with schema validation

```hlxl
let user = @14 {
    @0: "Alice",        // Field 0: string
    @1: 30,             // Field 1: integer
    @2: true            // Field 2: boolean
}
```

**Syntax:** `@CONTRACT_ID { @FIELD: value, ... }`

**Semantics:**
- Tagged with contract ID
- Fields must match schema
- Deterministically validated
- Type is "contract"

**See:** [CONTRACT_SYSTEM.md](CONTRACT_SYSTEM.md) for details.

---

## Type Operations

### Type Checking

```hlxl
type(42)                // "integer"
type(3.14)              // "float"
type("hello")           // "string"
type([1, 2, 3])         // "array"
type({a: 1})            // "object"
type(true)              // "boolean"
type(null)              // "null"
type(@14{...})          // "contract"
```

### Type Compatibility

HLX has **NO automatic type coercion**.

```hlxl
10 + 5                  // 15 (int + int = int)
10 + 5.0                // 15.0 (int + float = float)
"5" + 3                 // Error! (string + int = error)
[1] + [2]               // Error! (array + array = error)
```

**Rule:** Operations only work on compatible types.

### Comparison Type Rules

```hlxl
10 == 10                // true (int == int)
10 == 10.0              // false (int ≠ float, different types)
"10" == 10              // false (string ≠ int)
null == null            // true (null == null)
true == 1               // false (bool ≠ int)
[1] == [1]              // true (arrays compared by value)
{a:1} == {a:1}          // true (objects compared by value)
```

**Rule:** Different types are not equal (no coercion).

---

## Type Predicates

### Checking Types

```python
from hlx_runtime.type_system import get_type, is_null, is_bool, is_int, is_float, is_string, is_array, is_object, is_contract

value = 42
print(get_type(value))      # "integer"
print(is_int(value))        # True
print(is_string(value))     # False

value2 = "hello"
print(is_string(value2))    # True
print(is_array(value2))     # False
```

---

## Type Equivalence

### Structural Equality

Types are equivalent if they have the same structure:

```hlxl
{a: 1, b: 2} == {a: 1, b: 2}       // true (same structure)
{a: 1, b: 2} == {b: 2, a: 1}       // true (order irrelevant, sorted)
{a: 1, b: 2} == {a: 1, b: 3}       // false (different values)

[1, 2, 3] == [1, 2, 3]              // true (same elements)
[1, 2, 3] == [1, 3, 2]              // false (different order)
```

### Identity vs Equality

HLX doesn't distinguish (no reference identity):

```hlxl
let x = {a: 1}
let y = {a: 1}
x == y                  // true (equal by value)

// There is no concept of "same object"
// All values are compared structurally
```

---

## Type Preservation

### Through Operations

Types are preserved through arithmetic:

```hlxl
let a = 10              // integer
let b = a + 5           // integer + integer = integer
let c = b * 2           // integer * integer = integer

let x = 3.14            // float
let y = x + 1           // float + integer = float
let z = y * 2           // float * integer = float
```

### Through Collections

Collections preserve element types:

```hlxl
let ints = [1, 2, 3]              // array of mixed types
let mixed = [1, "two", 3.0]       // array of mixed types
let objs = [{a: 1}, {a: 2}]       // array of objects

// Types of elements preserved
ints[0] = 1             // integer
mixed[1] = "two"        // string
objs[0] = {a: 1}        // object
```

---

## Null Safety

### Null Propagation

Operations on null typically error:

```hlxl
null + 5                // Error: E_TYPE_ERROR
null == 5               // false (different types, no error)
type(null)              // "null" (type check OK)
```

### Null Checks

```hlxl
let x = null
let safe = x == null    // true
let type_name = type(x) // "null"
```

---

## Type Constraints

### Numeric Constraints

**Integers:**
- Min: -2^63 = -9,223,372,036,854,775,808
- Max: 2^63-1 = 9,223,372,036,854,775,807
- No overflow checking (UB at bounds)

**Floats:**
- Min: ±2.2250738585072014e-308
- Max: ±1.7976931348623157e+308
- Precision: ~15-17 significant digits
- NaN/Inf not allowed

### String Constraints

- **Max length:** Limited by memory
- **Encoding:** UTF-8
- **Normalization:** NFC (composed)
- **No null bytes:** UTF-8 encoded

### Collection Constraints

**Arrays:**
- **Max depth:** 64 levels (nesting)
- **Max elements:** Limited by memory
- **Heterogeneous:** Any types mixed

**Objects:**
- **Max depth:** 64 levels (nesting)
- **Max fields:** 256 (field indices @0-@255)
- **Key format:** Identifier (alphanumeric + underscore)
- **Key uniqueness:** No duplicates
- **Key ordering:** Always lexicographic

---

## Type Conversion Functions

### Converting Between Types

No automatic coercion, but you can manually convert:

```hlxl
// String to Integer: use parsing (not built-in)
// Would require string parsing logic

// Integer to String: create string
let str = "value"  // Manual

// Float to Integer: truncation (not built-in)
// Would require custom function
```

**Best practice:** Use contracts for explicit typed data structures.

---

## Error Handling by Type

### Type-Related Errors

| Error | Cause | Occurs |
|-------|-------|--------|
| `E_TYPE_ERROR` | Incompatible operation | `null + 5` |
| `E_FLOAT_SPECIAL` | NaN or Infinity | `0/0`, `1/0` |
| `E_DEPTH_EXCEEDED` | >64 levels nested | Deep nesting |
| `E_FIELD_ORDER` | Object keys not sorted | (auto-corrected) |

---

## Type System Guarantees

### Guarantee 1: Deterministic Type Checking

Same value always has same type:
```
type(value1) = type(value2)  if value1 = value2
```

### Guarantee 2: No Silent Coercion

Type errors are explicit, never silent:
```
"5" + 3  // Error (not 8)
10 == "10"  // false (not true)
```

### Guarantee 3: Type Preservation

Operations preserve types:
```
int + int = int
float + float = float
int + float = float
```

### Guarantee 4: Structural Equality

Equality depends only on structure:
```
{a: 1, b: 2} == {b: 2, a: 1}  // true (same structure)
```

---

## Best Practices

### ✓ DO

1. **Be explicit about types:**
```hlxl
let age = 30            // Integer
let height = 1.8        // Float
let name = "Alice"      // String
```

2. **Use contracts for typed structures:**
```hlxl
let person = @14 {
    @0: "Alice",
    @1: 30,
    @2: true
}
```

3. **Check types before operations:**
```hlxl
let x = type_check_input()
if type(x) == "integer" {
    // Use x as integer
}
```

4. **Document type expectations:**
```hlxl
// Returns: {users: [user_contracts], count: int}
let result = get_users()
```

### ✗ DON'T

1. **Assume type coercion:**
```hlxl
let bad = "5" + 3   // Error! Don't expect "53" or 8
```

2. **Mix types in comparisons without care:**
```hlxl
if x == "5" {}  // Only true if x is exactly "5"
```

3. **Rely on undefined behavior:**
```hlxl
let huge = 9223372036854775807 + 1  // Undefined (overflow)
```

---

## Implementation Notes

### For Runtime Implementers

1. **Type representation:** Use discriminated union or tagged value
2. **String handling:** UTF-8 + NFC normalization
3. **Number handling:** 64-bit int, IEEE 754 float
4. **Collection handling:** Dynamic arrays, hash maps
5. **Validation:** Strict type checking at boundaries

### For Compiler Implementers

1. **Type inference:** Can infer from literals
2. **Type checking:** Report errors early
3. **Code generation:** Preserve type information
4. **Optimization:** Use type info for better codegen

---

## Next Steps

- **Learning type usage:** See [HLXL_SPECIFICATION.md](HLXL_SPECIFICATION.md)
- **Contract system:** See [CONTRACT_SYSTEM.md](CONTRACT_SYSTEM.md)
- **Wire formats:** See [WIRE_FORMATS.md](WIRE_FORMATS.md)
- **Examples:** See [../examples/](../examples/)

---

**Version:** 1.1.0
**Status:** Production-ready
**Type system:** Simple, safe, deterministic
