
import unittest
import sys
import os

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from hlx_runtime.contracts import validate_contract, CONTRACT_IDS
from hlx_runtime.ls_ops import collapse, resolve
from hlx_runtime.errors import ContractError

class TestEmpireExtensions(unittest.TestCase):
    def test_vulkan_shader_contract(self):
        # Dummy SPIR-V (just random bytes for transport test)
        spirv_mock = b'\x03\x02\x23\x07' 
        
        payload = {
            'spirv_binary': spirv_mock,
            'entry_point': "main",
            'shader_stage': "compute",
            'descriptor_bindings': ["&h_buf_in", "&h_buf_out"]
        }
        
        # 1. Validate (Direct)
        # Note: validate_contract expects (id, inputs) or ({id: inputs})
        # It returns inputs if valid.
        res = validate_contract(CONTRACT_IDS['VULKAN_SHADER'], payload)
        self.assertEqual(res, payload)
        
        # 2. Collapse (Store in CAS)
        # We need to wrap it in the contract structure for collapse?
        # ls.collapse takes "Any". If we pass a dict { "900": payload }, it's a contract.
        wrapped = { str(CONTRACT_IDS['VULKAN_SHADER']): payload }
        handle = collapse(wrapped)
        
        # 3. Resolve
        resolved = resolve(handle)
        self.assertEqual(resolved, wrapped)
        
    def test_compute_kernel_contract(self):
        payload = {
            'kernel_name': "matrix_mul",
            'shader_handle': "&h_shader_123",
            'workgroup_size': [16, 16, 1],
            'shared_memory_bytes': 1024,
            'push_constants_layout': "float,int"
        }
        
        wrapped = { str(CONTRACT_IDS['COMPUTE_KERNEL']): payload }
        # Validate via collapse (which should ideally validate? No, collapse just stores.
        # But let's validate explicitly first)
        validate_contract(wrapped)
        
        handle = collapse(wrapped)
        resolved = resolve(handle)
        self.assertEqual(resolved['901']['kernel_name'], "matrix_mul")

    def test_pipeline_contract(self):
        payload = {
            'pipeline_id': "post_process_v1",
            'stages': ["&h_kernel_1", "&h_kernel_2"],
            'sync_barriers': [{'stage_idx': 0, 'memory_scope': "workgroup"}],
            'output_image': "&h_img_final"
        }
        
        wrapped = { str(CONTRACT_IDS['PIPELINE_CONFIG']): payload }
        validate_contract(wrapped)
        
        handle = collapse(wrapped)
        resolved = resolve(handle)
        self.assertEqual(len(resolved['902']['stages']), 2)

    def test_invalid_shader(self):
        # Missing field
        payload = {
            'spirv_binary': b'',
            # Missing entry_point
            'shader_stage': "compute",
            'descriptor_bindings': []
        }
        
        with self.assertRaises(ContractError):
            validate_contract(CONTRACT_IDS['VULKAN_SHADER'], payload)

if __name__ == '__main__':
    unittest.main()
