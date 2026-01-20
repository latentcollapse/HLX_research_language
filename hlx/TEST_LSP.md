# Quick LSP Test Guide

## 1-Minute Test

```bash
# Build LSP
cd /home/matt/hlx-compiler/hlx
cargo build --release --package hlx_lsp

# Run it (should print startup message)
./target/release/hlx_lsp 2>&1 | head -5

# Check for: "✓ Loaded contract catalogue from ../CONTRACT_CATALOGUE.json"
```

## Create Test File

```bash
cat > test_contracts.hlx << 'EOF'
program contract_test {
    fn main() {
        // Type @ below and see autocomplete
        @

        // Hover these to see docs:
        let x = @14 { @0: 42 };           // Int
        let s = @16 { @0: "hello" };      // String
        let arr = @18 { @0: [1,2,3] };    // Array

        // Math operations
        let sum = @200 { lhs: 5, rhs: 10 };  // Add
        let prod = @202 { lhs: 3, rhs: 4 };  // Mul

        // GPU operations (hover to see performance notes)
        let result = @906 { A: matrix_a, B: matrix_b };  // GEMM
    }
}
EOF
```

## What to Test

### Autocomplete
1. Place cursor after `@` on line 4
2. Trigger autocomplete (Ctrl+Space in most IDEs)
3. Should see list of 45+ contracts
4. Each should show: `@ID - Name - Tier`

### Hover
1. Hover over `@14` - Should show Int contract docs
2. Hover over `@906` - Should show GEMM docs with:
   - Signature
   - Fields (A, B with types)
   - Example code
   - Performance: "O(M×N×K), GPU-accelerated..."
   - Related contracts

### Validation
1. LSP should show diagnostics for syntax errors
2. Red squiggles on unclosed braces, etc.

## Expected Output Examples

### Hover on @906 (GEMM)
```markdown
# @906: GEMM

**Tier:** T4-GPU | **Status:** stable

General Matrix Multiply (C = A @ B)

## Signature
hlx
@906 { A: Tensor<M×K>, B: Tensor<K×N> } -> Tensor<M×N>


## Fields

- `A` (Tensor<M×K>, **required**): Left matrix (M rows, K columns)
- `B` (Tensor<K×N>, **required**): Right matrix (K rows, N columns)

## Example
hlx
let C = @906 { A: matrix_a, B: matrix_b };


## Usage
Matrix multiplication for neural networks, linear algebra

## Performance
O(M×N×K), GPU-accelerated (900+ GFLOPS on RTX 4070)

**Implementation:** `Vulkan compute shader (hlx-vulkan/shaders/gemm.comp)`

## Related Contracts
- @907: LayerNorm
- @908: GELU
```

### Hover on @200 (Add)
```markdown
# @200: Add

**Tier:** T2-Reserved | **Status:** stable

Addition (a + b)

## Signature
hlx
@200 { lhs: Number, rhs: Number } -> Number


## Fields

- `lhs` (Number, **required**): Left operand
- `rhs` (Number, **required**): Right operand

## Example
hlx
@200 { lhs: 5, rhs: 10 }


## Usage
Arithmetic addition
```

## Troubleshooting

### No autocomplete appearing
- Check LSP is running (IDE should show "HLX Language Server" in status bar)
- Check stderr for "✓ Loaded contract catalogue"
- Try typing `@` again

### Hover not working
- Make sure cursor is directly on `@123` part
- Check contract exists: `jq '.contracts["123"]' ../CONTRACT_CATALOGUE.json`
- Try hovering different contracts

### LSP not starting
```bash
# Check binary exists
ls -la ./target/release/hlx_lsp

# Check permissions
chmod +x ./target/release/hlx_lsp

# Run directly to see errors
./target/release/hlx_lsp
```

## Success Criteria

✅ **Working LSP** if you can:
1. Type `@` and see contract list
2. Hover `@906` and see full GEMM documentation
3. See 45+ contracts in autocomplete
4. Hover shows markdown with fields, examples, performance

## Next: Full IDE Setup

See `LSP_CONTRACT_INTEGRATION.md` for complete VS Code setup instructions.
