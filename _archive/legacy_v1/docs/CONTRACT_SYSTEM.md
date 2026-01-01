# Contract System Specification

## Overview

The **Contract System** is HLX's approach to schema validation and deterministic type safety. Contracts:
- Define **type schemas** for structured data
- Enable **cryptographic verification**
- Provide **deterministic validation**
- Support **version management**
- Allow **safe cross-vendor execution**

---

## Core Concept

A **Contract** is a versioned schema that defines fields and their types:

```hlxl
// Define a contract with @14
let user = @14 {
    @0: "alice",        // String field
    @1: 30,             // Integer field
    @2: true            // Boolean field
}
```

Contract `@14` defines:
- Field `@0` must be string
- Field `@1` must be integer
- Field `@2` must be boolean

---

## Contract Structure

### Contract Definition

```
@CONTRACT_ID {
    @FIELD_0: value_0,
    @FIELD_1: value_1,
    @FIELD_2: value_2,
    ...
}
```

### Contract Components

| Component | Range | Example |
|-----------|-------|---------|
| **ID** | 0-999 | `@14` |
| **Field Index** | 0-255 | `@0`, `@1`, `@127` |
| **Field Value** | Any type | Integer, string, array, etc. |

---

## Built-in Contracts

HLX defines several standard contracts:

### Contract @0: Reserved (System)

Reserved for future system use.

### Contract @1-@13: General Purpose

Available for user-defined structures:

```hlxl
let record = @5 {
    @0: "data",
    @1: 42
}
```

### Contract @14: User Structure (Standard)

Most common for general data:

```hlxl
let person = @14 {
    @0: "Alice",        // name
    @1: 30,             // age
    @2: ["Math", "Physics"]  // skills
}
```

### Contract @100+: Application-Specific

Reserved for specialized use:

```hlxl
let gpu_kernel = @905 {
    @0: "matrix_multiply",
    @1: [256, 256, 256],  // dimensions
    @2: true              // optimized
}
```

---

## Deterministic Validation

### Validation Rules

When encoding a contract, HLX validates:

1. **Field ordering** - Fields must be in numeric order (@0, @1, @2, ...)
2. **Type consistency** - Fields must match expected types
3. **Field completeness** - No missing required fields
4. **Value constraints** - Values must be in valid range

### Example: Valid Contract

```hlxl
let valid = @14 {
    @0: "Alice",
    @1: 30,
    @2: true
}
// ✓ Fields in order (@0, @1, @2)
// ✓ Types match schema
// ✓ All fields present
```

### Example: Invalid Contract

```hlxl
let invalid = @14 {
    @2: true,           // Wrong order!
    @0: "Alice",
    @1: 30
}
// ✗ Fields not in order
// Error: E_FIELD_ORDER
```

---

## Contract Versioning

### Immutable Versioning

Contract schemas are **immutable** - once defined, they never change:

```
CONTRACT_14_V1.0:
  @0: string
  @1: integer
  @2: boolean
```

If you need a different schema, create a new contract:

```
CONTRACT_15_V2.0:  // New contract ID
  @0: string
  @1: integer
  @2: boolean
  @3: array        // New field
```

### Version Independence

Each contract is independent:

```hlxl
let v1 = @14 {          // Contract 14
    @0: "Alice",
    @1: 30,
    @2: true
}

let v2 = @15 {          // Contract 15 (different schema)
    @0: "Bob",
    @1: 25,
    @2: true,
    @3: ["Physics"]     // New field
}
```

---

## Type Safety

### Field Types

Contract fields can contain any HLX type:

```hlxl
let complex = @14 {
    @0: "string",           // String
    @1: 42,                 // Integer
    @2: 3.14,               // Float
    @3: true,               // Boolean
    @4: null,               // Null
    @5: [1, 2, 3],          // Array
    @6: {x: 1, y: 2},       // Object
    @7: @14 {...}           // Nested contract
}
```

### Type Validation

When wrapping a value in a contract, type checking occurs:

```hlxl
let user = @14 {
    @0: "Alice",    // Must be string
    @1: 30,         // Must be integer
    @2: true        // Must be boolean
}

let invalid = @14 {
    @0: 123,        // ✗ Error: expected string, got integer
    @1: "thirty",   // ✗ Error: expected integer, got string
    @2: true
}
```

---

## Deterministic Encoding

### Encoding Process

```
Contract Value
  ↓
Type Validation
  ↓
Field Ordering Check
  ↓
LC-B Encoding
  ↓
SHA-256 Signature
  ↓
Final LC-B Message
```

### Example: Step-by-step

```hlxl
let contract_value = @14 {
    @0: "alice",
    @1: 30,
    @2: true
}
```

**Step 1: Type Validation**
```
@0 "alice" ✓ (string)
@1 30      ✓ (integer)
@2 true    ✓ (boolean)
```

**Step 2: Field Ordering**
```
@0 < @1 < @2 ✓ (ascending)
```

**Step 3: LC-B Encoding**
```
60 0E 03            // Contract type, ID=14, 3 fields
00 30 05 "alice"    // @0: string "alice"
01 10 1E            // @1: integer 30
02 02               // @2: boolean true
```

**Step 4: SHA-256 Signature**
```
[32 bytes of signature]
```

### Reproducibility Guarantee

```
encode(contract) = encode(contract)  // Always identical
```

Same contract always produces byte-identical encoding.

---

## Contract Registry

### Standard Contracts (0-99)

| ID | Name | Fields | Purpose |
|----|------|--------|---------|
| 1-13 | Reserved | — | General use |
| 14 | User | 3-N | Standard data structure |
| 15-99 | Available | — | Custom schemas |

