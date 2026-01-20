# HLX C FFI Complete Demo

This demonstrates the full C FFI workflow from HLX source to shared library.

## Step 1: Write HLX Library with FFI Exports

**File: math_lib.hlx**
```hlx
program math_lib {
    #[no_mangle]
    #[export]
    fn add(a: int, b: int) -> int {
        let result = a + b;
        return result;
    }

    #[export]
    fn multiply(x: int, y: int) -> int {
        let product = x * y;
        return product;
    }

    #[export]
    fn power_of_two(n: int) -> int {
        let result = n * n;
        return result;
    }
}
```

## Step 2: Compile to Shared Library

```bash
# Compile to .so on Linux (or .dylib on macOS, .dll on Windows)
hlx compile-native math_lib.hlx --shared -o libmath.so

# Or let it use the default name
hlx compile-native math_lib.hlx --shared
# Creates: math_lib.so
```

## Step 3: Generate C Header

```bash
# First compile to crate to capture metadata
hlx compile math_lib.hlx -o math_lib.hlxl

# Generate header
hlx generate-header math_lib.hlxl -o math_lib.h
```

**Generated math_lib.h:**
```c
#ifndef MATH_LIB_H
#define MATH_LIB_H

#include <stdint.h>
#include <stdbool.h>

#ifdef __cplusplus
extern "C" {
#endif

// HLX FFI Exports
int64_t add(int64_t arg0, int64_t arg1);
int64_t multiply(int64_t arg0, int64_t arg1);
int64_t power_of_two(int64_t arg0);

#ifdef __cplusplus
}
#endif

#endif // MATH_LIB_H
```

## Step 4: Use from C

**File: demo.c**
```c
#include <stdio.h>
#include "math_lib.h"

int main() {
    printf("HLX Shared Library Demo\n\n");

    int64_t sum = add(100, 200);
    printf("add(100, 200) = %ld\n", sum);

    int64_t product = multiply(7, 8);
    printf("multiply(7, 8) = %ld\n", product);

    int64_t squared = power_of_two(12);
    printf("power_of_two(12) = %ld\n", squared);

    return 0;
}
```

## Step 5: Compile and Link

```bash
# Compile C program and link to HLX shared library
gcc demo.c -L. -lmath -o demo

# Run (on Linux, need to set LD_LIBRARY_PATH)
LD_LIBRARY_PATH=. ./demo
```

**Output:**
```
HLX Shared Library Demo

add(100, 200) = 300
multiply(7, 8) = 56
power_of_two(12) = 144
```

## Alternative: Static Linking

```bash
# Compile to object file instead
hlx compile-native math_lib.hlx -o math_lib.o

# Link statically
gcc demo.c math_lib.o -o demo_static

# Run (no LD_LIBRARY_PATH needed)
./demo_static
```

## Features

- ✅ Zero-cost FFI (direct C calling convention)
- ✅ Automatic header generation
- ✅ Platform-specific shared library output (.so/.dylib/.dll)
- ✅ C++ compatibility (extern "C" guards)
- ✅ Works with existing C build systems
- ✅ Deterministic execution guaranteed by HLX

## Type Mapping

| HLX Type | C Type     |
|----------|------------|
| int      | int64_t    |
| i32      | int32_t    |
| float    | float      |
| f64      | double     |
| bool     | bool       |
| [T]      | T*         |

## Attributes

- `#[no_mangle]` - Disable name mangling (use exact function name)
- `#[export]` - Export symbol for C linkage
- Both can be combined for maximum compatibility
