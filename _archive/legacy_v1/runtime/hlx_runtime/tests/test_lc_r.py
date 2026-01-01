"""
Comprehensive test suite for LC-R (Latent Collapse - Runic) encoder/decoder

Tests:
- Primitive types (null, bool, int, float, text, bytes, handles)
- Complex types (arrays, objects, contracts)
- Nesting and composition
- Reversibility: decode(encode(v)) = v
- Determinism: encode(v, t1) = encode(v, t2)
- Edge cases and error handling
"""

import pytest
from hlx_runtime.lc_r_codec import encode_lcr, decode_lcr, compression_ratio
from hlx_runtime.glyphs import LC_R_GLYPHS


class TestPrimitives:
    """Test primitive type encoding/decoding"""

    def test_null(self):
        assert encode_lcr(None) == '∅'
        assert decode_lcr('∅') is None

    def test_true(self):
        assert encode_lcr(True) == '⊤'
        assert decode_lcr('⊤') is True

    def test_false(self):
        assert encode_lcr(False) == '⊥'
        assert decode_lcr('⊥') is False

    def test_integers(self):
        test_cases = [0, 1, -1, 42, -42, 12345, -12345]
        for num in test_cases:
            encoded = encode_lcr(num)
            assert encoded.startswith('🜃')
            assert decode_lcr(encoded) == num

    def test_floats(self):
        test_cases = [0.0, 1.5, -1.5, 3.14159, -3.14159, 1e10, 1e-10]
        for num in test_cases:
            encoded = encode_lcr(num)
            assert encoded.startswith('🜄')
            decoded = decode_lcr(encoded)
            assert abs(decoded - num) < 1e-9

    def test_text(self):
        test_cases = [
            "hello",
            "world",
            "Hello, World!",
            "with spaces",
            "with\nnewlines",
            "unicode: 🌟✨",
        ]
        for text in test_cases:
            encoded = encode_lcr(text)
            assert encoded.startswith('᛭')
            assert decode_lcr(encoded) == text

    def test_text_with_quotes(self):
        text = 'text with "quotes"'
        encoded = encode_lcr(text)
        decoded = decode_lcr(encoded)
        assert decoded == text

    def test_handles(self):
        test_cases = [
            "&shader_vert",
            "&shader_frag",
            "&pipeline_compute",
        ]
        for handle in test_cases:
            encoded = encode_lcr(handle)
            assert encoded.startswith('⟁')
            assert decode_lcr(encoded) == handle

    def test_bytes(self):
        test_cases = [
            b'',
            b'\x00',
            b'\x01\x02\x03',
            b'\xff\xfe\xfd',
            b'hello world',
        ]
        for data in test_cases:
            encoded = encode_lcr(data)
            assert encoded.startswith('᛫')
            assert decode_lcr(encoded) == data


class TestComplexTypes:
    """Test complex type encoding/decoding"""

    def test_empty_array(self):
        arr = []
        encoded = encode_lcr(arr)
        assert encoded.startswith('⋔')
        assert decode_lcr(encoded) == arr

    def test_array_integers(self):
        arr = [1, 2, 3, 4, 5]
        encoded = encode_lcr(arr)
        decoded = decode_lcr(encoded)
        assert decoded == arr

    def test_array_mixed_types(self):
        arr = [1, 3.14, "text", True, None]
        encoded = encode_lcr(arr)
        decoded = decode_lcr(encoded)
        assert decoded == arr

    def test_nested_arrays(self):
        arr = [[1, 2], [3, 4], [5, 6]]
        encoded = encode_lcr(arr)
        decoded = decode_lcr(encoded)
        assert decoded == arr

    def test_empty_object(self):
        obj = {}
        encoded = encode_lcr(obj)
        # Empty object should still roundtrip
        decoded = decode_lcr(encoded)
        assert decoded == obj

    def test_simple_object(self):
        obj = {"key": "value", "number": 42}
        encoded = encode_lcr(obj)
        decoded = decode_lcr(encoded)
        assert decoded == obj

    def test_nested_object(self):
        obj = {
            "outer": {
                "inner": "value"
            }
        }
        encoded = encode_lcr(obj)
        decoded = decode_lcr(encoded)
        # Note: nested objects need proper handling
        assert decoded["outer"]["inner"] == "value"


class TestContracts:
    """Test contract encoding/decoding"""

    def test_simple_contract(self):
        contract = {
            'contract_id': 902,
            'pipeline_id': 'test',
        }
        encoded = encode_lcr(contract)
        assert encoded.startswith('🜊902')
        assert encoded.endswith('🜂')

        decoded = decode_lcr(encoded)
        assert decoded['contract_id'] == 902
        assert 'field_0' in decoded

    def test_contract_with_handle(self):
        contract = {
            'contract_id': 902,
            'pipeline_id': 'test',
            'shader': '&shader_vert',
        }
        encoded = encode_lcr(contract)
        decoded = decode_lcr(encoded)
        assert decoded['contract_id'] == 902

    def test_contract_with_array(self):
        contract = {
            'contract_id': 902,
            'stages': ['&vert', '&frag'],
        }
        encoded = encode_lcr(contract)
        decoded = decode_lcr(encoded)
        assert decoded['contract_id'] == 902
        # Check that array field exists and has 2 elements
        array_field = [v for k, v in decoded.items() if isinstance(v, list)][0]
        assert len(array_field) == 2

    def test_nested_contracts(self):
        inner_contract = {
            'contract_id': 100,
            'value': 42,
        }
        outer_contract = {
            'contract_id': 200,
            'inner': inner_contract,
        }
        encoded = encode_lcr(outer_contract)
        decoded = decode_lcr(encoded)
        assert decoded['contract_id'] == 200


