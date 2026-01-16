# Ada/SPARK FFI Support

HLX now provides comprehensive Ada and SPARK binding generation for seamless integration with Ada-based systems, including formal verification support through SPARK.

## Overview

The Ada/SPARK FFI layer enables:

1. **Type-Safe Bindings**: Automatic conversion of HLX types to Ada types with proper C interoperability
2. **SPARK Contracts**: Auto-generated package specifications with SPARK annotations for formal verification
3. **Project Configuration**: Pre-configured GNAT project files with SPARK proof settings
4. **Pure Packages**: Generated Ada packages marked as `pragma Pure` for safety

## Generated Artifacts

When exporting HLX functions for Ada/SPARK use, three files are automatically generated:

### 1. Package Specification (`.ads`)

```ada
package Module_Name_FFI is
   pragma Pure;

   use Interfaces;
   use Interfaces.C;

   --  @pre True
   --  @post True
   function tensor_max (Arg_0 : Interfaces.C.long) return Interfaces.C.C_float with
     Import        => True,
     Convention    => C,
     External_Name => "tensor_max";

end Module_Name_FFI;
```

**Features:**
- SPARK contract annotations (preconditions and postconditions)
- C calling convention declarations
- Proper type mappings via `Interfaces.C`
- `pragma Pure` ensures no side effects

### 2. Package Body (`.adb`)

```ada
package body Module_Name_FFI is
   --  This package body is intentionally empty.
   --  All function implementations are provided by the external C library.
end Module_Name_FFI;
```

The body is empty because implementations come from the HLX compiled C library.

### 3. GNAT Project File (`project_spark_hlx.gpr`)

```ada
project SPARK_HLX is

   for Source_Dirs use ("src");
   for Object_Dir use ".objects";
   for Library_Dir use ".";

   package Compiler is
      for Default_Switches ("Ada") use
        ("-gnat2022",
         "-gnatwa",
         "-gnatwe",
         "-gnatyyM",
         "-gnaty3abdefhijklmnoprstux",
         "-gnaty_",
         "-gnatf",
         "-gnata");
   end Compiler;

   package Prove is
      for Switches use ("--level=1", "--proof=progressive");
   end Prove;

end SPARK_HLX;
```

## Type Mapping

HLX types are automatically mapped to Ada/C interop types:

| HLX Type | Ada Type | Notes |
|----------|----------|-------|
| `i32` | `Interfaces.C.int` | 32-bit signed integer |
| `i64` | `Interfaces.C.long` | 64-bit signed integer |
| `f32` | `Interfaces.C.C_float` | 32-bit floating point |
| `f64` | `Interfaces.C.double` | 64-bit floating point |
| `bool` | `Interfaces.C.unsigned_char` | C-compatible boolean |
| Arrays | `Interfaces.C.Pointers.Pointer_To_*` | Access types for arrays |

## Usage Example

### HLX Library Definition

```hlx
program tensor_math {
    fn add(a: f64, b: f64) -> f64 {
        return a + b;
    }

    #[export]
    fn exported_add(a: f64, b: f64) -> f64 {
        return add(a, b);
    }
}
```

### Generated Ada Binding

```ada
package Tensor_Math_FFI is
   pragma Pure;

   use Interfaces;
   use Interfaces.C;

   --  @pre True
   --  @post True
   function exported_add (Arg_0 : Interfaces.C.double; Arg_1 : Interfaces.C.double) return Interfaces.C.double with
     Import        => True,
     Convention    => C,
     External_Name => "exported_add";

end Tensor_Math_FFI;
```

### Ada Client Code

```ada
with Tensor_Math_FFI;
use Tensor_Math_FFI;

procedure Test_Math is
   Result : Interfaces.C.double;
begin
   Result := exported_add (2.5, 3.7);
   -- Result = 6.2
end Test_Math;
```

## SPARK Verification

The generated GNAT project includes SPARK proof settings:

```bash
# Run SPARK proof checks
gprbuild -Pproject_spark_hlx -P src

# Generate proof report
spark2014 -P project_spark_hlx
```

### SPARK Proof Levels

The default configuration uses:
- **Proof Level**: `--level=1` (basic verification)
- **Proof Mode**: `--proof=progressive` (incremental checking)

For stricter verification, modify `project_spark_hlx.gpr`:

```ada
package Prove is
   for Switches use ("--level=2", "--proof=all");  -- Full formal verification
end Prove;
```

## Building Ada Projects with HLX Bindings

### Project Structure

```
ada_project/
├── src/
│   ├── tensor_math_ffi.ads       (Auto-generated)
│   ├── tensor_math_ffi.adb       (Auto-generated)
│   └── main.ads
├── obj/
├── project_spark_hlx.gpr         (Auto-generated)
└── Makefile
```

### Makefile Example

```makefile
GNATFLAGS = -gnat2022 -gnatwa -gnatwe -gnata
SPARK_FLAGS = --level=1 --proof=progressive

build:
	gprbuild -Pproject_spark_hlx $(GNATFLAGS)

prove:
	spark2014 -Pproject_spark_hlx $(SPARK_FLAGS)

clean:
	rm -rf obj .objects *.ali *.o
```

### Linking HLX Library

Ensure the HLX shared library is in your library search path:

```bash
# Linux
export LD_LIBRARY_PATH=/path/to/hlx/lib:$LD_LIBRARY_PATH

# macOS
export DYLD_LIBRARY_PATH=/path/to/hlx/lib:$DYLD_LIBRARY_PATH

# Build
gprbuild -Pproject_spark_hlx -largs -L/path/to/hlx/lib
```

## Memory Safety Guarantees

Ada/SPARK bindings provide several safety properties:

1. **Type Safety**: All calls are type-checked at compile time
2. **Storage Checking**: No dangling pointers (Ada manages memory)
3. **Range Checking**: Ada's range semantics prevent overflow
4. **Proof Annotations**: SPARK contracts document preconditions/postconditions

## Advanced: Custom SPARK Contracts

For critical functions, you can enhance auto-generated contracts:

```ada
--  @pre Arg_0 > 0  -- tensor handle must be valid
--  @post Result >= 0.0  -- sum of non-negative values
function tensor_max (Arg_0 : Interfaces.C.long) return Interfaces.C.C_float with
  Import        => True,
  Convention    => C,
  External_Name => "tensor_max";
```

## Limitations and Considerations

1. **Array Handling**: Pointer-based arrays require careful lifetime management
2. **Callbacks**: C callbacks to Ada code require additional wrapper layers
3. **Performance**: FFI calls have small overhead; batch operations when possible
4. **Proof Scope**: SPARK contracts verify Ada code only; C implementation is trusted

## Related Documentation

- [GNAT User's Guide](https://gcc.gnu.org/onlinedocs/gnat_ugn/)
- [SPARK 2014 Manual](https://docs.adacore.com/spark2014-docs/)
- [Interfaces.C Documentation](https://gcc.gnu.org/onlinedocs/gnat_rm/Interfacing-to-C.html)

## Example: Tensor Operations with SPARK

See `examples/ada_ffi_example.ada` for a complete example demonstrating:
- Tensor creation
- Statistical operations (sum, mean, max)
- Assertions with SPARK contracts
- Proper resource cleanup
