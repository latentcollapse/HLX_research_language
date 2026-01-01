# Wire Formats Specification - LC-B, LC-R, LC-T

## Overview

HLX defines three binary/text wire formats for serialization and transmission:

| Format | Name | Purpose | Use Case |
|--------|------|---------|----------|
| **LC-B** | **Binary** | Canonical deterministic format | Network, storage, hash |
| **LC-R** | **Runic** | Text-based runic format | Debugging, inspection |
| **LC-T** | **Pedagogical** | Human-readable text | Learning, education |

All formats are **bijective** (1:1 correspondence) and **deterministic** (reproducible).

---

## LC-B: Binary Format

**Purpose:** Production wire format for network transmission and storage.

**Properties:**
- ✓ Most compact
- ✓ Deterministic encoding
- ✓ Zero-copy capable
- ✓ SHA-256 signable
- ✗ Not human-readable

### Structure

```
[MAGIC] [VALUE] [SIGNATURE]

MAGIC:       1 byte  (0x7C = '|')
VALUE:       Variable length
SIGNATURE:   32 bytes (SHA-256)
```

### Type Encoding

Each value starts with a type byte:

| Type | Byte | Encoding |
|------|------|----------|
| **Null** | `0x00` | Just type byte |
| **False** | `0x01` | Just type byte |
| **True** | `0x02` | Just type byte |
| **Integer** | `0x10` | Type + LEB128 |
| **Float** | `0x20` | Type + IEEE754 (8 bytes) |
| **String** | `0x30` | Type + LEB128 length + UTF-8 data |
| **Array** | `0x40` | Type + LEB128 count + elements |
| **Object** | `0x50` | Type + LEB128 count + key-value pairs |
| **Contract** | `0x60` | Type + LEB128 ID + fields |

### LEB128 Variable-Length Integers

Encodes integers in variable-width format:

```
Value    Bytes
0-127    1 byte:  0xxxxxxx
128-16k  2 bytes: 1xxxxxxx 0xxxxxxx
16k+     3+ bytes: continued...
```

**Example:**
- `42` → `0x2A` (1 byte)
- `300` → `0xAC 0x02` (2 bytes)
- `16384` → `0x80 0x80 0x01` (3 bytes)

### String Encoding

```
[0x30] [LENGTH_LEB128] [UTF-8_DATA]
```

**Example: "hello"**
```
0x30           // String type
0x05           // Length = 5
68656C6C6F     // "hello" in UTF-8
```

### Array Encoding

```
[0x40] [COUNT_LEB128] [ELEMENT_1] [ELEMENT_2] ... [ELEMENT_N]
```

**Example: [1, "two"]**
```
0x40           // Array type
0x02           // Count = 2
0x10 0x01      // Integer 1
0x30 0x03 747 76 6F   // String "two"
```

### Object Encoding

```
[0x50] [COUNT_LEB128] [KEY_1] [VALUE_1] [KEY_2] [VALUE_2] ...
```

**Constraint:** Keys must be in **sorted lexicographic order** (determinism).

**Example: {a: 1, b: 2}**
```
0x50           // Object type
0x02           // Count = 2
[String "a"]   // Key 1
[Integer 1]    // Value 1
[String "b"]   // Key 2
[Integer 2]    // Value 2
```

### Contract Encoding

```
[0x60] [CONTRACT_ID_LEB128] [FIELD_COUNT_LEB128] [@0: value, @1: value, ...]
```

**Example: @14 {$0: "alice", @1: 30}**
```
0x60           // Contract type
0x0E           // Contract ID = 14
0x02           // Field count = 2
0x00 0x30 0x05 "alice"  // @0: string "alice"
0x01 0x10 0x1E          // @1: integer 30
```

### Complete Example: {id: 42, name: "Alice"}

**Hex dump:**
```
7C                      // MAGIC (0x7C = '|')
50                      // Object type
02                      // Field count = 2
30 02 6964             // Key: string "id" (length 2)
10 2A                   // Value: integer 42
30 04 6E616D65         // Key: string "name" (length 4)
30 05 416C696365       // Value: string "Alice" (length 5)
[32 byte SHA-256 signature]
```

### Determinism Properties

**LC-B is deterministic because:**

1. **Type byte is fixed** - Same value always encodes with same type
2. **LEB128 is deterministic** - Same integer always encodes same way
3. **String encoding fixed** - UTF-8 NFC is deterministic
4. **Field ordering** - Objects sorted by key name
5. **No padding/alignment** - Byte-by-byte deterministic
6. **IEEE 754 deterministic** - Float bits reproducible per IEEE standard

