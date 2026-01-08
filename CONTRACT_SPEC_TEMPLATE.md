# Contract Documentation Template

Use this template when adding new contracts to `CONTRACT_CATALOGUE.json`.

## JSON Schema

```json
"CONTRACT_ID": {
  "name": "ShortDescriptiveName",
  "tier": "T0-Core|T1-AST|T2-Reserved|T3-Parser|T4-GPU|T5-UserDefined",
  "signature": "@ID { @0: type, @1: type, ... }",
  "description": "One-line description of what this contract does",
  "fields": {
    "field_name": {
      "type": "type_name",
      "description": "What this field represents",
      "required": true
    }
  },
  "example": "@ID { field: value, field2: value2 }",
  "usage": "When/where/why to use this contract",
  "performance": "Complexity, GPU acceleration, benchmarks (optional)",
  "related": ["other_contract_id", "another_id"],
  "status": "stable|experimental|compiler-internal|deprecated",
  "implementation": "path/to/implementation.rs (optional)"
}
```

## Field Descriptions

### Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | String | Short, descriptive name (PascalCase) |
| `tier` | String | Which tier this contract belongs to (see below) |
| `signature` | String | Contract syntax with field names and types |
| `description` | String | One-line explanation of purpose |
| `fields` | Object | Map of field names to field specs |
| `example` | String | Working code example |
| `usage` | String | When/why to use this |
| `related` | Array | IDs of related contracts |
| `status` | String | Stability level (see below) |

### Optional Fields

| Field | Type | Description |
|-------|------|-------------|
| `performance` | String | Complexity, benchmarks, GPU info |
| `implementation` | String | Path to implementation code |
| `notes` | Array | Additional notes/warnings |
| `aliases` | Array | Alternative names |

## Tier System

| Tier | Range | Purpose | Examples |
|------|-------|---------|----------|
| **T0-Core** | 0-99 | Fundamental types | Int, Float, String, Bool |
| **T1-AST** | 100-199 | Compiler internals | Block, Expr, Statement |
| **T2-Reserved** | 200-799 | Future expansion | Math, String, Array ops |
| **T3-Parser** | 800-899 | Text parsing | JSON, CSV, Binary formats |
| **T4-GPU** | 900-999 | GPU operations | GEMM, LayerNorm, Softmax |
| **T5-UserDefined** | 1000+ | Application-specific | Custom user contracts |

## Status Values

| Status | Meaning | Use When |
|--------|---------|----------|
| `stable` | Production-ready, won't change | Contract is finalized |
| `experimental` | Works but may change | Testing new features |
| `compiler-internal` | Not for user code | AST nodes, internal |
| `deprecated` | Use alternative instead | Being phased out |
| `proposed` | Design only, not implemented | RFC stage |

## Examples

### Example 1: Simple Type Contract

```json
"23": {
  "name": "Byte",
  "tier": "T0-Core",
  "signature": "@23 { @0: u8 }",
  "description": "Single unsigned byte (0-255)",
  "fields": {
    "@0": {
      "type": "u8",
      "description": "Byte value",
      "required": true
    }
  },
  "example": "@23 { @0: 255 }",
  "usage": "For binary data, flags, or small integers",
  "related": ["14", "17"],
  "status": "stable"
}
```

### Example 2: Operation Contract

```json
"200": {
  "name": "Add",
  "tier": "T2-Math",
  "signature": "@200 { @0: Number, @1: Number } -> Number",
  "description": "Addition operation (a + b)",
  "fields": {
    "@0": {
      "type": "Number",
      "description": "Left operand",
      "required": true
    },
    "@1": {
      "type": "Number",
      "description": "Right operand",
      "required": true
    }
  },
  "example": "let sum = @200 { @0: 5, @1: 10 };  // 15",
  "usage": "Arithmetic addition for Int or Float types",
  "performance": "O(1) constant time",
  "related": ["201", "202", "203"],
  "status": "stable"
}
```

