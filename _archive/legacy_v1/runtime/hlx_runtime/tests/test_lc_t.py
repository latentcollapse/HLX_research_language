"""
Test suite for LC-T encoder/decoder
Verifies CONTRACT_801 compliance
"""

import pytest
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from lc_t_codec import (
    encode_lct, decode_lct, verify_lct_bijection,
    LCTEncoder, LCTDecoder
)


class TestLCTPrimitives:
    """Test primitive value encoding/decoding"""

    def test_null(self):
        assert encode_lct(None) == "NULL"
        assert decode_lct("NULL") is None

    def test_bool_true(self):
        assert encode_lct(True) == "TRUE"
        assert decode_lct("TRUE") is True

    def test_bool_false(self):
        assert encode_lct(False) == "FALSE"
        assert decode_lct("FALSE") is False

    def test_integer(self):
        assert encode_lct(42) == "42"
        assert decode_lct("42") == 42

    def test_negative_integer(self):
        assert encode_lct(-17) == "-17"
        assert decode_lct("-17") == -17

    def test_zero(self):
        assert encode_lct(0) == "0"
        assert decode_lct("0") == 0

    def test_float(self):
        assert encode_lct(3.14) == "3.14"
        assert decode_lct("3.14") == 3.14

    def test_negative_float(self):
        encoded = encode_lct(-2.5)
        assert "-2.5" in encoded
        assert decode_lct(encoded) == -2.5

    def test_string(self):
        assert encode_lct("hello") == '"hello"'
        assert decode_lct('"hello"') == "hello"

    def test_empty_string(self):
        assert encode_lct("") == '""'
        assert decode_lct('""') == ""

    def test_string_with_quotes(self):
        # Escaped quotes
        encoded = encode_lct('say "hi"')
        assert '"' in encoded
        decoded = decode_lct(encoded)
        assert decoded == 'say "hi"'

    def test_string_with_backslash(self):
        encoded = encode_lct('path\\file')
        decoded = decode_lct(encoded)
        assert decoded == 'path\\file'

    def test_bytes(self):
        data = b'\x01\x02\x03'
        assert encode_lct(data) == "#010203"
        assert decode_lct("#010203") == data

    def test_empty_bytes(self):
        data = b''
        assert encode_lct(data) == "#"
        assert decode_lct("#") == data

    def test_handle_reference(self):
        # Handle references starting with & are encoded as @
        assert encode_lct("&shader_vert") == "@shader_vert"
        # When decoded, should restore the &h_ prefix
        decoded = decode_lct("@shader_vert")
        assert decoded == "&h_shader_vert"


class TestLCTStructures:
    """Test structured data encoding/decoding"""

    def test_empty_array(self):
        assert encode_lct([]) == "[]"
        assert decode_lct("[]") == []

    def test_integer_array(self):
        assert encode_lct([1, 2, 3]) == "[1,2,3]"
        assert decode_lct("[1,2,3]") == [1, 2, 3]

    def test_string_array(self):
        arr = ["a", "b"]
        encoded = encode_lct(arr)
        assert encoded == '["a","b"]'
        assert decode_lct(encoded) == arr

    def test_mixed_array(self):
        arr = [1, "hello", True, None]
        encoded = encode_lct(arr)
        decoded = decode_lct(encoded)
        assert decoded == arr

    def test_nested_array(self):
        arr = [[1, 2], [3, 4]]
        encoded = encode_lct(arr)
        decoded = decode_lct(encoded)
        assert decoded == arr

    def test_empty_object(self):
        assert encode_lct({}) == "{}"
        assert decode_lct("{}") == {}

    def test_simple_object(self):
        obj = {'x': 10}
        encoded = encode_lct(obj)
        assert encoded == '{x:10}'
        assert decode_lct(encoded) == obj

    def test_string_value_object(self):
        obj = {'name': 'Alice'}
        encoded = encode_lct(obj)
        decoded = decode_lct(encoded)
        assert decoded == obj

    def test_complex_object(self):
        obj = {'name': 'Alice', 'age': 30}
        encoded = encode_lct(obj)
        decoded = decode_lct(encoded)
        assert decoded == obj

    def test_nested_object(self):
        obj = {'person': {'name': 'Bob', 'age': 25}}
        encoded = encode_lct(obj)
        decoded = decode_lct(encoded)
        assert decoded == obj