---

## LC-R: Runic Wire Format

**Purpose:** Text-based inspection and debugging.

**Properties:**
- ✓ Human-readable glyphs
- ✓ Compact text format
- ✓ Bijective with LC-B
- ✗ Not for parsing (use LC-T for that)

### Syntax

```
ⓡNULL
ⓡTRUE
ⓡFALSE
ⓡ42
ⓡ3.14
ⓡ"hello"
ⓡ[value1 value2 ...]
ⓡ{key1:value1 key2:value2 ...}
ⓡ@14{@0:value ...}
```

### Examples

```
ⓡNULL                      // null
ⓡTRUE                      // true
ⓡ42                        // integer 42
ⓡ3.14                      // float 3.14
ⓡ"hello"                   // string "hello"
ⓡ[1 2 3]                   // array [1, 2, 3]
ⓡ{a:1 b:2}                 // object {a: 1, b: 2}
ⓡ@14{@0:"alice" @1:30}    // contract @14 {...}
```

### Complex Example

```
ⓡ{
  users: ⓡ[ⓡ{id:1 name:"Alice"} ⓡ{id:2 name:"Bob"}]
  count: ⓡ2
  active: ⓡTRUE
}
```

---

## LC-T: Pedagogical Text Format

**Purpose:** Learning and explicit type visualization.

**Properties:**
- ✓ Type information explicit
- ✓ Very readable
- ✓ Good for teaching
- ✓ Useful for debugging

### Syntax

```
[NULL]
[TRUE]
[FALSE]
[INT(42)]
[FLOAT(3.14)]
[STR("hello")]
[ARRAY(element1 element2 ...)]
[OBJECT(key1:value1 key2:value2 ...)]
[CONTRACT(id: field1:value1 field2:value2 ...)]
```

### Examples

```
[NULL]                      // null
[TRUE]                      // true
[FALSE]                     // false
[INT(42)]                   // integer 42
[FLOAT(3.14)]               // float 3.14
[STR("hello")]              // string "hello"
[ARRAY([INT(1)] [INT(2)])]  // array [1, 2]
[OBJECT(a:[INT(1)] b:[INT(2)])]  // object {a:1, b:2}
```

### Complex Example

```
[OBJECT(
  name:[STR("Alice")]
  age:[INT(30)]
  courses:[ARRAY([STR("Math")] [STR("Physics")])]
)]
```

---

## Format Comparison

### Example: {a: 1, b: "hello"}

**LC-B (9 bytes):**
```
7C 50 02 30 01 61 10 01 30 01 62 30 05 68656C6C6F [sig]
```

**LC-R (25 characters):**
```
ⓡ{a:ⓡ1 b:ⓡ"hello"}
```

**LC-T (50 characters):**
```
[OBJECT(a:[INT(1)] b:[STR("hello")])]
```

| Format | Size | Purpose |
|--------|------|---------|
| LC-B | Smallest | Production |
| LC-R | Medium | Debugging |
| LC-T | Largest | Learning |

---

## Conversion Between Formats

### LC-B ↔ LC-R

```python
from hlx_runtime.lc_codec import encode_lcb, decode_lcb, encode_runic

value = {"a": 1, "b": "hello"}

# Value → LC-B
lcb_bytes = encode_lcb(value)

# LC-B → LC-R
lcr_text = encode_runic(lcb_bytes)
print(lcr_text)  # ⓡ{a:ⓡ1 b:ⓡ"hello"}

# LC-B ← LC-R
decoded = decode_lcb(lcb_bytes)
print(decoded)   # {'a': 1, 'b': 'hello'}
```

### LC-T Parsing

```python
from hlx_runtime.lc_codec import LCTParser

parser = LCTParser()
value = parser.parse_text("[INT(42)]")
print(value)  # 42

text_repr = parser.to_text({"x": 10})
print(text_repr)  # [OBJECT(x:[INT(10)])]
```

---

## Signature & Verification

### SHA-256 Signing

All LC-B values can be cryptographically signed:

```
[LC-B VALUE BYTES] → SHA-256 → [32-BYTE SIGNATURE]
```

**Process:**
1. Encode value to LC-B (excluding signature field)
2. Compute SHA-256 hash of LC-B bytes
3. Append 32-byte signature to message