### Example 3: GPU Contract

```json
"911": {
  "name": "MatrixTranspose",
  "tier": "T4-GPU",
  "signature": "@911 { matrix: Tensor<M×N> } -> Tensor<N×M>",
  "description": "Transpose a matrix (swap rows and columns)",
  "fields": {
    "matrix": {
      "type": "Tensor<M×N>",
      "description": "Input matrix to transpose",
      "required": true
    }
  },
  "example": "let transposed = @911 { matrix: [[1,2,3], [4,5,6]] };  // [[1,4], [2,5], [3,6]]",
  "usage": "Linear algebra, neural network operations",
  "performance": "O(M×N), GPU-accelerated with shared memory optimization",
  "related": ["906", "912"],
  "status": "stable",
  "implementation": "hlx-vulkan/shaders/transpose.comp"
}
```

### Example 4: Parser Contract

```json
"805": {
  "name": "JSONParse",
  "tier": "T3-Parser",
  "signature": "@805 { input: String } -> Object | Error",
  "description": "Parse JSON string into HLX object",
  "fields": {
    "input": {
      "type": "String",
      "description": "JSON-formatted string",
      "required": true
    }
  },
  "example": "let obj = @805 { input: '{\"name\": \"Alice\", \"age\": 30}' };",
  "usage": "Parse JSON from APIs, config files, or user input",
  "performance": "O(n) where n is string length",
  "related": ["806", "807"],
  "status": "stable",
  "notes": [
    "Returns null on parse error",
    "Supports nested objects and arrays",
    "Handles escaped characters"
  ]
}
```

## Guidelines for Good Documentation

### DO ✅

- **Use real examples** - Show actual working code
- **Be specific** - "Matrix multiplication" not "Does math"
- **Link related contracts** - Help users discover functionality
- **Include complexity** - O(n), O(n²), GPU-accelerated, etc.
- **Explain use cases** - When/why someone would use this
- **Be consistent** - Follow the schema exactly

### DON'T ❌

- **Don't be vague** - "Does stuff with data" is useless
- **Don't duplicate info** - Description ≠ Name
- **Don't skip examples** - Examples are critical for learning
- **Don't use jargon** - Explain technical terms
- **Don't break the schema** - Validate with `jq` before committing

## Contract Numbering Guidelines

### How to Choose a Contract ID

1. **Check tier boundaries** - Use appropriate range for contract type
2. **Group related contracts** - Keep related ops near each other
3. **Leave gaps** - Room for future contracts in same category
4. **Document the gap** - Explain what future contracts might fill it

Example:
```json
// Math operations tier (T2)
"200": "Add",
"201": "Subtract",
"202": "Multiply",
"203": "Divide",
// 204-209: Reserved for more arithmetic ops
"210": "Power",
"211": "SquareRoot",
// 212-219: Reserved for power/root ops
"220": "Sin",
"221": "Cos",
// etc.
```

## Validation Checklist

Before submitting new contracts:

- [ ] JSON syntax is valid (`jq empty CONTRACT_CATALOGUE.json`)
- [ ] All required fields present
- [ ] Contract ID doesn't conflict with existing
- [ ] Contract ID is in correct tier range
- [ ] Example code is valid HLX syntax
- [ ] Related contracts exist and are linked
- [ ] Status is accurate (stable/experimental/etc)
- [ ] Description is clear and concise
- [ ] Fields are well-documented

## Questions?

If you're unsure about:
- **Which tier?** → Check existing similar contracts
- **What ID number?** → Find the relevant range, use next available
- **Status?** → When in doubt, use "experimental"
- **Performance?** → Skip if unknown, Claude can fill it in later

Add questions/notes to the collab doc (`CLAUDE_GEMINI_COLLAB.md`) if stuck!

---

**Remember**: This documentation directly powers the LSP autocomplete and hover docs. Quality here = quality IDE experience! 🎯
