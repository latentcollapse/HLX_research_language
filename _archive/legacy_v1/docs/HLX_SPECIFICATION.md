# HLX Specification - Runic Compact Language

## Overview

**HLX** (the glyph form) is a compact representation of the HLX language using Unicode glyphs. It's designed for:
- **Efficient encoding** - 30-40% more compact than HLXL
- **Bandwidth efficiency** - Smaller wire transmissions
- **Aesthetic value** - Mathematically beautiful glyphs
- **Deterministic mapping** - 1:1 bijection with HLXL

All HLX code is semantically identical to HLXL - only the syntax differs.

---

## Glyph Reference

### Literals

| Type | HLXL | HLX | Notes |
|------|------|-----|-------|
| **Null** | `null` | `ⓝ` | Circle-N |
| **True** | `true` | `ⓣ` | Circle-T |
| **False** | `false` | `ⓕ` | Circle-F |

### Type Indicators

| Form | Glyph | Used In |
|------|-------|---------|
| **Integer** | `ⓘ` | `ⓘ42` |
| **Float** | `ⓡ` | `ⓡ3.14` |
| **String** | `ⓢ` | `ⓢ"hello"` |
| **Array** | `ⓐ` | `ⓐ[1 2 3]` |
| **Object** | `ⓞ` | `ⓞ{a:1 b:2}` |
| **Contract** | `ⓒ` | `ⓒ14{@0:42}` |

### Operators

| Operation | HLXL | HLX | Notes |
|-----------|------|-----|-------|
| **Add** | `+` | `⊕` | Circled plus |
| **Subtract** | `-` | `⊖` | Circled minus |
| **Multiply** | `*` | `⊗` | Circled times |
| **Divide** | `/` | `⊘` | Circled divide |
| **Equal** | `==` | `⩵` | Equals with two lines |
| **Not-equal** | `!=` | `≠` | Not-equal sign |
| **Less-than** | `<` | `≪` | Double less-than |
| **Greater-than** | `>` | `≫` | Double greater-than |
| **And** | `and` | `∧` | Logical and |
| **Or** | `or` | `∨` | Logical or |
| **Not** | `not` | `¬` | Logical not |

### Special Symbols

| Symbol | Meaning | Example |
|--------|---------|---------|
| `✦` | Bind (let) | `✦x=42` |
| `⟨` `⟩` | Parentheses | `⟨42⊕5⟩` |
| `⌊` `⌋` | Array brackets | `⌊1 2 3⌋` |
| `❴` `❵` | Object braces | `❴a:1 b:2❵` |

---

## Syntax

### Basic Literals

```hlx
ⓝ                          // null
ⓣ                          // true
ⓕ                          // false
ⓘ42                        // Integer 42
ⓘ-17                       // Integer -17
ⓡ3.14                      // Float 3.14
ⓡ-0.5                      // Float -0.5
ⓢ"hello"                   // String "hello"
ⓢ"café"                    // String with Unicode
```

### Arrays

```hlx
⌊⌋                          // Empty array: []
⌊ⓘ1 ⓘ2 ⓘ3⌋               // Array: [1, 2, 3]
⌊ⓘ1 ⓢ"two" ⓡ3.0⌋         // Mixed: [1, "two", 3.0]
⌊⌊ⓘ1 ⓘ2⌋ ⌊ⓘ3 ⓘ4⌋⌋      // Nested: [[1,2], [3,4]]
```

**Syntax:**
```
⌊ element element ... element ⌋
```

### Objects

```hlx
❴❵                         // Empty: {}
❴a:ⓘ1❵                     // Single field: {a: 1}
❴a:ⓘ1 b:ⓘ2 c:ⓘ3❵         // Multiple: {a:1, b:2, c:3}
❴name:ⓢ"Alice" age:ⓘ30❵  // Mixed types: {name: "Alice", age: 30}
```

**Syntax:**
```
❴ key:value key:value ... key:value ❵
```

**Notes:**
- Keys are identifiers (no quotes)
- Fields separated by whitespace
- Keys automatically sorted

### Binding (let)

```hlx
✦x=ⓘ42                     // let x = 42
✦name=ⓢ"Alice"            // let name = "Alice"
✦arr=⌊ⓘ1 ⓘ2 ⓘ3⌋          // let arr = [1, 2, 3]
```

**Syntax:**
```
✦ identifier = expression
```

### Operators

