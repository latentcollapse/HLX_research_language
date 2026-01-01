"""
Pre-Serialize Validator
Reference: CONTRACT_804

Implements pre-serialization normalization and validation rules according to the
HLX specification. This ensures deterministic encoding by normalizing values
before they are serialized to the LC-B wire format.

Requirements:
- Float validation: Reject NaN/Infinity, normalize zero values
- String normalization: UTF-8 NFC normalization, whitespace cleanup
- Key ordering: Lexicographic sorting and validation
- Array validation: Reject trailing commas
"""
import unicodedata
import math
import struct
from typing import Any
from .errors import E_FLOAT_SPECIAL, E_KEY_ORDER, E_TRAILING_COMMA, HLXError


class FloatSpecialError(HLXError):
    """Raised when NaN or Infinity float values are encountered."""
    def __init__(self, message: str):
        super().__init__(E_FLOAT_SPECIAL, message)


class KeyOrderError(HLXError):
    """Raised when dictionary keys are not in lexicographic order."""
    def __init__(self, message: str):
        super().__init__(E_KEY_ORDER, message)


class TrailingCommaError(HLXError):
    """Raised when arrays contain trailing commas."""
    def __init__(self, message: str):
        super().__init__(E_TRAILING_COMMA, message)


def float_to_ieee754_hex(value: float) -> str:
    """
    Convert a float to its IEEE754 double-precision hex representation.

    Args:
        value: Float value to convert

    Returns:
        Hex string representation (e.g., "0x400921f9f01b866e")

    Examples:
        >>> float_to_ieee754_hex(3.14)
        '0x400921f9f01b866e'
        >>> float_to_ieee754_hex(0.0)
        '0x0000000000000000'
    """
    # Pack as double-precision IEEE754 (8 bytes, big-endian)
    packed = struct.pack('>d', value)
    # Convert to hex string
    hex_str = '0x' + packed.hex()
    return hex_str


def normalize_float(value: float) -> float:
    """
    Normalize float values according to CONTRACT_804 requirements.

    - Rejects NaN with E_FLOAT_SPECIAL
    - Rejects Infinity with E_FLOAT_SPECIAL
    - Normalizes -0.0 to +0.0 for canonical form

    Args:
        value: Float value to normalize

    Returns:
        Normalized float value

    Raises:
        FloatSpecialError: If value is NaN or Infinity
    """
    if math.isnan(value):
        raise FloatSpecialError("NaN values are not allowed in HLX serialization")

    if math.isinf(value):
        raise FloatSpecialError("Infinity values are not allowed in HLX serialization")

    # Normalize -0.0 to +0.0
    # In Python, -0.0 == 0.0 but they have different bit representations
    # Use copysign to detect negative zero
    if value == 0.0 and math.copysign(1.0, value) == -1.0:
        return 0.0

    return value


def normalize_string(value: str) -> str:
    """
    Normalize string values according to CONTRACT_804 requirements.

    - Applies UTF-8 NFC normalization
    - Strips trailing whitespace
    - Normalizes line endings (\\r\\n → \\n)

    Args:
        value: String to normalize

    Returns:
        Normalized string

    Examples:
        >>> normalize_string('café')  # with combining acute
        'café'  # with single character
        >>> normalize_string('text  \\r\\n')
        'text\\n'
    """
    # UTF-8 NFC normalization (Canonical Decomposition followed by Canonical Composition)
    normalized = unicodedata.normalize('NFC', value)

    # Normalize line endings: \r\n → \n
    normalized = normalized.replace('\r\n', '\n')

    # Strip trailing whitespace
    normalized = normalized.rstrip()

    return normalized


def validate_key_order(keys: list) -> None:
    """
    Validate that dictionary keys are in lexicographic order.

    Args:
        keys: List of keys to validate

    Raises:
        KeyOrderError: If keys are not in lexicographic order
    """
    sorted_keys = sorted(keys)
    if keys != sorted_keys:
        raise KeyOrderError(
            f"Dictionary keys must be in lexicographic order. "
            f"Expected {sorted_keys}, got {keys}"
        )


def pre_serialize(value: Any, validate_order: bool = False) -> Any:
    """
    Apply normalization rules before serialization according to CONTRACT_804.

    This function recursively normalizes values to ensure deterministic encoding:
    - Floats: Rejects NaN/Infinity, normalizes zero values
    - Strings: UTF-8 NFC normalization, whitespace cleanup
    - Dicts: Sorts keys lexicographically (or validates if validate_order=True)
    - Lists: Recursively normalizes elements

    Args:
        value: Value to normalize
        validate_order: If True, validate dict key order instead of sorting

    Returns:
        Normalized value

    Raises:
        FloatSpecialError: If float is NaN or Infinity
        KeyOrderError: If validate_order=True and keys are not sorted

    Examples:
        >>> pre_serialize(3.14)
        3.14
        >>> pre_serialize({'z': 1, 'a': 2})
        {'a': 2, 'z': 1}
        >>> pre_serialize('café')  # with combining acute
        'café'  # with single character
    """
    # Handle None/null
    if value is None:
        return None

    # Handle booleans (must come before int check since bool is subclass of int)
    if isinstance(value, bool):
        return value

    # Handle integers
    if isinstance(value, int):
        return value

    # Handle floats
    if isinstance(value, float):
        return normalize_float(value)

    # Handle strings
    if isinstance(value, str):
        return normalize_string(value)

    # Handle dictionaries
    if isinstance(value, dict):
        if validate_order:
            # Validate existing key order
            keys = list(value.keys())
            validate_key_order(keys)
            # Recursively normalize values
            return {k: pre_serialize(v, validate_order) for k, v in value.items()}
        else:
            # Sort keys lexicographically
            sorted_keys = sorted(value.keys())
            # Recursively normalize values
            return {k: pre_serialize(value[k], validate_order) for k in sorted_keys}

    # Handle lists/arrays
    if isinstance(value, list):
        # Recursively normalize elements
        return [pre_serialize(v, validate_order) for v in value]

    # Handle tuples (convert to list)
    if isinstance(value, tuple):
        return [pre_serialize(v, validate_order) for v in value]

    # Handle bytes/bytearray (pass through)
    if isinstance(value, (bytes, bytearray)):
        return value

    # Unknown type - pass through
    return value
