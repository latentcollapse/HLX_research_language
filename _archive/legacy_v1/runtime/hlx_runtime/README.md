# HLX Runtime Package v2.0.0

Complete implementation of the HLX language family with wire formats and execution runtimes.

## Overview

This package provides the complete HLX ecosystem:

### Language Runtimes

**HLX (Runic)**
- `HLXBasicRuntime` - Basic Runic interpreter (no Latent Space operations)
- `HLXRuntime` - Full Runic interpreter with Latent Space support

**HLXL (ASCII)**
- `HLXLBasicRuntime` - Basic ASCII interpreter (no Latent Space operations)
- `HLXLRuntime` - Full ASCII interpreter with Latent Space support

### Wire Formats

**LC-B (Binary)** - Compact binary encoding
- `encode_lcb()` / `decode_lcb()`
- Deterministic, bijective
- Smallest size

**LC-R (Runic)** - Unicode glyph encoding
- `encode_lcr()` / `decode_lcr()`
- Beautiful, symbolic
- Human-inspectable

**LC-T (Text)** - ASCII-safe text encoding
- `encode_lct_new()` / `decode_lct()`
- No Unicode required
- Terminal-friendly

### Core Infrastructure

- **CAS** - Content-Addressed Storage
- **Contracts** - Contract validation and wrapping
- **LS Operations** - Collapse, resolve, snapshot
- **Glyphs** - Complete Unicode glyph definitions

## Installation

```python
# Add to Python path
import sys
sys.path.append('/path/to/hlx-dev-studio/frontend')

from hlx_runtime import HLXLRuntime, encode_lcb
```

## Quick Start

### HLXL (ASCII) - Recommended for General Use

```python
from hlx_runtime import HLXLBasicRuntime

runtime = HLXLRuntime()

# Literals
runtime.execute('42')  # => 42
runtime.execute('"hello"')  # => "hello"
runtime.execute('true')  # => True

# Variables
runtime.execute('let x = 42')
runtime.execute('let y = x + 10')
print(runtime.get_var('y'))  # => 52

# Arrays
runtime.execute('[1, 2, 3]')  # => [1, 2, 3]

# Objects
runtime.execute('{ name: "Alice", age: 30 }')
# => {'name': 'Alice', 'age': 30}

# Functions
runtime.execute('print("Hello, World!")')  # Prints to console
```

### HLX (Runic) - Compact Glyph Notation

```python
from hlx_runtime import HLXBasicRuntime

runtime = HLXBasicRuntime()

# Literals using glyphs
runtime.execute('∅')        # null
runtime.execute('⊤')        # true
runtime.execute('⊥')        # false
runtime.execute('🜃42')     # integer
runtime.execute('🜄3.14')   # float
runtime.execute('᛭"text"')  # string
```

### Latent Space Operations

```python
from hlx_runtime import HLXLRuntime

# HLXL-LS (ASCII with Latent Space)
runtime = HLXLRuntime()

# Collapse a value into CAS
handle = runtime.execute('ls.collapse(42)')
print(handle)  # => h:<sha256-hash>

# Resolve it back
runtime.set_var('h', handle)
value = runtime.execute('ls.resolve(h)')
print(value)  # => 42
```

### Wire Format Encoding

```python
from hlx_runtime import encode_lcb, decode_lcb, encode_lcr, decode_lcr

# LC-B: Binary encoding (compact)
data = {'x': 42, 'y': [1, 2, 3]}
binary = encode_lcb(data)
print(len(binary))  # Compact size

# LC-R: Runic encoding (beautiful)
runic = encode_lcr(data)
print(runic)  # Unicode glyphs

# Decode
decoded = decode_lcb(binary)
print(decoded)  # => {'x': 42, 'y': [1, 2, 3]}
```

## Architecture

```
┌─────────────────────────────────────────┐
│         User Programs (HLXL)            │
│  Human-readable ASCII language          │
└─────────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────┐
│      Runtime Execution Layer            │
│  • Tokenizer → Parser → Evaluator      │
│  • AST-based interpretation             │
│  • Built-in functions                   │
└─────────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────┐
│         Data Layer (HLX)                │
│  Compact glyph representation           │
│  Latent Space operations                │
└─────────────────────────────────────────┘
                 │
                 ↓
┌─────────────────────────────────────────┐
│      Wire Formats (LC-*)                │
│  • LC-B: Binary (compact)               │
│  • LC-R: Runic (beautiful)              │
│  • LC-T: Text (ASCII-safe)              │
└─────────────────────────────────────────┘
```

## File Structure

