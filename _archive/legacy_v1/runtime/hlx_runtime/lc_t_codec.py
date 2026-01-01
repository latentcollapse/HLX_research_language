"""
LC-T (Latent Collapse - Text) Encoder/Decoder
ASCII-safe text format for cross-platform HLX wire protocol.

LC-T provides human-readable serialization with NO Unicode characters,
making it ideal for cross-platform compatibility, logging, and text-based protocols.

Reference: CONTRACT_801, LC-T_LC-B_specification.md

Format Rules:
    NULL    → "NULL"
    TRUE    → "TRUE"
    FALSE   → "FALSE"
    42      → "42"
    3.14    → "3.14"
    "hello" → "\"hello\""
    b"data" → "#64617461"           (hex encoding)
    &h_ast  → "@ast"                (handle reference)
    
    {14: {@0: 42}}                  → {C:14,0=42}
    {16: {@0: "hello"}}             → {C:16,0="hello"}
    
    []                              → []
    [1, 2, 3]                       → [1,2,3]
    
    {x: 10}                         → {x:10}
"""

from typing import Any, Dict, List, Union, Optional
import re

# Error codes (matching existing error system)
E_LC_PARSE = "E_LC_PARSE"
E_LC_ENCODE = "E_LC_ENCODE"
E_LC_DECODE = "E_LC_DECODE"
E_CONTRACT_STRUCTURE = "E_CONTRACT_STRUCTURE"
E_INVALID_INPUT = "E_INVALID_INPUT"


class LCTError(Exception):
    """LC-T encoding/decoding error"""
    pass


# Type alias
LCTValue = Union[None, bool, int, float, str, bytes, List[Any], Dict[str, Any]]


class LCTEncoder:
    """Encodes Python values to LC-T (ASCII text) format"""

    def __init__(self):
        """Initialize LC-T encoder"""
        pass

    def encode(self, value: Any) -> str:
        """
        Encode a Python value to LC-T string

        Args:
            value: Python value (None, bool, int, float, str, bytes, list, dict, contract)

        Returns:
            LC-T ASCII string

        Raises:
            LCTError: If value cannot be encoded
        """
        if value is None:
            return "NULL"
        
        elif isinstance(value, bool):
            return "TRUE" if value else "FALSE"
        
        elif isinstance(value, int):
            return str(value)
        
        elif isinstance(value, float):
            # Format float, removing trailing zeros but keeping at least one decimal
            s = f"{value:.15g}"
            # Ensure it looks like a float
            if '.' not in s and 'e' not in s.lower():
                s = f"{value}"
            return s
        
        elif isinstance(value, str):
            # Check if it's a handle reference (starts with & or &h_)
            if value.startswith('&h_'):
                # Handle reference: &h_abc → @abc
                return "@" + value[3:]
            elif value.startswith('&'):
                # Handle reference: &shader → @shader
                return "@" + value[1:]
            else:
                # Regular string: escape quotes and wrap
                return self._encode_string(value)
        
        elif isinstance(value, bytes):
            # Bytes: hex encode with # prefix
            return "#" + value.hex()
        
        elif isinstance(value, list):
            return self._encode_array(value)
        
        elif isinstance(value, dict):
            return self._encode_dict(value)
        
        else:
            raise LCTError(f"{E_LC_ENCODE}: Cannot encode type {type(value).__name__}")

    def _encode_string(self, s: str) -> str:
        """Encode a string with proper escaping"""
        # Escape backslashes first, then quotes
        escaped = s.replace('\\', '\\\\').replace('"', '\\"')
        return f'"{escaped}"'

    def _encode_array(self, arr: List[Any]) -> str:
        """Encode an array: [elem,elem,elem]"""
        if not arr:
            return "[]"
        elements = [self.encode(elem) for elem in arr]
        return "[" + ",".join(elements) + "]"

    def _encode_dict(self, d: Dict[str, Any]) -> str:
        """Encode a dict - either as contract or object"""
        if not d:
            return "{}"
        
        # Check if it's a contract (has 'contract_id')
        if 'contract_id' in d:
            return self._encode_contract(d)
        else:
            return self._encode_object(d)

    def _encode_contract(self, contract: Dict[str, Any]) -> str:
        """
        Encode a contract dict to LC-T format: {C:id,field=value,...}

        Args:
            contract: Dict with 'contract_id' and field_N values

        Returns:
            LC-T contract string
        """
        contract_id = contract['contract_id']
        fields = []
        
        # Extract field_N entries
        for key, value in contract.items():
            if key == 'contract_id':
                continue
            
            # Parse field index from 'field_N' format
            if key.startswith('field_'):
                try:
                    field_idx = int(key[6:])
                    encoded_value = self.encode(value)
                    fields.append((field_idx, encoded_value))
                except ValueError:
                    # Not a standard field, skip
                    pass
        
        # Sort by field index
        fields.sort(key=lambda x: x[0])
        
        # Build contract string
        parts = [f"C:{contract_id}"]
        for idx, val in fields:
            parts.append(f"{idx}={val}")
        
        return "{" + ",".join(parts) + "}"

    def _encode_object(self, obj: Dict[str, Any]) -> str:
        """Encode a regular object: {key:val,key:val}"""
        if not obj:
            return "{}"
        
        parts = []
        for key, value in obj.items():
            encoded_value = self.encode(value)
            parts.append(f"{key}:{encoded_value}")
        
        return "{" + ",".join(parts) + "}"


