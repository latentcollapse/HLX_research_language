"""Python wrapper for test_ffi_lib HLX library

Auto-generated FFI bindings using ctypes.
"""

import ctypes
import os
import sys
from pathlib import Path

# Load shared library with platform-specific naming
def _load_library():
    """Load the HLX shared library with platform-specific extensions"""
    lib_dir = Path(__file__).parent
    
    if sys.platform == 'linux':
        lib_path = lib_dir / 'libtest_ffi_lib.so'
    elif sys.platform == 'darwin':
        lib_path = lib_dir / 'libtest_ffi_lib.dylib'
    elif sys.platform == 'win32':
        lib_path = lib_dir / 'test_ffi_lib.dll'
    else:
        raise RuntimeError(f'Unsupported platform: {sys.platform}')
    
    if not lib_path.exists():
        raise FileNotFoundError(f'HLX library not found: {lib_path}')
    
    return ctypes.CDLL(str(lib_path))

_lib = _load_library()

# Configure function signatures
_lib.add.argtypes = [ctypes.c_int64, ctypes.c_int64]
_lib.add.restype = ctypes.c_int64
_lib.multiply.argtypes = [ctypes.c_int64, ctypes.c_int64]
_lib.multiply.restype = ctypes.c_int64

# Python wrapper functions
def add(arg0: int, arg1: int) -> int:
    """Call HLX function: add"""
    return _lib.add(arg0, arg1)

def multiply(arg0: int, arg1: int) -> int:
    """Call HLX function: multiply"""
    return _lib.multiply(arg0, arg1)

__all__ = ["multiply", "add"]
