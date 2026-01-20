# HLX Standard Library Audit
**Date:** 2026-01-08
**Status:** Comprehensive review of stdlib implementation vs contract catalogue

---

## Executive Summary

**Current State:**
- 📁 **stdlib Directory:** 1 file (`math.hlx` - 147 lines)
- 📋 **Contract Catalogue:** 125 contracts defined (T0-T4)
- 🔧 **Implemented:** ~25% (mostly CPU-based)
- ⚠️  **Critical Gaps:** GPU/tensor operations, graphics shaders, high compute

**Priority:** HIGH - Stdlib expansion needed for high-performance graphics and ML workloads

---

## Current stdlib Implementation

### `/stdlib/math.hlx` (147 lines)
**Status:** ✅ Implemented (pure HLX)

**Functions:**
- `abs(x)` - Absolute value
- `min(a, b)` - Minimum of two numbers
- `max(a, b)` - Maximum of two numbers
- `sqrt(x)` - Square root (Newton's method)
- `pow(base, exp)` - Integer power function
- `floor(x)` - Floor function
- `ceil(x)` - Ceiling function
- `clamp(x, min, max)` - Clamp between bounds
- `lerp(a, b, t)` - Linear interpolation
- `sign(x)` - Sign function (-1, 0, 1)

**Issues:**
- ⚠️  Inefficient implementations (loop-based floor/ceil)
- ⚠️  Missing: sin, cos, tan, log, exp (defined in contracts but not in stdlib)
- ⚠️  No GPU acceleration

---

## Contract Catalogue Analysis

### T0-Core (Types) - 9 contracts
**IDs:** 14-22
**Status:** ✅ Fully implemented in runtime

```
14: Int       - Integer literals
15: Float     - Floating-point literals
16: String    - String literals
17: Bytes     - Binary data
18: Array     - Array/list
19: Object    - Dictionary/map
20: Handle    - Resource handles
21: Null      - Null value
22: Bool      - Boolean
```

### T1-AST (Compiler) - 6 contracts
**IDs:** 100-105
**Status:** ✅ Implemented in compiler

```
100: Block        - Code blocks
101: Expr         - Expressions
102: VarRef       - Variable references
103: Assignment   - Assignments
104: FunctionDef  - Function definitions
105: FunctionCall - Function calls
```

### T2-Reserved (Stdlib Operations) - 85 contracts
**Status:** ❌ **MOSTLY UNIMPLEMENTED**

#### Math Operations (200-219) - 20 contracts
**Implemented:** 7/20 (35%)
**Missing:** 13 contracts

```
✅ 200: Add         - Addition
✅ 201: Sub         - Subtraction
✅ 202: Mul         - Multiplication
✅ 203: Div         - Division
✅ 204: Mod         - Modulo
❌ 205: Pow         - Power (stdlib has integer version only)
❌ 206: Sqrt        - Square root (stdlib version is slow)
✅ 207: Abs         - Absolute value
✅ 208: Min         - Minimum
✅ 209: Max         - Maximum
❌ 210: Floor       - Floor (stdlib version is VERY slow)
❌ 211: Ceil        - Ceiling (stdlib version is VERY slow)
❌ 212: Round       - Rounding
❌ 213: Sin         - Sine **MISSING**
❌ 214: Cos         - Cosine **MISSING**
❌ 215: Tan         - Tangent **MISSING**
❌ 216: Log         - Logarithm **MISSING**
❌ 217: Exp         - Exponential **MISSING**
❌ 218: Random      - Random number **MISSING**
✅ 219: Clamp       - Clamp
```

#### String Operations (300-314) - 15 contracts
**Implemented:** 0/15 (0%)
**Status:** ❌ **ALL MISSING**

```
❌ 300: Concat       - String concatenation
❌ 301: StrLen       - String length
❌ 302: Substring    - Substring extraction
❌ 303: IndexOf      - Find substring
❌ 304: Replace      - String replacement
❌ 305: Split        - Split by delimiter
❌ 306: Join         - Join array of strings
❌ 307: ToUpperCase  - Convert to uppercase
❌ 308: ToLowerCase  - Convert to lowercase
❌ 309: Trim         - Trim whitespace
❌ 310: StartsWith   - Check prefix
❌ 311: EndsWith     - Check suffix
❌ 312: Repeat       - Repeat string n times
❌ 313: Reverse      - Reverse string
❌ 314: CharAt       - Get character at index
```

#### Array Operations (400-414) - 15 contracts
**Implemented:** 0/15 (0%)
**Status:** ❌ **ALL MISSING**

```
❌ 400: ArrLen      - Array length
❌ 401: ArrGet      - Get element
❌ 402: ArrSet      - Set element
❌ 403: ArrPush     - Push element
❌ 404: ArrPop      - Pop element
❌ 405: ArrShift    - Shift element
❌ 406: ArrUnshift  - Unshift element
❌ 407: ArrSlice    - Slice array
❌ 408: ArrMap      - Map over array
❌ 409: ArrFilter   - Filter array
❌ 410: ArrReduce   - Reduce array
❌ 411: ArrFind     - Find element
❌ 412: ArrSort     - Sort array
❌ 413: ArrReverse  - Reverse array
❌ 414: ArrConcat   - Concatenate arrays
```

#### Control Flow (500-509) - 10 contracts
**Implemented:** ~5/10 (50% - partially via compiler)
**Status:** ⚠️  **PARTIALLY IMPLEMENTED**

```
⚠️  500: If         - Conditionals (compiler-level)
⚠️  501: While      - While loops (compiler-level)
⚠️  502: For        - For loops (compiler-level)
⚠️  503: Break      - Break statement
⚠️  504: Continue   - Continue statement
❌ 505: Switch      - Switch statement **MISSING**
❌ 506: Match       - Pattern matching **MISSING**
❌ 507: Try         - Exception handling **MISSING**
❌ 508: Throw       - Throw exception **MISSING**
⚠️  509: Return     - Return statement (compiler-level)
```

#### I/O Operations (600-622) - 23 contracts
**Implemented:** 4/23 (17%)
**Status:** ❌ **MOSTLY MISSING**

```
✅ 600: Print         - Print to stdout
❌ 601: ReadLine      - Read line from stdin
✅ 602: ReadFile      - Read file
❌ 603: HttpRequest   - HTTP request
❌ 604: JsonParse     - Parse JSON
❌ 605: Snapshot      - Debug snapshot
❌ 606: WriteSnapshot - Write snapshot
✅ 607: WriteFile     - Write file
❌ 608: AppendFile    - Append to file
❌ 609: FileExists    - Check file exists
❌ 610: DeleteFile    - Delete file
❌ 611: ListFiles     - List files in directory
❌ 612: CreateDir     - Create directory
❌ 613: DeleteDir     - Delete directory
❌ 614: ReadJSON      - Read JSON file
❌ 615: WriteJSON     - Write JSON file
❌ 616: ReadCSV       - Read CSV file
❌ 617: WriteCSV      - Write CSV file
❌ 618: HttpGet       - HTTP GET request
❌ 619: HttpPost      - HTTP POST request
❌ 620: TcpConnect    - TCP connection
❌ 621: UdpSend       - UDP send
✅ 622: ExportTrace   - Export execution trace
```

#### Misc Operations (700-703) - 4 contracts
**Implemented:** 0/4 (0%)
**Status:** ❌ **ALL MISSING**

```
❌ 700: ScreenCapture - Capture screen
❌ 701: PipeWrite     - Write to pipe
❌ 702: PipeOpen      - Open pipe
❌ 703: Sleep         - Sleep/delay
```

### T3-Parser (Parser/Binary) - 10 contracts
**IDs:** 800-809
**Implemented:** 1/10 (10%)
**Status:** ❌ **MOSTLY MISSING**

```
❌ 800: ParseInt       - Parse integer
❌ 801: ParseFloat     - Parse float
✅ 802: ParseJSON      - Parse JSON (partial)
❌ 803: SerializeJSON  - Serialize JSON
❌ 804: ParseXML       - Parse XML
❌ 805: ParseCSV       - Parse CSV
❌ 806: ParseURL       - Parse URL
❌ 807: FormatString   - Format string
❌ 808: RegexMatch     - Regex matching
❌ 809: RegexReplace   - Regex replacement
```

### T4-GPU (GPU/Vulkan) - 16 contracts
**IDs:** 900-915
**Implemented:** 1/16 (6%)
**Status:** 🔴 **CRITICAL GAP**

```
✅ 900: VulkanShader   - Shader definition (basic)
❌ 901: ComputeKernel  - Compute kernel **MISSING**
❌ 902: PipelineConfig - Pipeline configuration **MISSING**
❌ 906: GEMM           - Matrix multiply **MISSING**
❌ 907: LayerNorm      - Layer normalization **MISSING**
❌ 908: GELU           - GELU activation **MISSING**
❌ 909: Softmax        - Softmax function **MISSING**
❌ 910: CrossEntropy   - Cross-entropy loss **MISSING**
❌ 911: ReLU           - ReLU activation **MISSING**
❌ 912: Sigmoid        - Sigmoid activation **MISSING**
❌ 913: Tanh           - Tanh activation **MISSING**
❌ 914: Dropout        - Dropout regularization **MISSING**
❌ 915: BatchNorm      - Batch normalization **MISSING**
```

**Critical Issue:** Only 1 shader exists (`pointwise_add.comp`)!

---

## Infrastructure Analysis

### Runtime Backend Support

**CPU Backend** (`backends/cpu.rs` - 23KB)
- ✅ Basic tensor storage
- ✅ TensorHandle management
- ✅ Metadata tracking
- ❌ No SIMD optimizations
- ❌ No multi-threading

**Vulkan Backend** (`backends/vulkan.rs` - 28KB)
- ✅ Basic Vulkan setup
- ✅ Buffer management
- ✅ TensorHandle support
- ❌ Only 1 shader implemented
- ❌ No compute pipeline setup
- ❌ No descriptor sets for complex ops

**Shaders** (`backends/vulkan/shaders/`)
- ✅ `pointwise_add.comp` - Element-wise addition (414 bytes)
- ❌ **All other operations missing**

---

## Critical Gaps for High Compute Graphics

### 1. Tensor Operations (ML/AI) - **URGENT**
**Missing:**
- Matrix multiplication (GEMM) - contract 906
- Convolution operations
- Pooling operations (max, avg)
- Activation functions (ReLU, GELU, Sigmoid, Tanh)
- Normalization (LayerNorm, BatchNorm)
- Loss functions (CrossEntropy, MSE)
- Gradient computation
- Backpropagation operations

**Impact:** Cannot run neural networks or ML workloads

### 2. Graphics Shaders - **URGENT**
**Missing:**
- Vertex shaders
- Fragment shaders
- Geometry shaders
- Tessellation shaders
- Ray tracing shaders (ray generation, closest hit, miss)
- Graphics pipeline configuration

**Impact:** Cannot render 3D graphics or do ray tracing

### 3. Compute Shaders - **URGENT**
**Missing:**
- Parallel reduction operations
- Scan/prefix sum
- Histogram computation
- Image processing kernels
- Physics simulations
- Particle systems

**Impact:** Limited GPU utilization

### 4. Image/Texture Operations - **HIGH**
**Missing:**
- Image loading/saving
- Texture creation and binding
- Sampler configuration
- Mipmap generation
- Image format conversions
- Cubemap operations

**Impact:** Cannot work with textures in graphics

### 5. Linear Algebra - **HIGH**
**Missing:**
- Vector operations (dot, cross, norm)
- Matrix operations (transpose, inverse, determinant)
- Matrix decompositions (LU, QR, SVD, Cholesky)
- Eigenvalue/eigenvector computation
- Least squares solving

**Impact:** Limited scientific computing capabilities

### 6. Signal Processing - **MEDIUM**
**Missing:**
- FFT/IFFT
- Convolution (1D, 2D)
- Correlation
- Filtering (low-pass, high-pass, band-pass)
- Windowing functions

**Impact:** Cannot do audio/signal processing

---

## Shader Architecture Gaps

### Current State:
```
backends/vulkan/shaders/
├── pointwise_add.comp  (414 bytes)
└── (that's it!)
```

### Needed Shader Structure:
```
backends/vulkan/shaders/
├── compute/
│   ├── tensor/
│   │   ├── gemm.comp                  # Matrix multiply (CRITICAL)
│   │   ├── gemm_batched.comp          # Batched GEMM
│   │   ├── transpose.comp             # Matrix transpose
│   │   ├── elementwise_ops.comp       # Add, mul, div, etc.
│   │   ├── reduction.comp             # Sum, max, min reduction
│   │   ├── broadcast.comp             # Broadcasting operations
│   │   └── gather_scatter.comp        # Gather/scatter ops
│   ├── activation/
│   │   ├── relu.comp                  # ReLU activation
│   │   ├── gelu.comp                  # GELU activation
│   │   ├── sigmoid.comp               # Sigmoid activation
│   │   ├── tanh.comp                  # Tanh activation
│   │   └── softmax.comp               # Softmax (CRITICAL)
│   ├── normalization/
│   │   ├── layernorm.comp             # Layer normalization
│   │   ├── batchnorm.comp             # Batch normalization
│   │   ├── instancenorm.comp          # Instance normalization
│   │   └── groupnorm.comp             # Group normalization
│   ├── loss/
│   │   ├── cross_entropy.comp         # Cross-entropy loss
│   │   ├── mse.comp                   # Mean squared error
│   │   └── l1.comp                    # L1 loss
│   ├── image/
│   │   ├── conv2d.comp                # 2D convolution
│   │   ├── maxpool.comp               # Max pooling
│   │   ├── avgpool.comp               # Average pooling
│   │   ├── resize.comp                # Image resizing
│   │   ├── blur.comp                  # Gaussian blur
│   │   └── sobel.comp                 # Edge detection
│   └── misc/
│       ├── scan.comp                  # Prefix sum
│       ├── histogram.comp             # Histogram computation
│       └── fft.comp                   # Fast Fourier Transform
├── graphics/
│   ├── vertex/
│   │   ├── basic.vert                 # Basic vertex shader
│   │   ├── skinning.vert              # Skinned mesh
│   │   └── instancing.vert            # Instanced rendering
│   ├── fragment/
│   │   ├── basic.frag                 # Basic fragment shader
│   │   ├── pbr.frag                   # PBR shading
│   │   ├── deferred.frag              # Deferred shading
│   │   └── shadow.frag                # Shadow mapping
│   ├── geometry/
│   │   └── explode.geom               # Geometry shader example
│   └── raytracing/
│       ├── raygen.rgen                # Ray generation
│       ├── closest_hit.rchit          # Closest hit
│       ├── miss.rmiss                 # Miss shader
│       └── shadow.rahit               # Any hit (shadows)
└── util/
    ├── copy.comp                      # Memory copy
    ├── fill.comp                      # Memory fill
    └── clear.comp                     # Clear buffers
```

**Estimated:** ~60-80 shaders needed for comprehensive stdlib

---

## Performance Concerns

### Current `stdlib/math.hlx` Issues:

**1. `floor(x)` and `ceil(x)` - EXTREMELY SLOW**
```hlxa
// This counts up one-by-one until it exceeds x!
loop(int_part <= x, 1000000) {
    if int_part > x {
        return int_part - 1;
    }
    int_part = int_part + 1;
}
```
**Issue:** O(n) complexity for an O(1) operation
**Fix:** Should be a contract that uses CPU/GPU intrinsics

**2. `sqrt(x)` - ACCEPTABLE BUT SLOW**
```hlxa
// Newton's method - good algorithm but interpreted
loop(i < 20, 20) {
    let next = (guess + (x / guess)) / 2;
    // ...
}
```
**Issue:** Loop overhead in interpreted HLX
**Fix:** Should use hardware sqrt instruction

**3. `pow(base, exp)` - LIMITED**
```hlxa
// Only handles integer exponents
loop(i < exp, 1000) {
    result = result * base;
}
```
**Issue:** No support for fractional exponents, slow
**Fix:** Need proper pow implementation (contract 205)

---

## Proposed Expansion Plan

### Phase 1: Critical GPU Operations (Week 1-2)
**Priority:** URGENT
**Goal:** Enable basic ML/graphics workloads

1. **Tensor Operations:**
   - [x] Contract 906: GEMM (matrix multiply) - shader + runtime
   - [ ] Contract 907: LayerNorm - shader + runtime
   - [ ] Contract 908: GELU - shader + runtime
   - [ ] Contract 909: Softmax - shader + runtime
   - [ ] Contract 910: CrossEntropy - shader + runtime
   - [ ] Contract 911: ReLU - shader + runtime

2. **Basic Shaders:**
   - [ ] `gemm.comp` - Tiled matrix multiply (critical bottleneck)
   - [ ] `activation_pack.comp` - ReLU, GELU, Sigmoid, Tanh
   - [ ] `normalization_pack.comp` - LayerNorm, BatchNorm
   - [ ] `softmax.comp` - Numerically stable softmax

3. **Runtime Support:**
   - [ ] Compute pipeline setup
   - [ ] Descriptor set management
   - [ ] Push constants for small params
   - [ ] Double buffering for async execution

**Deliverable:** Run a simple neural network layer on GPU

### Phase 2: Complete stdlib/math (Week 2-3)
**Priority:** HIGH
**Goal:** Professional-grade math library

1. **Implement Missing Math Contracts:**
   - [ ] 213: Sin, 214: Cos, 215: Tan (trigonometry)
   - [ ] 216: Log, 217: Exp (exponentials)
   - [ ] 212: Round (rounding)
   - [ ] 218: Random (RNG)

2. **Optimize Existing:**
   - [ ] Replace `floor()`/`ceil()` with intrinsic contracts
   - [ ] Replace `sqrt()` with hardware accelerated version
   - [ ] Upgrade `pow()` to handle fractional exponents

3. **New stdlib Files:**
   - [ ] `stdlib/trig.hlx` - Trigonometry (sin, cos, tan, asin, acos, atan, atan2)
   - [ ] `stdlib/random.hlx` - RNG (uniform, normal, exponential)
   - [ ] `stdlib/complex.hlx` - Complex numbers

**Deliverable:** Complete, fast math library

### Phase 3: Array & String Operations (Week 3-4)
**Priority:** HIGH
**Goal:** Essential data structure operations

1. **Array Operations (400-414):**
   - [ ] All 15 array contracts implemented
   - [ ] `stdlib/array.hlx` - High-level array utils
   - [ ] GPU-accelerated map/filter/reduce

2. **String Operations (300-314):**
   - [ ] All 15 string contracts implemented
   - [ ] `stdlib/string.hlx` - String manipulation
   - [ ] Unicode support

**Deliverable:** Full data structure manipulation

### Phase 4: Graphics Shaders (Week 4-6)
**Priority:** HIGH
**Goal:** 3D rendering capabilities

1. **Graphics Pipeline:**
   - [ ] Vertex shaders (basic, skinning, instancing)
   - [ ] Fragment shaders (basic, PBR, deferred)
   - [ ] Graphics pipeline configuration contract

2. **Texture Support:**
   - [ ] Texture loading contracts
   - [ ] Sampler configuration
   - [ ] Mipmap generation

3. **New Contracts (T4 expansion):**
   - [ ] 920: VertexShader
   - [ ] 921: FragmentShader
   - [ ] 922: GraphicsPipeline
   - [ ] 923: Texture2D
   - [ ] 924: Sampler

**Deliverable:** Render 3D scenes with PBR

### Phase 5: Advanced Compute (Week 6-8)
**Priority:** MEDIUM
**Goal:** Scientific computing and physics

1. **Linear Algebra:**
   - [ ] Matrix operations (transpose, inverse, determinant)
   - [ ] Vector operations (dot, cross, norm)
   - [ ] `stdlib/linalg.hlx`

2. **Image Processing:**
   - [ ] Convolution, pooling
   - [ ] Filters, transforms
   - [ ] `stdlib/image.hlx`

3. **Signal Processing:**
   - [ ] FFT/IFFT
   - [ ] Correlation, convolution
   - [ ] `stdlib/signal.hlx`

**Deliverable:** Scientific computing toolkit

### Phase 6: I/O & Parsing (Week 8-10)
**Priority:** MEDIUM
**Goal:** Complete I/O and data parsing

1. **File I/O (600-622):**
   - [ ] Implement all missing file operations
   - [ ] Async I/O support

2. **Parsing (800-809):**
   - [ ] JSON, XML, CSV parsers
   - [ ] Regex support
   - [ ] `stdlib/parse.hlx`

3. **Networking:**
   - [ ] HTTP client
   - [ ] TCP/UDP sockets
   - [ ] `stdlib/net.hlx`

**Deliverable:** Full I/O and parsing capabilities

---

## Estimated Effort

**Total Contracts to Implement:** ~100
**Total Shaders to Write:** ~60-80
**Estimated Time:** 8-10 weeks (single developer)
**Priority Breakdown:**
- 🔴 **Critical (P1):** 25 contracts, 15 shaders (2-3 weeks)
- 🟡 **High (P2):** 40 contracts, 20 shaders (3-4 weeks)
- 🟢 **Medium (P3):** 35 contracts, 25 shaders (3-4 weeks)

---

## Recommendations

### Immediate Actions (This Week):

1. **🔴 URGENT: Implement GEMM shader**
   - This is the #1 bottleneck for ML workloads
   - Tiled implementation for cache efficiency
   - Workgroup size optimization

2. **🔴 URGENT: Implement activation functions**
   - ReLU, GELU, Sigmoid, Tanh in one shader
   - Used constantly in neural networks

3. **🔴 URGENT: Fix stdlib/math.hlx performance**
   - Replace floor/ceil with intrinsic contracts
   - Benchmark and optimize hot paths

4. **🟡 HIGH: Add missing math functions**
   - Sin, cos, tan, log, exp
   - Essential for graphics and scientific computing

### Long-term Strategy:

1. **Shader-First Approach:**
   - Write compute shaders for all T4 contracts
   - Prioritize GPU over CPU implementations
   - CPU implementations as fallbacks only

2. **Testing Infrastructure:**
   - Unit tests for every stdlib function
   - Benchmark suite for performance regression
   - Visual tests for graphics shaders

3. **Documentation:**
   - Examples for every contract
   - Performance characteristics
   - GPU memory requirements

4. **Community Contributions:**
   - Open up stdlib for community PRs
   - Shader competition for performance
   - Benchmark leaderboard

---

## Summary

**Current stdlib:** 1 file, ~10 functions
**Contract coverage:** ~25% implemented
**GPU utilization:** <5%

**Opportunity:** 🚀 **MASSIVE**

By implementing the GPU/tensor operations (T4) and filling out the stdlib, HLX could become a **high-performance graphics and ML language** competitive with CUDA/OpenCL/Metal.

The infrastructure is there - we just need to write the shaders and contracts!

---

**Next Steps:** Prioritize GEMM shader implementation and activation functions.