### GPU Contracts (900-999)

Reserved for GPU compute:

| ID | Name | Fields | Purpose |
|----|------|--------|---------|
| 900 | Reserved | — | GPU metadata |
| 901-910 | Ops | 2-4 | GPU operations |
| 911-999 | Custom | — | Application-specific |

---

## Advanced: Nested Contracts

### Contracts within Contracts

```hlxl
let inner = @14 {
    @0: "nested",
    @1: 42,
    @2: true
}

let outer = @15 {
    @0: "container",
    @1: inner,              // Nested contract
    @2: [inner, inner]      // Array of contracts
}
```

### Validation with Nesting

Each contract validated independently:

```
outer (@15)
  ├─ @0: "container" ✓
  ├─ @1: inner (@14)
  │   ├─ @0: "nested" ✓
  │   ├─ @1: 42 ✓
  │   └─ @2: true ✓
  └─ @2: [inner, inner]
      └─ Both validated ✓
```

---

## Determinism Axioms for Contracts

### Axiom C1: Schema Immutability

Once defined, a contract's schema never changes:
```
schema(@14) = schema(@14)  // Always same
```

### Axiom C2: Type Consistency

Same field always has same type:
```
type(contract.@0) = type(contract.@0)  // Always same type
```

### Axiom C3: Field Ordering

Fields always encoded in ascending order:
```
encode({ @2, @0, @1 }) = encode({ @0, @1, @2 })
```

### Axiom C4: Reproducible Validation

Validation result is deterministic:
```
validate(contract) = validate(contract)  // Same result always
```

---

## Error Handling

### Validation Errors

| Error | Cause | Example |
|-------|-------|---------|
| `E_FIELD_ORDER` | Fields not in order | `{@2: ..., @0: ...}` |
| `E_TYPE_ERROR` | Wrong field type | `@0: 123` when expecting string |
| `E_CONTRACT_UNKNOWN` | Invalid contract ID | `@999` (if not defined) |
| `E_FIELD_MISSING` | Required field absent | `@14 {@0: "a"}` (missing @1, @2) |

### Error Recovery

```hlxl
// Construct contract safely
let safe = @14 {
    @0: "Alice",
    @1: 30,
    @2: true
}
// All validations pass ✓

// Attempt invalid construction
let invalid = @14 {
    @0: 123,  // Type error!
}
// Error: E_TYPE_ERROR at @0
```

---

## Use Cases

### Use Case 1: API Schema

```hlxl
// Request contract
let request = @20 {
    @0: "GET",              // method
    @1: "/api/users",       // path
    @2: {token: "abc123"}   // headers
}

// Response contract
let response = @21 {
    @0: 200,                // status
    @1: {id: 1, name: "Alice"},  // body
    @2: {content-type: "application/json"}  // headers
}
```

### Use Case 2: GPU Kernel

```hlxl
let kernel = @905 {
    @0: "matmul",           // operation name
    @1: [256, 256, 256],    // dimensions
    @2: true,               // optimized
    @3: 0.4783              // expected_loss
}
```

### Use Case 3: Cryptographic Verification

```hlxl
let signed_data = @30 {
    @0: {data: "important"},      // payload
    @1: "abc123def456...",         // signature
    @2: true                       // verified
}

// Signature can be validated deterministically
```

---

## Contract Inspection

### Query Contract ID

```python
from hlx_runtime.contracts import get_contract_id

contract = {"@_id": 14, "@0": "alice", "@1": 30}
contract_id = get_contract_id(contract)
print(contract_id)  # 14
```

### Validate Contract

```python
from hlx_runtime.contracts import validate_contract

try:
    validate_contract(14, {"@0": "alice", "@1": 30, "@2": true})
    print("Valid!")
except Exception as e:
    print(f"Invalid: {e}")
```

### Extract Fields

```python
from hlx_runtime.contracts import get_fields

contract = {"@_id": 14, "@0": "alice", "@1": 30, "@2": true}
fields = get_fields(contract)
# Returns: {0: "alice", 1: 30, 2: true}
```

---

## Best Practices

### ✓ DO

1. **Define contracts carefully** - Schema should be stable
2. **Use meaningful contract IDs** - Documentable and memorable
3. **Validate early** - Check contracts immediately
4. **Document field meanings** - Add comments outside code:

```
CONTRACT 14:
  @0: Username (string)
  @1: Age (integer)
  @2: Active (boolean)
```

5. **Version with new IDs** - If schema changes, create new contract

### ✗ DON'T

1. **Redefine contract IDs** - IDs must be immutable
2. **Skip field validation** - Always validate
3. **Mix contract types** - Use same ID for same structure
4. **Assume optional fields** - All fields must be present
5. **Use variable field counts** - All contracts must have exact field count

---

## Performance

### Validation Cost

```
Operation  | Cost    | Notes
-----------|---------|-------
Encode     | O(n)    | Linear in field count
Decode     | O(n)    | Linear in field count
Validate   | O(n)    | Linear in field count
```

Where `n` = number of fields (typically 3-10).

### Memory Usage

```
Memory = Contract ID (2 bytes) + Fields (variable)
```

Contracts are memory-efficient.

---

## Next Steps

- **Learning contracts:** See examples in [../examples/](../examples/)
- **Type system:** See [TYPE_SYSTEM.md](TYPE_SYSTEM.md)
- **Wire formats:** See [WIRE_FORMATS.md](WIRE_FORMATS.md)
- **Implementation:** See runtime modules

---

**Version:** 1.1.0
**Status:** Production-ready
**Contracts:** Immutable and deterministic
