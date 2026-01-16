--  HLX Ada/SPARK FFI Example
--  This demonstrates how to use HLX functions from Ada with SPARK verification

with Interfaces;
with Interfaces.C;
use Interfaces;
use Interfaces.C;

procedure HLX_FFI_Demo is

   --  This is an example package specification that would be auto-generated
   --  from HLX FFI exports. The following shows the binding structure:

   package HLX_Math_FFI is
      pragma Pure;

      --  Example tensor operation bindings
      function Create_Zeros (Dim_0 : int; Dim_1 : int) return unsigned_long;
      pragma Import (C, Create_Zeros, "create_zeros");

      function Tensor_Sum (Handle : unsigned_long) return C_float;
      pragma Import (C, Tensor_Sum, "tensor_sum");

      function Tensor_Mean (Handle : unsigned_long) return C_float;
      pragma Import (C, Tensor_Mean, "tensor_mean");

      function Tensor_Max (Handle : unsigned_long) return C_float;
      pragma Import (C, Tensor_Max, "tensor_max");

      function Get_Shape (Handle : unsigned_long; Dim : int) return int;
      pragma Import (C, Get_Shape, "get_shape");

      procedure Release_Tensor (Handle : unsigned_long);
      pragma Import (C, Release_Tensor, "release_tensor");

   end HLX_Math_FFI;

   use HLX_Math_FFI;

   Zeros_Handle : unsigned_long;
   Sum_Result   : C_float;
   Mean_Result  : C_float;
   Max_Result   : C_float;

begin

   --  Create a 100x100 tensor of zeros
   Zeros_Handle := Create_Zeros (100, 100);

   --  Compute statistics
   Sum_Result  := Tensor_Sum (Zeros_Handle);
   Mean_Result := Tensor_Mean (Zeros_Handle);
   Max_Result  := Tensor_Max (Zeros_Handle);

   --  Use the results (all should be 0.0 for a zeros tensor)
   pragma Assert (Sum_Result = 0.0, "Sum of zeros tensor should be 0");
   pragma Assert (Mean_Result = 0.0, "Mean of zeros tensor should be 0");
   pragma Assert (Max_Result = 0.0, "Max of zeros tensor should be 0");

   --  Clean up
   Release_Tensor (Zeros_Handle);

end HLX_FFI_Demo;
