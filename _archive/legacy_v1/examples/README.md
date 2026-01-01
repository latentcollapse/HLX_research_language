# HLX Examples

Simple example programs demonstrating HLXL syntax and features.

## Running Examples

### Using the Runtime Directly

```bash
cd runtime/hlx_runtime

python3 <<EOF
from hlxl_runtime import HLXLBasicRuntime

# Read and execute example
with open('../../examples/hello_world.hlxl') as f:
    code = f.read()

r = HLXLBasicRuntime()
r.execute(code)
EOF
```

### Interactive Mode

```bash
cd runtime/hlx_runtime

python3 <<EOF
from hlxl_runtime import HLXLBasicRuntime

r = HLXLBasicRuntime()

# Try commands interactively
r.execute('print("Hello from HLXL!")')
r.execute('let x = 42')
print(r.get_var('x'))  # 42
EOF
```

## Examples

### hello_world.hlxl

Basic "Hello World" with variables and string concatenation.

### matrix_multiply.hlxl

2x2 matrix multiplication demonstrating array operations.

**Expected output:**
```
Matrix A: [[1, 2], [3, 4]]
Matrix B: [[5, 6], [7, 8]]
Result C: [[19, 22], [43, 50]]
```

## Full Transformer Example

The full transformer training is in `src/bin/train_transformer_full.rs`. Run with:

```bash
./target/release/train_transformer_full
```

This demonstrates:
- 4-layer transformer (3.32M parameters)
- GPU compilation via Vulkan/SPIR-V
- Deterministic training (100% reproducible)
- Achieves 0.4783 final loss (6.7% better than CUDA)

## Next Steps

- Modify these examples and experiment
- Read [README_AMD.md](../README_AMD.md) for architecture details
- Check [QUICKSTART.md](../QUICKSTART.md) for setup instructions
