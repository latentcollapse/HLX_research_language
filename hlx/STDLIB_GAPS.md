# HLX Standard Library: Missing Builtins Analysis

## Overview

HLX is a tensor-native compute language focused on deterministic GPU-accelerated operations. This document analyzes critical missing builtins and stdlib functions needed for practical use.

**Current Status**: ~90+ operations implemented across 5 phases
**Key Gap**: Tensor manipulation and introspection builtins missing

---

## Critical Priority: Tensor Manipulation

### Tensor Creation (HIGH PRIORITY)
- ✅ `tensor(data, shape)` - Create tensor from array data **[JUST IMPLEMENTED!]**
- ❌ `zeros(shape)` - Create tensor filled with 0.0
- ❌ `ones(shape)` - Create tensor filled with 1.0
- ❌ `full(shape, value)` - Create tensor filled with specific value
- ❌ `random(shape, min, max)` - Random uniform values
- ❌ `random_normal(shape, mean, std)` - Random normal distribution
- ❌ `arange(start, end, step)` - Range tensor [0, 1, 2, ...]
- ❌ `linspace(start, end, count)` - Linearly spaced values
- ❌ `eye(n)` - Identity matrix

**Rationale**: These are foundational for any tensor computation. Currently users can only create tensors from explicit array data or load images.

### Tensor Introspection (HIGH PRIORITY)
- ❌ `shape(tensor)` - Returns array of dimensions
- ❌ `size(tensor)` - Total element count
- ❌ `rank(tensor)` - Number of dimensions
- ❌ `dtype(tensor)` - Data type (F32, F64, I32, etc.)

**Rationale**: Cannot inspect tensor properties at runtime. Critical for dynamic programming.

**Example Current Limitation**:
```hlx
let img = load_image("test.png");
// How do I know the shape? Can't check!
// Have to hardcode assumptions about dimensions
```

**With Introspection**:
```hlx
let img = load_image("test.png");
let dims = shape(img);  // [1080, 1920, 4]
let h = dims[0];
let w = dims[1];
```

### Tensor Indexing/Slicing (MEDIUM-HIGH PRIORITY)
- ❌ `get(tensor, indices)` - Extract single element
- ❌ `set(tensor, indices, value)` - Set single element
- ❌ `slice(tensor, axis, start, end)` - Extract slice along axis
- ❌ `index(tensor, indices_tensor)` - Advanced indexing
- ❌ `concat(tensors, axis)` - Concatenate tensors
- ❌ `stack(tensors, axis)` - Stack tensors into new dimension
- ❌ `split(tensor, splits, axis)` - Split tensor

**Rationale**: Currently cannot manipulate tensor data beyond whole-tensor operations.

---

## Math Operations

### Reductions (HIGH PRIORITY)
- ❌ `sum(tensor, axis)` - Sum along axis (or all axes if none)
- ❌ `mean(tensor, axis)` - Mean along axis
- ❌ `max(tensor, axis)` - Maximum along axis
- ❌ `min(tensor, axis)` - Minimum along axis
- ❌ `argmax(tensor, axis)` - Index of maximum
- ❌ `argmin(tensor, axis)` - Index of minimum
- ❌ `prod(tensor, axis)` - Product along axis
- ❌ `std(tensor, axis)` - Standard deviation
- ❌ `var(tensor, axis)` - Variance

**Rationale**: Essential for statistics, loss functions, and data analysis.

### Element-wise Math (MEDIUM PRIORITY)
Currently have: sin, cos, tan, exp, log, sqrt, abs, pow, min, max
Still missing:
- ❌ `tanh(tensor)` - Hyperbolic tangent
- ❌ `sinh(tensor)` - Hyperbolic sine
- ❌ `cosh(tensor)` - Hyperbolic cosine
- ❌ `atan2(y, x)` - Two-argument arctangent
- ❌ `clamp(tensor, min, max)` - Clamp values to range
- ❌ `sign(tensor)` - Sign (-1, 0, or 1)
- ❌ `round(tensor)` - Round to nearest integer
- ❌ `ceil(tensor)` - Ceiling
- ❌ `floor(tensor)` - Floor

---

## Array Operations (For non-tensor data)