**Example:**
```python
from hlx_runtime.lc_codec import encode_lcb, compute_signature, verify_signature

value = {"id": 1, "name": "Alice"}
lcb_bytes = encode_lcb(value)
signature = compute_signature(lcb_bytes)

# Later, verify:
is_valid = verify_signature(lcb_bytes, signature)
```

### Deterministic Signatures

**Key property:** Same value always produces same signature.

```
encode(value1) = encode(value2)  →  signature(value1) = signature(value2)
```

This enables:
- Content-addressed storage
- Deduplication
- Integrity verification
- Reproducibility validation

---

## Streaming & Chunking

### Chunked Encoding

For large values, LC-B supports streaming:

```python
from hlx_runtime.lc_codec import LCBStreamer

value = [1, 2, 3, 4, 5]
streamer = LCBStreamer()

# Encode in chunks
for chunk in streamer.encode_chunks(value, chunk_size=256):
    send_to_network(chunk)

# Decode incrementally
decoder = LCBStreamer()
for chunk in receive_from_network():
    result = decoder.decode_chunk(chunk)
    if result:
        print(result)  # Prints value when complete
```

---

## Performance Characteristics

### Encoding Speed

```
Format  | Speed    | Notes
--------|----------|-------
LC-B    | Fastest  | Native binary
LC-R    | Fast     | Text generation
LC-T    | Slower   | Requires parsing
```

### Decoding Speed

```
Format  | Speed    | Notes
--------|----------|-------
LC-B    | Fastest  | Direct binary read
LC-R    | Fast     | Glyph mapping
LC-T    | Slow     | Full parsing
```

### Size Comparison

```
Format  | Avg Size | Ratio
--------|----------|-------
LC-B    | Baseline | 1.0×
LC-R    | 2-3×     | 2.5×
LC-T    | 4-5×     | 4.5×
```

---

## Compliance & Testing

### Format Compliance Tests

All implementations must pass:

```
✓ Encode any value → LC-B
✓ Decode LC-B → original value
✓ Encode → Decode → Encode = same LC-B
✓ LC-B → LC-R → inspectable
✓ LC-T parses all types
✓ Signatures deterministic
✓ All field orderings sorted
```

### Determinism Verification

```python
from hlx_runtime.lc_codec import verify_determinism

value = {"z": 1, "a": 2, "m": 3}

# Run 1000 times
for _ in range(1000):
    encoded1 = encode_lcb(value)
    encoded2 = encode_lcb(value)
    assert encoded1 == encoded2, "Non-deterministic encoding!"
```

---

## Error Handling

### Encoding Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| `E_FLOAT_SPECIAL` | NaN or Infinity | Use normalized values |
| `E_DEPTH_EXCEEDED` | >64 levels nested | Flatten structure |
| `E_FIELD_ORDER` | Keys not sorted | Sort object keys |

### Decoding Errors

| Error | Cause | Recovery |
|-------|-------|----------|
| `E_LC_PARSE` | Invalid LC-B | Validate encoding |
| `E_TRUNCATED` | Incomplete data | Retry reception |
| `E_SIGNATURE` | Hash mismatch | Validate source |

---

## Example Use Cases

### Use Case 1: Network Protocol

```
Client → [LC-B serialized value] → Server
Server ← [LC-B serialized result] ← Client
```

Binary format minimizes bandwidth.

### Use Case 2: Content Addressing

```
value = {id: 42, ...}
hash = SHA-256(encode_lcb(value))
storage[hash] = value
```

Deterministic encoding enables content-addressed storage.

### Use Case 3: Debugging

```
# During development
print(encode_runic(lcb_bytes))  # ⓡ{...}

# In production
send(lcb_bytes)  # Compact binary
```

### Use Case 4: Cryptographic Verification

```
lcb_bytes = encode_lcb(value)
signature = sign(lcb_bytes)

# Later
verify(signature, lcb_bytes)  # True or False
```

---

## Next Steps

- **HLXL syntax:** See [HLXL_SPECIFICATION.md](HLXL_SPECIFICATION.md)
- **Contract system:** See [CONTRACT_SYSTEM.md](CONTRACT_SYSTEM.md)
- **Type system:** See [TYPE_SYSTEM.md](TYPE_SYSTEM.md)
- **Implementation:** See runtime modules

---

**Version:** 1.1.0
**Status:** Production-ready
**All formats:** Bijective and deterministic