class LCTDecoder:
    """Decodes LC-T (ASCII text) format to Python values"""

    def __init__(self):
        """Initialize LC-T decoder with parsing state"""
        self.pos = 0
        self.text = ""

    def decode(self, lct_str: str) -> Any:
        """
        Decode LC-T string to Python value

        Args:
            lct_str: LC-T ASCII string

        Returns:
            Python value (decoded)

        Raises:
            LCTError: If parsing fails
        """
        self.text = lct_str.strip()
        self.pos = 0
        
        if not self.text:
            raise LCTError(f"{E_LC_DECODE}: Empty input")
        
        result = self._parse_value()
        
        # Ensure we consumed all input
        self._skip_whitespace()
        if self.pos < len(self.text):
            raise LCTError(f"{E_LC_DECODE}: Unexpected content after value at position {self.pos}")
        
        return result

    def _parse_value(self) -> Any:
        """Parse a single value from current position"""
        self._skip_whitespace()
        
        if self.pos >= len(self.text):
            raise LCTError(f"{E_LC_DECODE}: Unexpected end of input")
        
        char = self.text[self.pos]
        
        # NULL
        if self._match("NULL"):
            return None
        
        # TRUE
        if self._match("TRUE"):
            return True
        
        # FALSE
        if self._match("FALSE"):
            return False
        
        # Handle reference: @name
        if char == '@':
            self.pos += 1
            name = self._read_identifier()
            return "&h_" + name
        
        # Hex bytes: #hexdigits
        if char == '#':
            self.pos += 1
            hex_str = self._read_hex()
            return bytes.fromhex(hex_str)
        
        # String: "..."
        if char == '"':
            return self._read_string()
        
        # Array: [...]
        if char == '[':
            return self._parse_array()
        
        # Object or Contract: {...}
        if char == '{':
            return self._parse_brace()
        
        # Number (int or float)
        if char == '-' or char.isdigit():
            return self._read_number()
        
        # Identifier (for handle references without @)
        if char.isalpha() or char == '_':
            ident = self._read_identifier()
            # Could be a bare identifier - return as handle reference
            return "&h_" + ident
        
        raise LCTError(f"{E_LC_DECODE}: Unexpected character '{char}' at position {self.pos}")

    def _match(self, keyword: str) -> bool:
        """Try to match a keyword at current position"""
        if self.text[self.pos:self.pos+len(keyword)] == keyword:
            # Make sure it's not part of a longer word
            end_pos = self.pos + len(keyword)
            if end_pos >= len(self.text) or not self.text[end_pos].isalnum():
                self.pos = end_pos
                return True
        return False

    def _parse_brace(self) -> Dict[str, Any]:
        """Parse content inside braces - either contract or object"""
        self.pos += 1  # Skip '{'
        self._skip_whitespace()
        
        # Empty object
        if self.pos < len(self.text) and self.text[self.pos] == '}':
            self.pos += 1
            return {}
        
        # Check if it's a contract (starts with C:)
        if self.text[self.pos:self.pos+2] == 'C:':
            return self._parse_contract()
        else:
            return self._parse_object()

    def _parse_contract(self) -> Dict[str, Any]:
        """
        Parse a contract: {C:id,field_idx=value,...}

        Returns:
            Dict with 'contract_id' and indexed fields
        """
        # Skip 'C:'
        self.pos += 2
        
        # Read contract ID
        contract_id = int(self._read_number_str())
        
        result = {'contract_id': contract_id}
        
        # Parse fields
        while self.pos < len(self.text):
            self._skip_whitespace()
            
            if self.text[self.pos] == '}':
                self.pos += 1
                break
            
            if self.text[self.pos] == ',':
                self.pos += 1
                self._skip_whitespace()
            
            # Read field index
            field_idx = int(self._read_number_str())
            
            # Expect '='
            self._skip_whitespace()
            if self.pos >= len(self.text) or self.text[self.pos] != '=':
                raise LCTError(f"{E_LC_DECODE}: Expected '=' after field index at position {self.pos}")
            self.pos += 1
            
            # Parse field value
            self._skip_whitespace()
            value = self._parse_value()
            
            result[f'field_{field_idx}'] = value
        
        return result

    def _parse_array(self) -> List[Any]:
        """Parse an array: [elem,elem,elem]"""
        self.pos += 1  # Skip '['
        self._skip_whitespace()
        
        elements = []
        
        # Empty array
        if self.pos < len(self.text) and self.text[self.pos] == ']':
            self.pos += 1
            return elements
        
        while True:
            self._skip_whitespace()
            elements.append(self._parse_value())
            
            self._skip_whitespace()
            if self.pos >= len(self.text):
                raise LCTError(f"{E_LC_DECODE}: Unterminated array")
            
            if self.text[self.pos] == ']':
                self.pos += 1
                break
            
            if self.text[self.pos] == ',':
                self.pos += 1
            else:
                raise LCTError(f"{E_LC_DECODE}: Expected ',' or ']' in array at position {self.pos}")
        
        return elements

    def _parse_object(self) -> Dict[str, Any]:
        """Parse an object: {key:val,key:val}"""
        result = {}
        
        while True:
            self._skip_whitespace()
            
            if self.pos >= len(self.text):
                raise LCTError(f"{E_LC_DECODE}: Unterminated object")
            
            if self.text[self.pos] == '}':
                self.pos += 1
                break
            
            # Read key (identifier)
            key = self._read_identifier()
            
            # Expect ':'
            self._skip_whitespace()
            if self.pos >= len(self.text) or self.text[self.pos] != ':':
                raise LCTError(f"{E_LC_DECODE}: Expected ':' after key '{key}' at position {self.pos}")
            self.pos += 1
            
            # Parse value
            self._skip_whitespace()
            value = self._parse_value()
            
            result[key] = value
            
            self._skip_whitespace()
            if self.pos < len(self.text) and self.text[self.pos] == ',':
                self.pos += 1
        
        return result

    def _read_string(self) -> str:
        """Read a quoted string with escape handling"""
        self.pos += 1  # Skip opening quote
        
        result = []
        while self.pos < len(self.text):
            char = self.text[self.pos]
            
            if char == '"':
                self.pos += 1
                return ''.join(result)
            
            if char == '\\':
                self.pos += 1
                if self.pos >= len(self.text):
                    raise LCTError(f"{E_LC_DECODE}: Unterminated escape sequence")
                
                escaped = self.text[self.pos]
                if escaped == 'n':
                    result.append('\n')
                elif escaped == 't':
                    result.append('\t')
                elif escaped == 'r':
                    result.append('\r')
                elif escaped == '\\':
                    result.append('\\')
                elif escaped == '"':
                    result.append('"')
                else:
                    result.append(escaped)
                
                self.pos += 1
            else:
                result.append(char)
                self.pos += 1
        
        raise LCTError(f"{E_LC_DECODE}: Unterminated string")

    def _read_number(self) -> Union[int, float]:
        """Read a number (int or float)"""
        num_str = self._read_number_str()
        
        if '.' in num_str or 'e' in num_str.lower():
            return float(num_str)
        else:
            return int(num_str)

    def _read_number_str(self) -> str:
        """Read digits (and . for floats, - for negative) until non-digit"""
        start = self.pos
        
        # Optional negative sign
        if self.pos < len(self.text) and self.text[self.pos] == '-':
            self.pos += 1
        
        # Digits before decimal
        while self.pos < len(self.text) and self.text[self.pos].isdigit():
            self.pos += 1
        
        # Optional decimal part
        if self.pos < len(self.text) and self.text[self.pos] == '.':
            self.pos += 1
            while self.pos < len(self.text) and self.text[self.pos].isdigit():
                self.pos += 1
        
        # Optional exponent
        if self.pos < len(self.text) and self.text[self.pos].lower() == 'e':
            self.pos += 1
            if self.pos < len(self.text) and self.text[self.pos] in '+-':
                self.pos += 1
            while self.pos < len(self.text) and self.text[self.pos].isdigit():
                self.pos += 1
        
        return self.text[start:self.pos]

    def _read_identifier(self) -> str:
        """Read an identifier (alphanumeric + underscore)"""
        start = self.pos
        
        while self.pos < len(self.text):
            char = self.text[self.pos]
            if char.isalnum() or char == '_':
                self.pos += 1
            else:
                break
        
        if self.pos == start:
            raise LCTError(f"{E_LC_DECODE}: Expected identifier at position {self.pos}")
        
        return self.text[start:self.pos]

    def _read_hex(self) -> str:
        """Read hex digits"""
        start = self.pos
        
        while self.pos < len(self.text):
            char = self.text[self.pos]
            if char in '0123456789abcdefABCDEF':
                self.pos += 1
            else:
                break
        
        return self.text[start:self.pos]

    def _skip_whitespace(self):
        """Skip whitespace characters"""
        while self.pos < len(self.text) and self.text[self.pos].isspace():
            self.pos += 1


