#!/usr/bin/env python3
"""Test HLX Python FFI wrapper"""

import hlx_math

print("Testing HLX Python FFI\n")

# Test add function
result = hlx_math.add(100, 50)
print(f"add(100, 50) = {result}")

# Test multiply function
result = hlx_math.multiply(7, 9)
print(f"multiply(7, 9) = {result}")

# Test composition
x = hlx_math.add(10, 20)
y = hlx_math.multiply(x, 2)
print(f"multiply(add(10, 20), 2) = {y}")

# Test with type checking
result: int = hlx_math.add(1000, 2000)
print(f"add(1000, 2000) = {result}")

print("\nAll tests passed!")
