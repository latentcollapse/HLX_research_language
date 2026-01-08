# HLX Quick Start Guide

## Building

```bash
# Debug build (fast compilation, slow runtime)
cargo build

# Release build (slow compilation, fast runtime - USE THIS)
cargo build --release
```

## Using the Compiler

```bash
# Compile a .hlxa file to bytecode
./target/release/hlx compile examples/factorial.hlxa

# This creates factorial.lcc (bytecode)

# Run the bytecode
./target/release/hlx run examples/factorial.lcc

# One-liner (compile + run)
./target/release/hlx compile examples/factorial.hlxa && \
./target/release/hlx run examples/factorial.lcc

# Inspect bytecode
./target/release/hlx inspect examples/factorial.lcc
```

## Performance Tools

```bash
# Quick benchmark
./perf_toolkit.sh quick examples/fibonacci.hlxa

# Full analysis (CPU, memory, cache, everything)
./perf_toolkit.sh all examples/showcase_math.hlxa

# Compare two files
./perf_toolkit.sh compare old.hlxa new.hlxa

# Test all examples
./perf_toolkit.sh regression

# Run stress tests
./stress_test_generator.sh
```

## Common Commands

```bash
# Bootstrap (self-hosting test)
./bootstrap.sh

# Benchmark the bootstrap
./benchmark.sh

# Clean build artifacts
cargo clean
rm -f *.lcc *.lcb perf.data
```

## Example Files

```
examples/
├── axiom_test.hlxa          # Basic language tests
├── factorial.hlxa           # Recursive + iterative factorial
├── fibonacci.hlxa           # Fibonacci sequence
├── fizzbuzz.hlxa            # Classic FizzBuzz
├── primes.hlxa              # Prime number generation
├── showcase_math.hlxa       # Comprehensive math examples
├── test_simple_math.hlxa    # Basic arithmetic
├── test_stdlib.hlxa         # Standard library functions
└── test_tensor.hlxa         # Tensor operations
```

## Language Syntax Cheat Sheet

```hlx
// Comments
// Single line only (for now)

// Program structure
program my_program {
    fn main() {
        // Your code here
    }
}

// Variables
let x = 42;
let name = "HLX";
let arr = [1, 2, 3];
let obj = { "key": "value", "num": 123 };

// Arithmetic (explicit parentheses recommended)
let result = ((x + 5) * 2);
let divided = (x / 3);

// Conditionals
if (x > 10) {
    print("Big!");
} else {
    print("Small!");
}

// Loops (must specify max iterations)
let i = 0;
loop (i < 10, DEFAULT_MAX_ITER()) {
    print(i);
    i = (i + 1);
}

// Functions
fn add(a, b) {
    return (a + b);
}

fn factorial(n) -> Int {  // Type annotations optional
    if (n == 0) { return 1; }
    return (n * factorial((n - 1)));
}

// Built-in functions
print(value);              // Print to stdout
append(array, item);       // Add to array
to_int(value);            // Convert to integer
int_to_string(n);         // Convert int to string

// Control flow
break;                    // Exit loop
continue;                 // Next iteration
return value;             // Return from function
```

## Troubleshooting

### "error: no such subcommand: `compile`"
- Make sure you built the release version: `cargo build --release`
- Use `./target/release/hlx` not `./target/debug/hlx`

### "Parse error"
- Check for missing semicolons (`;`)
- Use explicit parentheses in complex expressions: `(a + (b * c))`
- Make sure loops have max iteration count: `loop (cond, DEFAULT_MAX_ITER())`

### "Slow compilation"
- Run stress tests to find bottlenecks: `./stress_test_generator.sh`
- Profile the slow file: `./perf_toolkit.sh profile your_file.hlxa`
- Check for O(n²) patterns (see PERFORMANCE_GUIDE.md)

### "Runtime error: stack overflow"
- Reduce recursion depth
- Use iterative loops instead of recursion
- Check for infinite loops

### "Permission denied" on scripts
```bash
chmod +x perf_toolkit.sh
chmod +x stress_test_generator.sh
chmod +x bootstrap.sh
chmod +x benchmark.sh
```

## Learning Path

1. **Start here**: Read `examples/axiom_test.hlxa`
2. **Basic examples**: `factorial.hlxa`, `fibonacci.hlxa`
3. **Complex example**: `showcase_math.hlxa`
4. **Real application**: `HLX_Training_Examples/Synthetic_Vault/scripts/generator.hlxa`
5. **Performance tuning**: Read `PERFORMANCE_GUIDE.md`
6. **Deep dive**: Read `HLX_SOLIDIFIED.md`

## Getting Help

- **Language docs**: Check `examples/` for working code
- **Performance issues**: Read `PERFORMANCE_GUIDE.md`
- **Compiler internals**: Read `BUILD_SUMMARY.md`
- **Benchmarking**: Read `BENCHMARKS.md`

## Contributing

Before making changes:
1. Run `./perf_toolkit.sh regression` to save baseline
2. Make your changes
3. Run `./perf_toolkit.sh regression` again
4. Compare results (should not regress >10%)

Include in PR:
- Before/after benchmarks
- Flamegraphs if performance-related
- Updated examples if language changes

---

**HLX is production-ready. Build cool shit.** 🚀