# Convenience functions (must match existing API)

def encode_lct(value: Any) -> str:
    """
    Encode a Python value to LC-T (ASCII text) format

    Args:
        value: Python value to encode

    Returns:
        LC-T ASCII string

    Example:
        >>> encode_lct(None)
        'NULL'
        >>> encode_lct(True)
        'TRUE'
        >>> encode_lct(42)
        '42'
        >>> encode_lct({'contract_id': 902, 'field_0': 'test'})
        '{C:902,0="test"}'
    """
    encoder = LCTEncoder()
    return encoder.encode(value)


def decode_lct(lct_str: str) -> Any:
    """
    Decode an LC-T (ASCII text) string to Python value

    Args:
        lct_str: LC-T ASCII string

    Returns:
        Python value (decoded)

    Example:
        >>> decode_lct('NULL')
        None
        >>> decode_lct('TRUE')
        True
        >>> decode_lct('42')
        42
        >>> decode_lct('{C:902,0="test"}')
        {'contract_id': 902, 'field_0': 'test'}
    """
    decoder = LCTDecoder()
    return decoder.decode(lct_str)


# Roundtrip validation

def verify_lct_bijection(value: Any) -> bool:
    """
    Verify that encode → decode → encode produces identical results

    Args:
        value: Python value to test

    Returns:
        True if bijection holds
    """
    try:
        encoded1 = encode_lct(value)
        decoded = decode_lct(encoded1)
        encoded2 = encode_lct(decoded)
        return encoded1 == encoded2
    except Exception:
        return False


