# HLX Standard Library

Pure HLX implementations of common functionality. Written in HLX, compiled by HLX, proven deterministic.

## Status: Proof of Concept

The stdlib demonstrates that HLX can implement real functionality in itself (dogfooding). Functions here use only the core language features - no Rust runtime calls.

## math.hlxa

Mathematical functions implemented in pure HLX:

### Working:
- `abs(x)` - Absolute value
- `sqrt(x)` - Square root via Newton's method
- `min(a, b)` - Minimum of two numbers
- `max(a, b)` - Maximum of two numbers
- `pow(base, exp)` - Integer exponentiation
- `clamp(x, min, max)` - Clamp value between bounds
- `sign(x)` - Returns -1, 0, or 1
- `lerp(a, b, t)` - Linear interpolation

### Partially Working:
- `floor(x)`, `ceil(x)` - Implemented but inefficient (linear search)

See `examples/showcase_math.hlxa` for working demo.

## Design Philosophy

1. **Pure HLX** - No hidden Rust calls, verifiable by reading source
2. **Deterministic** - Same input → same output, always
3. **Self-contained** - Each function is standalone (no import system yet)
4. **Practical** - Implements what's actually needed, not academic exercises

## Known Limitations

- **No import system yet** - Functions must be copied into user programs
- **Floating point precision** - sqrt() accurate to ~0.0001 due to convergence threshold
- **Inefficient floor/ceil** - Use linear search, need better algorithm

## Next Steps

1. Fix type coercion (enable sqrt, floor, ceil)
2. Implement import system
3. Add string manipulation functions
4. Add collection utilities (sort, filter, map)
5. Add JSON parser/serializer

## Testing

```bash
# Run the showcase
./target/release/hlx run examples/showcase_math.hlxa

# Expected output:
# ✓ abs, min, max, pow, clamp, sign all work
# ✓ All operations deterministic
```

## Contribution

To add a function:
1. Implement in pure HLX (no Rust runtime calls)
2. Add tests in showcase example
3. Verify deterministic output
4. Document limitations if any

**The stdlib is how we prove HLX works.**