class TestLCTContracts:
    """Test contract encoding/decoding"""

    def test_simple_contract(self):
        contract = {'contract_id': 14, 'field_0': 42}
        encoded = encode_lct(contract)
        assert encoded == '{C:14,0=42}'
        decoded = decode_lct(encoded)
        assert decoded['contract_id'] == 14
        assert decoded['field_0'] == 42

    def test_string_contract(self):
        contract = {'contract_id': 16, 'field_0': 'hello'}
        encoded = encode_lct(contract)
        assert encoded == '{C:16,0="hello"}'
        decoded = decode_lct(encoded)
        assert decoded['contract_id'] == 16
        assert decoded['field_0'] == 'hello'

    def test_multi_field_contract(self):
        contract = {
            'contract_id': 1000,
            'field_0': 'search',
            'field_1': '&h_query'
        }
        encoded = encode_lct(contract)
        # Should contain contract ID and both fields
        assert 'C:1000' in encoded
        assert '0=' in encoded
        assert '1=' in encoded
        
        decoded = decode_lct(encoded)
        assert decoded['contract_id'] == 1000
        assert decoded['field_0'] == 'search'

    def test_contract_with_array(self):
        contract = {'contract_id': 100, 'field_0': [1, 2, 3]}
        encoded = encode_lct(contract)
        decoded = decode_lct(encoded)
        assert decoded['contract_id'] == 100
        assert decoded['field_0'] == [1, 2, 3]


class TestLCTBijection:
    """Test encode → decode → encode produces identical results"""

    def test_bijection_null(self):
        assert verify_lct_bijection(None)

    def test_bijection_booleans(self):
        assert verify_lct_bijection(True)
        assert verify_lct_bijection(False)

    def test_bijection_integers(self):
        for val in [0, 1, -1, 42, -17, 1000000]:
            assert verify_lct_bijection(val), f"Bijection failed for {val}"

    def test_bijection_floats(self):
        for val in [0.0, 1.5, -2.5, 3.14159]:
            assert verify_lct_bijection(val), f"Bijection failed for {val}"

    def test_bijection_strings(self):
        for val in ["", "hello", "with spaces", 'with "quotes"']:
            assert verify_lct_bijection(val), f"Bijection failed for {val}"

    def test_bijection_bytes(self):
        for val in [b'', b'\x00', b'\x01\x02\x03', b'hello']:
            assert verify_lct_bijection(val), f"Bijection failed for {val}"

    def test_bijection_arrays(self):
        arrays = [[], [1], [1, 2, 3], ["a", "b", "c"], [[1, 2], [3, 4]]]
        for arr in arrays:
            assert verify_lct_bijection(arr), f"Bijection failed for {arr}"

    def test_bijection_objects(self):
        objects = [{}, {'x': 10}, {'name': 'test', 'value': 42}]
        for obj in objects:
            assert verify_lct_bijection(obj), f"Bijection failed for {obj}"

    def test_bijection_contracts(self):
        contracts = [
            {'contract_id': 14, 'field_0': 42},
            {'contract_id': 16, 'field_0': 'hello'},
        ]
        for contract in contracts:
            assert verify_lct_bijection(contract), f"Bijection failed for {contract}"


class TestLCTEdgeCases:
    """Test edge cases and error handling"""

    def test_whitespace_handling(self):
        # Should handle leading/trailing whitespace
        assert decode_lct("  42  ") == 42
        assert decode_lct("\n\tNULL\n") is None

    def test_large_number(self):
        val = 999999999999
        encoded = encode_lct(val)
        assert decode_lct(encoded) == val

    def test_unicode_in_string(self):
        # Unicode should be preserved in strings
        val = "hello 世界"
        encoded = encode_lct(val)
        decoded = decode_lct(encoded)
        assert decoded == val

    def test_deeply_nested(self):
        val = {'a': {'b': {'c': {'d': 1}}}}
        encoded = encode_lct(val)
        decoded = decode_lct(encoded)
        assert decoded == val


class TestLCTEncoderClass:
    """Test LCTEncoder class directly"""

    def test_encoder_instance(self):
        encoder = LCTEncoder()
        assert encoder.encode(42) == "42"

    def test_encoder_reuse(self):
        encoder = LCTEncoder()
        assert encoder.encode(1) == "1"
        assert encoder.encode(2) == "2"
        assert encoder.encode("hello") == '"hello"'


class TestLCTDecoderClass:
    """Test LCTDecoder class directly"""

    def test_decoder_instance(self):
        decoder = LCTDecoder()
        assert decoder.decode("42") == 42

    def test_decoder_reuse(self):
        decoder = LCTDecoder()
        assert decoder.decode("1") == 1
        assert decoder.decode("2") == 2
        assert decoder.decode('"hello"') == "hello"


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