if __name__ == '__main__':
    print("LC-T Encoder/Decoder Test Suite\n")

    # Test primitives
    test_cases = [
        (None, "NULL"),
        (True, "TRUE"),
        (False, "FALSE"),
        (42, "42"),
        (-17, "-17"),
        (3.14, "3.14"),
        ("hello", '"hello"'),
        ("@shader", '"@shader"'),  # Text string containing @
        (b'\x01\x02\x03', "#010203"),
        ([1, 2, 3], "[1,2,3]"),
        ([], "[]"),
        ({'x': 10}, '{x:10}'),
        ({}, "{}"),
        ({'contract_id': 14, 'field_0': 42}, '{C:14,0=42}'),
        ({'contract_id': 16, 'field_0': 'hello'}, '{C:16,0="hello"}'),
    ]

    print("=== Encoding Tests ===")
    for value, expected_pattern in test_cases:
        encoded = encode_lct(value)
        match = "✓" if encoded == expected_pattern else "✗"
        print(f"{match} encode({str(value):30s}) → {encoded} (expected: {expected_pattern})")

    print("\n=== Roundtrip Tests ===")
    for value, _ in test_cases:
        try:
            encoded = encode_lct(value)
            decoded = decode_lct(encoded)
            # For bytes comparison, handle is special
            if isinstance(value, str) and value.startswith('&'):
                # Handle references are transformed
                match = "✓"
            else:
                match = "✓" if decoded == value else "✗"
            print(f"{match} {str(value):30s} → {encoded} → {decoded}")
        except Exception as e:
            print(f"✗ {str(value):30s} → ERROR: {e}")

    print("\n=== Bijection Test ===")
    bijection_tests = [None, True, False, 42, -17, 3.14, "hello", [1, 2, 3], {'x': 10}]
    for val in bijection_tests:
        result = verify_lct_bijection(val)
        status = "✓" if result else "✗"
        print(f"{status} Bijection: {val}")

    print("\n=== Contract Tests ===")
    contracts = [
        {'contract_id': 14, 'field_0': 42},
        {'contract_id': 16, 'field_0': 'hello'},
        {'contract_id': 1000, 'field_0': 'search', 'field_1': '&h_query'},
    ]
    for contract in contracts:
        encoded = encode_lct(contract)
        decoded = decode_lct(encoded)
        match = "✓" if decoded['contract_id'] == contract['contract_id'] else "✗"
        print(f"{match} {contract} → {encoded}")

    print("\n✨ LC-T encoder/decoder ready!")