```hlx
ⓘ10⊕ⓘ5                     // 10 + 5
ⓘ10⊖ⓘ5                     // 10 - 5
ⓘ10⊗ⓘ5                     // 10 * 5
ⓘ10⊘ⓘ5                     // 10 / 5

ⓘ10⩵ⓘ10                    // 10 == 10
ⓘ10≠ⓘ5                     // 10 != 5
ⓘ10≪ⓘ5                     // 10 < 5 (false)
ⓘ10≫ⓘ5                     // 10 > 5 (true)

ⓣ∧ⓕ                        // true and false
ⓣ∨ⓕ                        // true or false
¬ⓣ                         // not true
```

### Complex Example

```hlx
✦principal=ⓘ1000
✦rate=ⓡ0.05
✦time=ⓘ3
✦interest=principal⊗rate⊗time
✦total=principal⊕interest
```

Equivalent HLXL:
```hlxl
let principal = 1000
let rate = 0.05
let time = 3
let interest = principal * rate * time
let total = principal + interest
```

---

## Whitespace Handling

In HLX, whitespace is significant for token separation but insignificant otherwise:

```hlx
✦x=ⓘ10                    // Valid
✦x = ⓘ 10                 // Valid (extra spaces OK)
✦x=ⓘ10⊕ⓘ5                // Valid
✦x=ⓘ10 ⊕ ⓘ5              // Valid
```

**Rule:** One or more whitespace characters separates tokens.

---

## Operator Precedence (Same as HLXL)

From highest to lowest:

| Level | Operators | Associativity |
|-------|-----------|---------------|
| 1 | Array indexing | Left |
| 2 | `⊗`, `⊘` | Left |
| 3 | `⊕`, `⊖` | Left |
| 4 | `⩵`, `≠`, `≪`, `≫`, `≤`, `≥` | Left |
| 5 | `∧` | Left |
| 6 | `∨` | Left |
| 7 | `¬` | Right |

**Examples:**
```hlx
ⓘ1⊕ⓘ2⊗ⓘ3                // 7 (multiply first)
ⓣ∨ⓕ∧ⓕ                    // true (and binds tighter)
¬ⓕ∨ⓣ                      // true (not binds tightest)
```

---

## Contracts

```hlx
ⓒ14❴@0:ⓢ"alice" @1:ⓘ30 @2:ⓣ❵
```

Equivalent HLXL:
```hlxl
@14 {
    @0: "alice",
    @1: 30,
    @2: true
}
```

**Syntax:**
```
ⓒ contract_id ❴ @field:value @field:value ... ❵
```

---

## String Handling

HLX uses quoted strings just like HLXL:

```hlx
ⓢ"hello"
ⓢ"Hello\nWorld"
ⓢ"Escaped \"quotes\""
ⓢ"café"                   // UTF-8 Unicode
ⓢ"日本語"                  // Full Unicode support
```

All string rules from HLXL apply:
- UTF-8 encoding
- NFC normalization
- Escape sequences supported
- Trailing whitespace trimmed

---

## Comparison: HLXL vs HLX

### Example 1: Simple Data Structure

**HLXL (45 bytes):**
```hlxl
let person = {
  name: "Alice",
  age: 30
}
```

**HLX (28 bytes):**
```hlx
✦person=❴name:ⓢ"Alice" age:ⓘ30❵
```

**Savings: 38%**

### Example 2: Array of Objects

**HLXL (92 bytes):**
```hlxl
let users = [
  {id: 1, name: "Alice"},
  {id: 2, name: "Bob"}
]
```

**HLX (66 bytes):**
```hlx
✦users=⌊❴id:ⓘ1 name:ⓢ"Alice"❵ ❴id:ⓘ2 name:ⓢ"Bob"❵⌋
```

**Savings: 28%**

### Example 3: Mathematical Computation

**HLXL (67 bytes):**
```hlxl
let x = 10
let y = 20
let z = x + y
```

**HLX (28 bytes):**
```hlx
✦x=ⓘ10
✦y=ⓘ20
✦z=x⊕y
```

**Savings: 58%**

---

## Determinism in HLX

HLX preserves all HLXL determinism guarantees:

1. **Glyph mapping is 1:1** - Each glyph maps to exactly one HLXL token
2. **Parsing is deterministic** - Same glyph sequence always parses same way
3. **Semantics identical** - HLX and HLXL execute identically
4. **Bijection maintained** - Can convert between forms without loss