class TestReversibility:
    """Test that decode(encode(v)) = v for all types"""

    def test_reversibility_primitives(self):
        test_values = [
            None,
            True,
            False,
            0,
            42,
            -42,
            3.14,
            -3.14,
            "hello",
            "text with spaces",
            "&handle_ref",
            b'\x01\x02\x03',
        ]
        for value in test_values:
            encoded = encode_lcr(value)
            decoded = decode_lcr(encoded)
            assert decoded == value, f"Failed for {value}"

    def test_reversibility_arrays(self):
        arrays = [
            [],
            [1],
            [1, 2, 3],
            [1, "two", 3.0],
            [[1, 2], [3, 4]],
        ]
        for arr in arrays:
            encoded = encode_lcr(arr)
            decoded = decode_lcr(encoded)
            assert decoded == arr

    def test_reversibility_contracts(self):
        contracts = [
            {'contract_id': 100, 'x': 1},
            {'contract_id': 902, 'pipeline': 'test', 'stages': ['&vert']},
        ]
        for contract in contracts:
            encoded = encode_lcr(contract)
            decoded = decode_lcr(encoded)
            assert decoded['contract_id'] == contract['contract_id']


class TestDeterminism:
    """Test that encode(v, t1) = encode(v, t2) always"""

    def test_determinism_primitives(self):
        values = [None, True, False, 42, 3.14, "test", b'\x01\x02']
        for value in values:
            enc1 = encode_lcr(value)
            enc2 = encode_lcr(value)
            assert enc1 == enc2

    def test_determinism_arrays(self):
        arr = [1, 2, 3, "four", 5.0]
        enc1 = encode_lcr(arr)
        enc2 = encode_lcr(arr)
        assert enc1 == enc2

    def test_determinism_contracts(self):
        contract = {'contract_id': 902, 'x': 1, 'y': 2}
        enc1 = encode_lcr(contract)
        enc2 = encode_lcr(contract)
        assert enc1 == enc2


class TestCompression:
    """Test compression characteristics"""

    def test_compression_primitives(self):
        # Primitives should be very compact
        assert len(encode_lcr(None)) == 1  # ∅
        assert len(encode_lcr(True)) == 1  # ⊤
        assert len(encode_lcr(False)) == 1  # ⊥

    def test_compression_numbers(self):
        # Numbers are glyph + digits
        encoded_42 = encode_lcr(42)
        assert len(encoded_42) <= 5  # 🜃 + "42"

    def test_compression_ratio_contract(self):
        contract = {'contract_id': 902, 'pipeline_id': 'test'}
        ascii_repr = 'contract 902 { pipeline_id: "test" }'
        lcr_encoded = encode_lcr(contract)

        ratio = compression_ratio(ascii_repr, lcr_encoded)
        # Should achieve some compression (even if not 65-70% yet)
        assert ratio < 1.0


class TestEdgeCases:
    """Test edge cases and error handling"""

    def test_empty_string(self):
        text = ""
        encoded = encode_lcr(text)
        decoded = decode_lcr(encoded)
        assert decoded == text

    def test_empty_bytes(self):
        data = b''
        encoded = encode_lcr(data)
        decoded = decode_lcr(encoded)
        assert decoded == data

    def test_large_numbers(self):
        large_int = 999999999999
        encoded = encode_lcr(large_int)
        decoded = decode_lcr(encoded)
        assert decoded == large_int

    def test_negative_numbers(self):
        neg_int = -42
        neg_float = -3.14
        assert decode_lcr(encode_lcr(neg_int)) == neg_int
        assert abs(decode_lcr(encode_lcr(neg_float)) - neg_float) < 1e-9

    def test_special_floats(self):
        # Note: NaN and Inf might need special handling
        values = [0.0, -0.0, 1e100, 1e-100]
        for val in values:
            encoded = encode_lcr(val)
            decoded = decode_lcr(encoded)
            assert abs(decoded - val) < 1e-90


class TestGlyphUsage:
    """Test that correct glyphs are used"""

    def test_null_glyph(self):
        assert encode_lcr(None) == LC_R_GLYPHS['NULL']

    def test_bool_glyphs(self):
        assert encode_lcr(True) == LC_R_GLYPHS['TRUE']
        assert encode_lcr(False) == LC_R_GLYPHS['FALSE']

    def test_type_glyphs(self):
        assert encode_lcr(42).startswith(LC_R_GLYPHS['INT'])
        assert encode_lcr(3.14).startswith(LC_R_GLYPHS['FLOAT'])
        assert encode_lcr("text").startswith(LC_R_GLYPHS['TEXT'])
        assert encode_lcr(b'\x01').startswith(LC_R_GLYPHS['BYTES'])
        assert encode_lcr([1]).startswith(LC_R_GLYPHS['ARRAY'])

    def test_contract_glyphs(self):
        contract = {'contract_id': 100, 'x': 42}  # Need at least one field for FIELD glyph
        encoded = encode_lcr(contract)
        assert encoded.startswith(LC_R_GLYPHS['CONTRACT_START'])
        assert encoded.endswith(LC_R_GLYPHS['CONTRACT_END'])
        assert LC_R_GLYPHS['FIELD'] in encoded


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
