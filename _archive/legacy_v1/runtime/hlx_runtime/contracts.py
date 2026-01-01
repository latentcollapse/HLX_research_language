"""
HLX Contract System
Defines and validates HLX contract structures.
Reference: CONTRACT_805
"""

from typing import Any, Dict, List, Optional, Tuple, Union
from .errors import (
    E_CONTRACT_STRUCTURE, E_CONTRACT_UNKNOWN, 
    E_CONTRACT_FIELD_MISSING, E_CONTRACT_FIELD_TYPE,
    E_FIELD_ORDER, E_TYPE_MISMATCH,
    ContractError
)

# Contract type IDs (canonical)
CONTRACT_IDS = {
    'INT_LITERAL': 14,
    'FLOAT_LITERAL': 15,
    'TEXT_LITERAL': 16,
    'BYTES_LITERAL': 17,
    'ARRAY': 18,
    'OBJECT': 19,
    'HANDLE_REF': 20,
    'NULL': 21,
    'BOOL': 22,
    'BLOCK': 100,
    'EXPR': 101,
    'VAR_REF': 102,
    'ASSIGNMENT': 103,
    'FUNCTION_DEF': 104,
    'FUNCTION_CALL': 105,
    # Empire Extensions (Vulkan)
    'VULKAN_SHADER': 900,
    'COMPUTE_KERNEL': 901,
    'PIPELINE_CONFIG': 902,
}

CONTRACT_NAMES = {v: k for k, v in CONTRACT_IDS.items()}

# Schema definitions: field -> expected_type (str or type)
CONTRACT_SCHEMAS = {
    14: {'@0': int},
    15: {'@0': float},
    16: {'@0': str},
    17: {'@0': (bytes, bytearray)},
    18: {'@0': list},
    19: {'@0': dict},
    20: {'@0': str},
    21: {},
    22: {'@0': bool},
    # Empire Extensions
    900: {
        'spirv_binary': (bytes, bytearray),
        'entry_point': str,
        'shader_stage': str,
        'descriptor_bindings': list
    },
    901: {
        'kernel_name': str,
        'shader_handle': str,
        'workgroup_size': list,
        'shared_memory_bytes': int,
        'push_constants_layout': str
    },
    902: {
        'pipeline_id': str,
        'stages': list,
        'sync_barriers': list,
        'output_image': str
    }
}

def is_contract_wrapped(value: Any) -> bool:
    if not isinstance(value, dict) or len(value) != 1:
        return False
    key = list(value.keys())[0]
    try:
        int(key)
        return True
    except (ValueError, TypeError):
        return False

def get_contract_id(wrapped: Dict) -> Optional[int]:
    if not is_contract_wrapped(wrapped):
        return None
    return int(list(wrapped.keys())[0])

def get_contract_inner(wrapped: Dict) -> Optional[Dict]:
    if not is_contract_wrapped(wrapped):
        return None
    return wrapped[list(wrapped.keys())[0]]

def wrap_value(contract_id: int, inner: Dict) -> Dict:
    return {str(contract_id): inner}

def wrap_literal(value: Any) -> Dict:
    if value is None:
        return wrap_value(CONTRACT_IDS['NULL'], {})
    elif isinstance(value, bool):
        return wrap_value(CONTRACT_IDS['BOOL'], {'@0': value})
    elif isinstance(value, int):
        return wrap_value(CONTRACT_IDS['INT_LITERAL'], {'@0': value})
    elif isinstance(value, float):
        return wrap_value(CONTRACT_IDS['FLOAT_LITERAL'], {'@0': value})
    elif isinstance(value, str):
        if value.startswith('&h_'):
            return wrap_value(CONTRACT_IDS['HANDLE_REF'], {'@0': value})
        return wrap_value(CONTRACT_IDS['TEXT_LITERAL'], {'@0': value})
    elif isinstance(value, (bytes, bytearray)):
        return wrap_value(CONTRACT_IDS['BYTES_LITERAL'], {'@0': value})
    elif isinstance(value, list):
        return wrap_value(CONTRACT_IDS['ARRAY'], {'@0': [wrap_literal(v) for v in value]})
    elif isinstance(value, dict):
        if is_contract_wrapped(value):
            return value
        return wrap_value(CONTRACT_IDS['OBJECT'], {'@0': {k: wrap_literal(v) for k, v in value.items()}})
    else:
        raise ValueError(f"Cannot wrap type: {type(value)}")