### Array Manipulation (MEDIUM PRIORITY)
Currently have: array indexing, array creation
Still missing:
- ❌ `len(array)` - Array length
- ❌ `push(array, element)` - Append element
- ❌ `pop(array)` - Remove and return last element
- ❌ `insert(array, index, element)` - Insert at index
- ❌ `remove(array, index)` - Remove at index
- ❌ `slice(array, start, end)` - Extract slice
- ❌ `reverse(array)` - Reverse order
- ❌ `sort(array)` - Sort ascending
- ❌ `contains(array, element)` - Check membership
- ❌ `index_of(array, element)` - Find index

**Rationale**: Arrays are used for metadata, configs, coordinates, etc. Need basic manipulation.

---

## String Operations

Currently have: Basic string concat, string equality, print
Still missing:
- ❌ `strlen(string)` - String length
- ❌ `substr(string, start, end)` - Substring
- ❌ `split(string, delimiter)` - Split into array
- ❌ `join(array, separator)` - Join array into string
- ❌ `trim(string)` - Remove whitespace
- ❌ `upper(string)` - Uppercase
- ❌ `lower(string)` - Lowercase
- ❌ `replace(string, old, new)` - String replacement
- ❌ `starts_with(string, prefix)` - Prefix check
- ❌ `ends_with(string, suffix)` - Suffix check
- ❌ `contains(string, substring)` - Substring check
- ❌ `to_string(value)` - Convert to string
- ❌ `parse_int(string)` - Parse integer
- ❌ `parse_float(string)` - Parse float

---

## File I/O Enhancement

Currently have: read_file, write_file, load_image, save_image, parse_json, write_json, parse_csv, write_csv
Still missing:
- ❌ `file_exists(path)` - Check if file exists
- ❌ `list_dir(path)` - List directory contents
- ❌ `create_dir(path)` - Create directory
- ❌ `delete_file(path)` - Delete file
- ❌ `rename_file(old, new)` - Rename/move file
- ❌ `file_size(path)` - Get file size in bytes
- ❌ `is_dir(path)` - Check if path is directory
- ❌ `is_file(path)` - Check if path is file

---

## Specialized Operations

### Image Processing (Already Implemented!)
- ✅ load_image, save_image
- ✅ grayscale, threshold, brightness, contrast, invert_colors, sharpen
- ⏳ gaussian_blur, sobel_edges (shaders exist, dispatch needed)

### Linear Algebra (Partially Implemented)
- ✅ matmul, matmul_bias
- ❌ `dot(a, b)` - Dot product
- ❌ `cross(a, b)` - Cross product
- ❌ `norm(tensor, p)` - Lp norm
- ❌ `normalize(tensor)` - Normalize to unit length
- ❌ `det(matrix)` - Determinant
- ❌ `inv(matrix)` - Matrix inverse
- ❌ `solve(A, b)` - Solve linear system Ax=b
- ❌ `eig(matrix)` - Eigenvalues/eigenvectors
- ❌ `svd(matrix)` - Singular value decomposition

### Neural Network Operations (Partially Implemented)
- ✅ LayerNorm, Softmax, Gelu, Relu, Attention
- ❌ `sigmoid(tensor)` - Sigmoid activation
- ❌ `leaky_relu(tensor, alpha)` - Leaky ReLU
- ❌ `elu(tensor, alpha)` - ELU activation
- ❌ `selu(tensor)` - SELU activation
- ❌ `dropout(tensor, rate)` - Dropout layer
- ❌ `conv2d(input, kernel, stride, padding)` - 2D convolution
- ❌ `max_pool(input, size, stride)` - Max pooling
- ❌ `avg_pool(input, size, stride)` - Average pooling
- ❌ `batch_norm(input, gamma, beta)` - Batch normalization

---

## Implementation Priority Ranking

### Tier 1: Absolutely Critical (Should implement next)
1. **Tensor introspection**: `shape()`, `size()`, `rank()`, `dtype()`
2. **Tensor creation helpers**: `zeros()`, `ones()`, `full()`
3. **Basic reductions**: `sum()`, `mean()`, `max()`, `min()`
4. **Array length**: `len()`

**Rationale**: Without these, HLX feels incomplete for tensor programming.

