import ctypes, json, os, struct

class HlxRuntime:
    def __init__(self, lib_path="libhlx.so"):
        # Resolve lib_path relative to this script if it's not absolute
        if not os.path.isabs(lib_path):
            base_dir = os.path.dirname(os.path.abspath(__file__))
            # Check for libhlx.so in various places
            search_paths = [
                lib_path,
                os.path.join(base_dir, "..", "target", "release", lib_path),
                os.path.join(base_dir, "..", "target", "debug", lib_path),
                os.path.join(base_dir, lib_path)
            ]
            for p in search_paths:
                if os.path.exists(p):
                    lib_path = p
                    break

        self.lib = ctypes.CDLL(lib_path)
        
        self.lib.hlx_open.restype = ctypes.c_void_p
        
        self.lib.hlx_close.argtypes = [ctypes.c_void_p]
        
        self.lib.hlx_compile_source.restype = ctypes.c_int
        self.lib.hlx_compile_source.argtypes = [ctypes.c_void_p, ctypes.c_char_p]
        
        self.lib.hlx_call.restype = ctypes.c_void_p  # Return void_p so we can free it
        self.lib.hlx_call.argtypes = [ctypes.c_void_p, ctypes.c_char_p, ctypes.c_char_p]
        
        self.lib.hlx_free_string.argtypes = [ctypes.c_void_p]
        
        self.lib.hlx_errmsg.restype = ctypes.c_char_p
        self.lib.hlx_errmsg.argtypes = [ctypes.c_void_p]
        
        self.lib.hlx_reset.restype = ctypes.c_int
        self.lib.hlx_reset.argtypes = [ctypes.c_void_p]
        
        self.handle = self.lib.hlx_open()

    def compile_source(self, source: str):
        if not self.lib.hlx_compile_source(self.handle, source.encode()):
            err = self.lib.hlx_errmsg(self.handle)
            raise Exception(err.decode() if err else "Unknown compilation error")

    def call(self, func: str, *args):
        hlx_args = []
        for arg in args:
            if isinstance(arg, bool): hlx_args.append({"type":"Bool","value":arg})
            elif isinstance(arg, int): hlx_args.append({"type":"I64","value":arg})
            elif isinstance(arg, float): hlx_args.append({"type":"F64","value":arg})
            elif isinstance(arg, str): hlx_args.append({"type":"String","value":arg})
            elif isinstance(arg, (list, tuple)): hlx_args.append({"type":"Array","value":list(arg)})
            elif arg is None: hlx_args.append({"type":"Nil"})
            else: raise ValueError(f"Unsupported argument type: {type(arg)}")
        
        args_json = json.dumps(hlx_args).encode()
        res_ptr = self.lib.hlx_call(self.handle, func.encode(), args_json)
        
        if not res_ptr:
            err = self.lib.hlx_errmsg(self.handle)
            raise Exception(err.decode() if err else "Call failed")

        try:
            res_json = ctypes.string_at(res_ptr).decode()
            val = json.loads(res_json)
        finally:
            self.lib.hlx_free_string(res_ptr)

        # Handle the tagged union format {"type":"...", "value":...}
        if isinstance(val, dict) and "value" in val:
            return val["value"]
        elif isinstance(val, dict) and "type" in val:
            # For variants like Void or Nil that might not have a "value" key
            return None
        return val

    def reset(self):
        """Reset VM to fresh state, discarding all persistent memory."""
        if self.handle:
            self.lib.hlx_reset(self.handle)

    def close(self):
        if self.handle:
            self.lib.hlx_close(self.handle)
            self.handle = None
            
    # ── Binary ABI (Phase 15: Zero-Copy) ──────────────────────────────────

    def _init_binary_ffi(self):
        """Lazily initialize binary ABI function signatures."""
        if hasattr(self, '_binary_ffi_ready'):
            return
        self.lib.hlx_call_binary.restype = ctypes.POINTER(ctypes.c_uint8)
        self.lib.hlx_call_binary.argtypes = [
            ctypes.c_void_p, ctypes.c_char_p,
            ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t),
        ]
        self.lib.hlx_free_binary.argtypes = [ctypes.POINTER(ctypes.c_uint8), ctypes.c_size_t]

        self.lib.hlx_tensor_create_from_ptr.restype = ctypes.c_void_p
        self.lib.hlx_tensor_create_from_ptr.argtypes = [
            ctypes.POINTER(ctypes.c_double), ctypes.c_size_t,
            ctypes.POINTER(ctypes.c_size_t), ctypes.c_size_t,
        ]
        self.lib.hlx_tensor_get_data.restype = ctypes.POINTER(ctypes.c_double)
        self.lib.hlx_tensor_get_data.argtypes = [ctypes.c_void_p]
        self.lib.hlx_tensor_get_data_len.restype = ctypes.c_size_t
        self.lib.hlx_tensor_get_data_len.argtypes = [ctypes.c_void_p]
        self.lib.hlx_tensor_get_shape.restype = ctypes.POINTER(ctypes.c_size_t)
        self.lib.hlx_tensor_get_shape.argtypes = [ctypes.c_void_p]
        self.lib.hlx_tensor_get_ndim.restype = ctypes.c_size_t
        self.lib.hlx_tensor_get_ndim.argtypes = [ctypes.c_void_p]
        self.lib.hlx_tensor_free.argtypes = [ctypes.c_void_p]
        self._binary_ffi_ready = True

    @staticmethod
    def _binary_encode(value):
        """Encode a Python value into the HLX binary wire format."""
        if value is None:
            return struct.pack('<II', 0, 0)  # nil
        elif isinstance(value, bool):
            return struct.pack('<IIB', 7, 1, 1 if value else 0)
        elif isinstance(value, int):
            return struct.pack('<II', 1, 8) + struct.pack('<q', value)
        elif isinstance(value, float):
            return struct.pack('<II', 2, 8) + struct.pack('<d', value)
        elif isinstance(value, str):
            b = value.encode('utf-8')
            return struct.pack('<II', 3, len(b)) + b
        else:
            raise ValueError(f"Unsupported binary type: {type(value)}")

    @staticmethod
    def _binary_decode(data, offset=0):
        """Decode one value from binary wire format. Returns (value, new_offset)."""
        tag, dlen = struct.unpack_from('<II', data, offset)
        payload = data[offset + 8:offset + 8 + dlen]
        if tag == 0:
            return None, offset + 8
        elif tag == 1:
            return struct.unpack('<q', payload)[0], offset + 8 + dlen
        elif tag == 2:
            return struct.unpack('<d', payload)[0], offset + 8 + dlen
        elif tag == 3:
            return payload.decode('utf-8'), offset + 8 + dlen
        elif tag == 7:
            return payload[0] != 0, offset + 8 + dlen
        else:
            raise ValueError(f"Unknown binary tag: {tag}")

    def call_binary(self, func: str, *args):
        """Call a function using the binary ABI (faster than JSON for primitives)."""
        self._init_binary_ffi()

        # Encode args
        buf = b''.join(self._binary_encode(a) for a in args)
        args_arr = (ctypes.c_uint8 * len(buf))(*buf) if buf else None
        out_len = ctypes.c_size_t(0)

        result_ptr = self.lib.hlx_call_binary(
            self.handle, func.encode(),
            args_arr, len(buf) if buf else 0,
            ctypes.byref(out_len),
        )

        if not result_ptr:
            err = self.lib.hlx_errmsg(self.handle)
            raise Exception(err.decode() if err else "Binary call failed")

        try:
            result_data = bytes(result_ptr[:out_len.value])
            val, _ = self._binary_decode(result_data)
            return val
        finally:
            self.lib.hlx_free_binary(result_ptr, out_len.value)

    def create_tensor(self, data: list, shape: list):
        """Create a tensor handle from Python lists. Returns an opaque handle."""
        self._init_binary_ffi()
        c_data = (ctypes.c_double * len(data))(*data)
        c_shape = (ctypes.c_size_t * len(shape))(*shape)
        handle = self.lib.hlx_tensor_create_from_ptr(c_data, len(data), c_shape, len(shape))
        if not handle:
            raise ValueError("Failed to create tensor (shape/data mismatch?)")
        return handle

    def read_tensor(self, tensor_handle):
        """Read tensor data and shape from a handle. Returns (data_list, shape_list)."""
        self._init_binary_ffi()
        data_len = self.lib.hlx_tensor_get_data_len(tensor_handle)
        ndim = self.lib.hlx_tensor_get_ndim(tensor_handle)
        data_ptr = self.lib.hlx_tensor_get_data(tensor_handle)
        shape_ptr = self.lib.hlx_tensor_get_shape(tensor_handle)
        data = [data_ptr[i] for i in range(data_len)]
        shape = [shape_ptr[i] for i in range(ndim)]
        return data, shape

    def free_tensor(self, tensor_handle):
        """Free a tensor handle created by create_tensor."""
        self._init_binary_ffi()
        self.lib.hlx_tensor_free(tensor_handle)

    def __enter__(self): return self
    def __exit__(self, *args): self.close()
    def __del__(self): self.close()