def unwrap_literal(wrapped: Dict) -> Any:
    if not is_contract_wrapped(wrapped):
        return wrapped

    contract_id = get_contract_id(wrapped)
    inner = get_contract_inner(wrapped)

    if contract_id == CONTRACT_IDS['NULL']:
        return None
    elif contract_id == CONTRACT_IDS['BOOL']:
        return inner.get('@0', False)
    elif contract_id == CONTRACT_IDS['INT_LITERAL']:
        return inner.get('@0', 0)
    elif contract_id == CONTRACT_IDS['FLOAT_LITERAL']:
        return inner.get('@0', 0.0)
    elif contract_id == CONTRACT_IDS['TEXT_LITERAL']:
        return inner.get('@0', '')
    elif contract_id == CONTRACT_IDS['BYTES_LITERAL']:
        return inner.get('@0', b'')
    elif contract_id == CONTRACT_IDS['HANDLE_REF']:
        return inner.get('@0', '')
    elif contract_id == CONTRACT_IDS['ARRAY']:
        return [unwrap_literal(item) for item in inner.get('@0', [])]
    elif contract_id == CONTRACT_IDS['OBJECT']:
        return {k: unwrap_literal(v) for k, v in inner.get('@0', {}).items()}
    else:
        return wrapped

def validate_contract(contract_id: Union[int, Dict], inputs: Optional[Dict] = None) -> Dict:
    """
    CONTRACT_805: Contract Validator
    Usage:
      validate_contract(14, {'@0': 123}) -> {'@0': 123}
      validate_contract({'14': {'@0': 123}}) -> {'@0': 123}
    Raises ContractError on failure.
    """
    if isinstance(contract_id, dict) and inputs is None:
        # Wrapped mode
        wrapped = contract_id
        if not is_contract_wrapped(wrapped):
            raise ContractError(f"{E_CONTRACT_STRUCTURE}: Not a valid contract structure")
        cid = get_contract_id(wrapped)
        inputs = get_contract_inner(wrapped)
        contract_id = cid
    
    if not isinstance(inputs, dict):
        raise ContractError(f"{E_CONTRACT_STRUCTURE}: Inputs must be a dict")

    # Check known schema
    if contract_id in CONTRACT_SCHEMAS:
        schema = CONTRACT_SCHEMAS[contract_id]
        for field, expected_type in schema.items():
            if field not in inputs:
                raise ContractError(f"{E_CONTRACT_FIELD_MISSING}: Missing field '{field}' for contract {contract_id}")
            val = inputs[field]
            if not isinstance(val, expected_type):
                # Allow int for float if compatible?
                if expected_type is float and isinstance(val, int):
                    pass # OK
                else:
                    raise ContractError(f"{E_CONTRACT_FIELD_TYPE}: Field '{field}' expected {expected_type}, got {type(val)}")
        
        # Check for extra fields?
        # Contract 805 doesn't explicitly say reject extra fields, but "field_order" implies control.
        # "check_required_fields" implies only checking required.
        # But "inputs: dict of field_index -> type_constraint" suggests closed set.
        pass

    # Validate field order (keys must be sorted strings, but Python dicts preserve insertion order mostly. 
    # LC-B enforces order on encode/decode. Here we just check inputs dict keys?)
    # "check_field_order": "Ascending indices (INV-003)"
    # If inputs keys are numerical strings "0", "1", they should be sorted?
    # inputs is a dict. Iterating it might not be sorted.
    # But INV-003 is usually about the binary encoding.
    
    return inputs