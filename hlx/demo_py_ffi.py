#!/usr/bin/env python3
"""
Demonstration of HLX Python FFI

This script shows how to use HLX compiled functions from Python
with full type hints and Python-native interface.
"""

import hlx_math

def main():
    print("=" * 60)
    print("HLX Python FFI Demo")
    print("=" * 60)
    print()

    # Simple arithmetic
    print("Basic Operations:")
    a, b = 123, 456
    sum_result = hlx_math.add(a, b)
    print(f"  {a} + {b} = {sum_result}")

    product = hlx_math.multiply(10, 10)
    print(f"  10 × 10 = {product}")
    print()

    # Function composition
    print("Function Composition:")
    x = hlx_math.add(50, 50)
    y = hlx_math.multiply(x, 3)
    print(f"  multiply(add(50, 50), 3) = {y}")
    print()

    # More complex chain
    print("Complex Chain:")
    step1 = hlx_math.add(10, 20)        # 30
    step2 = hlx_math.multiply(step1, 2)  # 60
    step3 = hlx_math.add(step2, 15)      # 75
    final = hlx_math.multiply(step3, 2)  # 150
    print(f"  Start: 10, 20")
    print(f"  add(10, 20) = {step1}")
    print(f"  multiply({step1}, 2) = {step2}")
    print(f"  add({step2}, 15) = {step3}")
    print(f"  multiply({step3}, 2) = {final}")
    print()

    # Performance test
    print("Performance Test (10,000 calls):")
    import time
    start = time.time()
    for i in range(10000):
        _ = hlx_math.add(i, i + 1)
        _ = hlx_math.multiply(i, 2)
    elapsed = time.time() - start
    print(f"  Time: {elapsed:.4f}s ({20000/elapsed:.0f} calls/sec)")
    print()

    print("=" * 60)
    print("Success! HLX functions work seamlessly in Python.")
    print("=" * 60)

if __name__ == "__main__":
    main()