### Conversion Guarantee

```
HLX code → Parse → AST → Execute → Result
 ↓ (bijective conversion)
HLXL code → Parse → AST → Execute → Result
 (same result)
```

---

## Implementation Notes

### Parser Requirements

HLX parser must:
1. Recognize Unicode glyph characters
2. Distinguish between type indicators and standalone glyphs
3. Handle operator precedence correctly
4. Preserve whitespace rules

### Font Requirements

To display HLX properly, use a Unicode-aware font with support for:
- Circled letters (ⓐ-ⓩ)
- Mathematical operators (⊕, ⊗, ∧, ∨, ¬)
- Angle brackets (⟨⟩)
- Curved brackets (❴❵)

**Recommended fonts:**
- DejaVu Sans
- Noto Sans
- Segoe UI (Windows)
- SF Pro Display (macOS)

---

## Complete Examples

### Example 1: Arithmetic

```hlx
✦principal=ⓘ1000
✦rate=ⓡ0.05
✦time=ⓘ3
✦interest=principal⊗rate⊗time
✦total=principal⊕interest
```

### Example 2: Data Processing

```hlx
✦students=⌊❴name:ⓢ"Alice" grade:ⓘ95❵ ❴name:ⓢ"Bob" grade:ⓘ87❵⌋
✦first=students⌊ⓘ0⌋
```

### Example 3: Nested Structures

```hlx
✦database=❴
  users:⌊❴id:ⓘ1 name:ⓢ"Alice"❵ ❴id:ⓘ2 name:ⓢ"Bob"❵⌋
  config:❴host:ⓢ"localhost" port:ⓘ5432❵
❵
```

---

## Glyph Meanings (Etymology)

| Glyph | Meaning | Unicode | Name |
|-------|---------|---------|------|
| `ⓝ` | Circle-N (null) | U+24C9 | CIRCLED LATIN SMALL LETTER N |
| `ⓣ` | Circle-T (true) | U+24E3 | CIRCLED LATIN SMALL LETTER T |
| `ⓕ` | Circle-F (false) | U+24DD | CIRCLED LATIN SMALL LETTER F |
| `ⓘ` | Circle-I (integer) | U+24D8 | CIRCLED LATIN SMALL LETTER I |
| `ⓡ` | Circle-R (real/float) | U+24E1 | CIRCLED LATIN SMALL LETTER R |
| `ⓢ` | Circle-S (string) | U+24E2 | CIRCLED LATIN SMALL LETTER S |
| `ⓐ` | Circle-A (array) | U+24D0 | CIRCLED LATIN SMALL LETTER A |
| `ⓞ` | Circle-O (object) | U+24DE | CIRCLED LATIN SMALL LETTER O |
| `ⓒ` | Circle-C (contract) | U+24D2 | CIRCLED LATIN SMALL LETTER C |
| `⊕` | Circled plus (add) | U+2295 | CIRCLED PLUS |
| `⊖` | Circled minus (subtract) | U+2296 | CIRCLED MINUS |
| `⊗` | Circled times (multiply) | U+2297 | CIRCLED TIMES |
| `⊘` | Circled divide | U+2298 | CIRCLED DIVISION SLASH |
| `∧` | Logical AND | U+2227 | LOGICAL AND |
| `∨` | Logical OR | U+2228 | LOGICAL OR |
| `¬` | Logical NOT | U+00AC | NOT SIGN |

---

## Conversion Tools

### HLX to HLXL

```python
from hlx_runtime.lc_codec import hlx_to_hlxl

hlx_code = "✦x=ⓘ42"
hlxl_code = hlx_to_hlxl(hlx_code)
# Returns: "let x = 42"
```

### HLXL to HLX

```python
from hlx_runtime.lc_codec import hlxl_to_hlx

hlxl_code = "let x = 42"
hlx_code = hlxl_to_hlx(hlxl_code)
# Returns: "✦x=ⓘ42"
```

---

## Next Steps

- **Learning HLXL:** See [HLXL_SPECIFICATION.md](HLXL_SPECIFICATION.md)
- **Running HLX:** Use `HLXRuntime` from runtime module
- **Wire formats:** See [WIRE_FORMATS.md](WIRE_FORMATS.md)
- **Examples:** See [../examples/](../examples/)

---

**Version:** 1.1.0
**Status:** Production-ready
**Glyph Set:** Stable (will not change in v1.x)
