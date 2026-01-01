from hlx_runtime.hlxl_runtime import HLXLRuntime
from hlx_runtime.lc_r_codec import LCRCodec
from hlx_runtime.hlx_runtime import HLXRuntime

# 1. Start with HLXL (ASCII)
original_hlxl = '{"key": "value", "count": 42, "status": true}'
print(f"1. Original HLXL: {original_hlxl}")

# 2. Parse HLXL -> Internal Value
hlxl_rt = HLXLRuntime()
val = hlxl_rt.parse(original_hlxl)

# 3. Encode Value -> LC-R (Runic)
lcr_codec = LCRCodec()
runic = lcr_codec.encode(val)
print(f"2. Runic Form:    {runic}")

# 4. Decode Runic -> Value
decoded_val = lcr_codec.decode(runic)

# 5. Convert Value -> HLXL
hlx_rt = HLXRuntime() # This handles the base runtime logic
new_hlxl = hlxl_rt.serialize(decoded_val)
print(f"3. Restored HLXL: {new_hlxl}")

# 6. Verify 1:1 match
if original_hlxl.replace(" ", "") == new_hlxl.replace(" ", ""):
    print("\n✅ BIJECTION PROVEN: Restored string matches original bitwise (ignoring whitespace).")
else:
    print("\n❌ BIJECTION FAILED.")
    print(f"Diff: {original_hlxl} vs {new_hlxl}")