```
hlx_runtime/
├── __init__.py              # Main package exports
├── README.md                # This file
│
├── hlx_runtime.py           # Basic HLX (Runic) runtime
├── hlx_ls_runtime.py        # HLX-LS runtime with Latent Space
├── hlxl_runtime.py          # Basic HLXL (ASCII) runtime
├── hlxl_ls_runtime.py       # HLXL-LS runtime with Latent Space
│
├── lc_codec.py              # LC-B binary codec
├── lc_r_codec.py            # LC-R runic codec
├── lc_t_codec.py            # LC-T text codec
│
├── cas.py                   # Content-Addressed Storage
├── contracts.py             # Contract validation
├── errors.py                # Error definitions
├── glyphs.py                # Unicode glyph definitions
├── ls_ops.py                # Latent Space operations
├── pre_serialize.py         # Pre-serialization utilities
├── tables.py                # Merkle trees and state tables
│
├── cli.py                   # CLI tools
└── tests/                   # Test suite
    ├── test_hlx_runtime.py
    ├── test_hlx_ls_runtime.py
    ├── test_hlxl_runtime.py
    ├── test_hlxl_ls_runtime.py
    ├── test_lc_t.py
    └── ... (more tests)
```

## API Reference

### Runtimes

**HLXBasicRuntime()** - Basic Runic interpreter
- `.execute(source: str) -> Any` - Execute HLX code
- `.set_var(name: str, value: Any)` - Set variable
- `.get_var(name: str) -> Any` - Get variable
- `.get_env() -> Dict` - Get all variables

**HLXRuntime()** - Runic with Latent Space
- Same as HLXBasicRuntime, plus:
- Supports `⊕` (collapse) and `⊖` (resolve) glyphs
- Integrated CAS for handle storage

**HLXLBasicRuntime()** - Basic ASCII interpreter
- `.execute(source: str) -> Any` - Execute HLXL code
- `.set_var(name: str, value: Any)` - Set variable
- `.get_var(name: str) -> Any` - Get variable
- `.get_env() -> Dict` - Get all variables

**HLXLRuntime()** - ASCII with Latent Space
- Same as HLXLBasicRuntime, plus:
- Supports `ls.collapse()` and `ls.resolve()`
- Integrated CAS for handle storage

### Wire Formats

**encode_lcb(value: Any) -> bytes** - Encode to LC-B binary
**decode_lcb(data: bytes) -> Any** - Decode from LC-B binary

**encode_lcr(value: Any) -> str** - Encode to LC-R runic glyphs
**decode_lcr(text: str) -> Any** - Decode from LC-R runic glyphs

**encode_lct_new(value: Any) -> str** - Encode to LC-T ASCII text
**decode_lct(text: str) -> Any** - Decode from LC-T ASCII text

### CAS Operations

**get_cas_store() -> CASStore** - Get global CAS instance

**collapse(value: Any) -> str** - Store value in CAS, return handle

**resolve(handle: str) -> Any** - Retrieve value from CAS

**snapshot() -> Dict** - Get CAS snapshot (for debugging)

## Testing

Run the test suite:

```bash
cd /home/matt/hlx-dev-studio/frontend
python3 -m pytest hlx_runtime/tests/ -v
```

Run specific tests:

```bash
python3 -m pytest hlx_runtime/tests/test_hlxl_runtime.py -v
python3 -m pytest hlx_runtime/tests/test_hlx_runtime.py -v
```

## Performance

Current performance (interpreter-based):
- HLXL execution: ~10-100k ops/sec
- HLX execution: ~10-100k ops/sec
- LC-B encoding: ~1-10 MB/sec
- LC-R encoding: ~1-10 MB/sec

Future optimizations planned:
- Bytecode compilation (10x speedup)
- JIT compilation (50-100x speedup)
- GPU compilation for parallelizable code

## Determinism

All operations are 100% deterministic:
- Same input → Same output (always)
- Reproducible across platforms
- Bit-exact floating point (IEEE 754)
- No hidden state or randomness

This makes HLX ideal for:
- Scientific computing (reproducibility)
- Financial systems (auditable)
- Distributed systems (consensus)
- ML training (reproducible experiments)

## Version History

**v2.0.0** (Dec 26, 2025) - Complete Runtime Package
- Added all 4 runtimes (HLX, HLX-LS, HLXL, HLXL-LS)
- Added LC-T wire format
- Unified package structure
- Comprehensive test suite

**v1.1.0** (Dec 24, 2025) - LC-R Support
- Added LC-R runic codec
- Added glyph definitions
- Beautiful Unicode output

**v1.0.0** (Dec 18, 2025) - Initial Release
- LC-B binary codec
- CAS implementation
- Contract system

## License

Proprietary - Part of HLX Development Studio

## Support

For issues, questions, or contributions, see the main HLX repository.

---

**Built with ❤️ for deterministic, reproducible computation**