### Tier 2: High Value (Next sprint)
1. **Tensor indexing**: `get()`, `set()`, `slice()`
2. **Math reductions**: `argmax()`, `argmin()`, `std()`, `var()`
3. **Random tensors**: `random()`, `random_normal()`
4. **String basics**: `strlen()`, `substr()`, `split()`, `join()`
5. **Array manipulation**: `push()`, `pop()`, `slice()`, `contains()`

### Tier 3: Nice to Have (Future)
1. **Advanced math**: `atan2()`, `clamp()`, `sign()`, `round()`
2. **Linear algebra**: `dot()`, `norm()`, `normalize()`, `inv()`
3. **More NN ops**: `sigmoid()`, `conv2d()`, `pooling()`
4. **File system ops**: `file_exists()`, `list_dir()`, `delete_file()`

---

## Example: What We Can't Do Right Now

### Cannot: Normalize Image
```hlx
let img = load_image("photo.jpg");
// STUCK: How do I normalize to mean=0, std=1?
// Need: mean(), std(), shape() to do this
```

### Cannot: Dynamic Tensor Operations
```hlx
fn process(data) {
    // STUCK: Is data a [10, 10] or [5, 20] tensor?
    // Need: shape() to branch on dimensions
}
```

### Cannot: Basic Array Manipulation
```hlx
let coords = [1, 2, 3, 4];
// STUCK: How many coordinates? Need len()
// STUCK: Want to add one more? Need push()
```

### Cannot: Find Maximum Value
```hlx
let scores = tensor([0.1, 0.8, 0.3, 0.6], [4]);
// STUCK: Which score is highest? Need argmax()
// STUCK: What's the average? Need mean()
```

---

## Current Strengths

What HLX **DOES** have well-covered:
1. ✅ Excellent math functions (11 ops: sin, cos, exp, log, sqrt, etc.)
2. ✅ Comprehensive string operations (12 ops: concat, upper, lower, trim, etc.)
3. ✅ Rich file I/O (read, write, JSON, CSV, images)
4. ✅ Image processing (8 GPU-accelerated operations)
5. ✅ Neural network layers (LayerNorm, Softmax, Attention, etc.)
6. ✅ Deterministic random number generation
7. ✅ Control flow (if/else, while, for loops)
8. ✅ Functions and recursion

What HLX **LACKS**:
1. ❌ Tensor introspection (can't inspect shapes/types)
2. ❌ Tensor manipulation (can't slice, index, concat)
3. ❌ Reduction operations (can't compute sum, mean, max)
4. ❌ Dynamic tensor creation (only zeros/ones via workarounds)
5. ❌ Array utilities (can't get length, push, pop)

---

## Recommendations

### Short Term (Next Session)
Implement Tier 1 builtins:
- `shape(tensor) -> array`
- `size(tensor) -> int`
- `zeros(shape) -> tensor`
- `ones(shape) -> tensor`
- `len(array) -> int`
- `sum(tensor, axis?) -> tensor/scalar`
- `mean(tensor, axis?) -> tensor/scalar`

**Impact**: These ~7 builtins would make HLX feel complete for basic tensor programming.

### Medium Term (This Week)
Implement Tier 2 builtins:
- Tensor indexing/slicing
- argmax/argmin
- Random tensor generation
- String manipulation
- Array manipulation

**Impact**: HLX becomes practical for real-world data science and ML tasks.

### Long Term (Future Sprints)
- Advanced linear algebra
- More neural network layers
- File system operations
- Performance optimization of existing ops

---

## Conclusion

HLX has an impressive **90+ operations** implemented, but the missing **tensor introspection** and **manipulation** builtins create a significant usability gap.

The good news: Implementing ~20 critical builtins (Tier 1 + Tier 2) would transform HLX from a demo language into a genuinely usable tensor programming language.

**Estimated Effort**: ~3-4 hours for Tier 1 (7 builtins) given current velocity

---

## Stats

- **Operations Implemented**: ~90
- **Critical Missing**: ~20 (Tier 1+2)
- **Nice to Have Missing**: ~50 (Tier 3)
- **Completion**: ~60% of "practical tensor language" featureset
- **Completion**: ~85% of "demo language" features

With Tier 1+2 implemented: ~90% practical usability!
