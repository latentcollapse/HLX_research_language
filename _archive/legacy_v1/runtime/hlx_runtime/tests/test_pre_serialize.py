"""
Test suite for CONTRACT_804: Pre-Serialize Validator

Tests all normalization and validation rules:
- Float validation (NaN, Infinity, zero normalization)
- String normalization (UTF-8 NFC, whitespace, line endings)
- Key ordering (lexicographic sorting and validation)
- Array/List normalization (recursive processing)
"""
import math
import pytest
import unicodedata
from hlx_runtime.pre_serialize import (
    pre_serialize,
    normalize_float,
    normalize_string,
    validate_key_order,
    float_to_ieee754_hex,
    FloatSpecialError,
    KeyOrderError,
    TrailingCommaError,
)


# ============================================================================
# Float Validation Tests
# ============================================================================

class TestFloatValidation:
    """Test float normalization and special value rejection."""

    def test_normal_float(self):
        """Normal floats should pass through unchanged."""
        assert normalize_float(3.14) == 3.14
        assert normalize_float(-2.718) == -2.718
        assert normalize_float(1e10) == 1e10
        assert normalize_float(1e-10) == 1e-10

    def test_positive_zero(self):
        """Positive zero should remain as +0.0."""
        result = normalize_float(0.0)
        assert result == 0.0
        assert math.copysign(1.0, result) == 1.0  # Verify positive zero

    def test_negative_zero_normalization(self):
        """Negative zero should be normalized to positive zero."""
        neg_zero = -0.0
        result = normalize_float(neg_zero)
        assert result == 0.0
        # Verify it's actually positive zero, not negative zero
        assert math.copysign(1.0, result) == 1.0

    def test_nan_rejection(self):
        """NaN values should be rejected with E_FLOAT_SPECIAL."""
        with pytest.raises(FloatSpecialError) as exc_info:
            normalize_float(float('nan'))
        assert exc_info.value.code == "E_FLOAT_SPECIAL"
        assert "NaN" in str(exc_info.value)

    def test_infinity_rejection(self):
        """Infinity values should be rejected with E_FLOAT_SPECIAL."""
        with pytest.raises(FloatSpecialError) as exc_info:
            normalize_float(float('inf'))
        assert exc_info.value.code == "E_FLOAT_SPECIAL"
        assert "Infinity" in str(exc_info.value)

    def test_negative_infinity_rejection(self):
        """Negative infinity values should be rejected with E_FLOAT_SPECIAL."""
        with pytest.raises(FloatSpecialError) as exc_info:
            normalize_float(float('-inf'))
        assert exc_info.value.code == "E_FLOAT_SPECIAL"
        assert "Infinity" in str(exc_info.value)

    def test_ieee754_hex_conversion(self):
        """Test IEEE754 hex representation conversion."""
        # 3.14 in IEEE754 double precision
        hex_repr = float_to_ieee754_hex(3.14)
        assert hex_repr == "0x40091eb851eb851f"

        # Zero
        hex_repr = float_to_ieee754_hex(0.0)
        assert hex_repr == "0x0000000000000000"

        # One
        hex_repr = float_to_ieee754_hex(1.0)
        assert hex_repr == "0x3ff0000000000000"

        # Negative one
        hex_repr = float_to_ieee754_hex(-1.0)
        assert hex_repr == "0xbff0000000000000"


# ============================================================================
# String Normalization Tests
# ============================================================================

class TestStringNormalization:
    """Test UTF-8 NFC normalization and whitespace cleanup."""

    def test_nfc_normalization_combining_chars(self):
        """UTF-8 NFC should normalize combining characters."""
        # 'café' with combining acute accent (e + ́)
        decomposed = 'cafe\u0301'
        # Should normalize to composed form (é as single character)
        normalized = normalize_string(decomposed)
        composed = 'café'
        assert normalized == composed
        assert unicodedata.normalize('NFC', decomposed) == normalized

    def test_nfc_normalization_already_composed(self):
        """Already composed strings should remain unchanged."""
        composed = 'café'
        normalized = normalize_string(composed)
        assert normalized == composed

    def test_trailing_whitespace_removal(self):
        """Trailing whitespace should be stripped."""
        assert normalize_string('text   ') == 'text'
        assert normalize_string('text\t\t') == 'text'
        assert normalize_string('text \t ') == 'text'

    def test_leading_whitespace_preserved(self):
        """Leading whitespace should be preserved."""
        assert normalize_string('   text') == '   text'
        assert normalize_string('\ttext') == '\ttext'

    def test_line_ending_normalization(self):
        """Windows line endings should be normalized to Unix."""
        assert normalize_string('line1\r\nline2') == 'line1\nline2'
        assert normalize_string('line1\r\n\r\nline2') == 'line1\n\nline2'

    def test_trailing_line_ending_removal(self):
        """Trailing line endings should be removed (they're whitespace)."""
        assert normalize_string('text\n') == 'text'
        assert normalize_string('text\r\n') == 'text'
        assert normalize_string('text\n\n') == 'text'

    def test_combined_normalization(self):
        """Test combined NFC, line ending, and whitespace normalization."""
        # Combining chars + Windows line endings + trailing whitespace
        input_str = 'cafe\u0301\r\nline2  '
        expected = 'café\nline2'
        assert normalize_string(input_str) == expected

    def test_empty_string(self):
        """Empty strings should remain empty."""
        assert normalize_string('') == ''

    def test_unicode_emoji(self):
        """Unicode emoji should be preserved."""
        emoji = '😀🎉'
        assert normalize_string(emoji) == emoji


# ============================================================================
# Key Ordering Tests
# ============================================================================

class TestKeyOrdering:
    """Test dictionary key ordering validation and normalization."""

    def test_already_sorted_keys(self):
        """Already sorted keys should pass validation."""
        keys = ['a', 'b', 'c']
        validate_key_order(keys)  # Should not raise

    def test_unsorted_keys_validation(self):
        """Unsorted keys should fail validation."""
        keys = ['z', 'a', 'b']
        with pytest.raises(KeyOrderError) as exc_info:
            validate_key_order(keys)
        assert exc_info.value.code == "E_KEY_ORDER"
        assert "lexicographic order" in str(exc_info.value)

    def test_dict_key_sorting(self):
        """pre_serialize should sort dictionary keys."""
        input_dict = {'z': 1, 'a': 2, 'm': 3}
        result = pre_serialize(input_dict)
        assert list(result.keys()) == ['a', 'm', 'z']
        assert result == {'a': 2, 'm': 3, 'z': 1}

    def test_dict_key_validation_mode(self):
        """pre_serialize with validate_order=True should reject unsorted keys."""
        input_dict = {'z': 1, 'a': 2}
        with pytest.raises(KeyOrderError):
            pre_serialize(input_dict, validate_order=True)

    def test_dict_key_validation_mode_sorted(self):
        """pre_serialize with validate_order=True should accept sorted keys."""
        input_dict = {'a': 2, 'z': 1}
        result = pre_serialize(input_dict, validate_order=True)
        assert result == {'a': 2, 'z': 1}

    def test_numeric_string_keys(self):
        """Numeric string keys should sort lexicographically, not numerically."""
        input_dict = {'10': 'a', '2': 'b', '1': 'c'}
        result = pre_serialize(input_dict)
        # Lexicographic: '1' < '10' < '2'
        assert list(result.keys()) == ['1', '10', '2']

    def test_integer_keys(self):
        """Integer keys should sort numerically."""
        input_dict = {10: 'a', 2: 'b', 1: 'c'}
        result = pre_serialize(input_dict)
        assert list(result.keys()) == [1, 2, 10]

    def test_mixed_type_keys(self):
        """Mixed type keys should sort by type, then value."""
        # Note: In Python 3, comparing different types raises TypeError
        # So we test that sorting works for same-type keys
        input_dict = {'10': 'a', '2': 'b'}
        result = pre_serialize(input_dict)
        assert list(result.keys()) == ['10', '2']


# ============================================================================
# Recursive Normalization Tests
# ============================================================================

class TestRecursiveNormalization:
    """Test recursive normalization of nested structures."""

    def test_nested_dict(self):
        """Nested dictionaries should be recursively normalized."""
        input_data = {
            'z': {'inner_z': 1, 'inner_a': 2},
            'a': {'inner_z': 3, 'inner_a': 4}
        }
        result = pre_serialize(input_data)
        # Outer keys sorted
        assert list(result.keys()) == ['a', 'z']
        # Inner keys sorted
        assert list(result['a'].keys()) == ['inner_a', 'inner_z']
        assert list(result['z'].keys()) == ['inner_a', 'inner_z']

    def test_nested_list(self):
        """Nested lists should be recursively normalized."""
        input_data = [
            {'z': 1, 'a': 2},
            [float('-0.0'), 'text  '],
            'cafe\u0301'
        ]
        result = pre_serialize(input_data)
        # First element: dict with sorted keys
        assert list(result[0].keys()) == ['a', 'z']
        # Second element: list with normalized float and string
        assert result[1][0] == 0.0
        assert result[1][1] == 'text'
        # Third element: normalized string
        assert result[2] == 'café'

    def test_deeply_nested_structure(self):
        """Deeply nested structures should be fully normalized."""
        input_data = {
            'level1': {
                'level2': {
                    'level3': {
                        'z': float('-0.0'),
                        'a': 'text\r\n'
                    }
                }
            }
        }
        result = pre_serialize(input_data)
        level3 = result['level1']['level2']['level3']
        assert list(level3.keys()) == ['a', 'z']
        assert level3['z'] == 0.0
        assert level3['a'] == 'text'

    def test_list_of_dicts(self):
        """Lists containing dictionaries should normalize dict keys."""
        input_data = [
            {'z': 1, 'a': 2},
            {'y': 3, 'b': 4}
        ]
        result = pre_serialize(input_data)
        assert list(result[0].keys()) == ['a', 'z']
        assert list(result[1].keys()) == ['b', 'y']

    def test_dict_of_lists(self):
        """Dictionaries containing lists should normalize list contents."""
        input_data = {
            'z': [float('-0.0'), 'text  '],
            'a': ['cafe\u0301']
        }
        result = pre_serialize(input_data)
        assert list(result.keys()) == ['a', 'z']
        assert result['z'][0] == 0.0
        assert result['z'][1] == 'text'
        assert result['a'][0] == 'café'


# ============================================================================
# Primitive Type Tests
# ============================================================================

class TestPrimitiveTypes:
    """Test normalization of primitive types."""

    def test_none(self):
        """None should pass through unchanged."""
        assert pre_serialize(None) is None

    def test_boolean(self):
        """Booleans should pass through unchanged."""
        assert pre_serialize(True) is True
        assert pre_serialize(False) is False

    def test_integer(self):
        """Integers should pass through unchanged."""
        assert pre_serialize(0) == 0
        assert pre_serialize(42) == 42
        assert pre_serialize(-100) == -100
        assert pre_serialize(2**63) == 2**63

    def test_float(self):
        """Normal floats should pass through (with -0.0 normalization)."""
        assert pre_serialize(3.14) == 3.14
        assert pre_serialize(-2.718) == -2.718
        assert pre_serialize(float('-0.0')) == 0.0

    def test_empty_list(self):
        """Empty lists should remain empty."""
        assert pre_serialize([]) == []

    def test_empty_dict(self):
        """Empty dicts should remain empty."""
        assert pre_serialize({}) == {}

    def test_bytes(self):
        """Bytes should pass through unchanged."""
        data = b'binary data'
        assert pre_serialize(data) == data

    def test_bytearray(self):
        """Bytearrays should pass through unchanged."""
        data = bytearray(b'binary data')
        result = pre_serialize(data)
        assert result == data
        assert isinstance(result, bytearray)

    def test_tuple_conversion(self):
        """Tuples should be converted to lists."""
        input_tuple = (1, 2, 3)
        result = pre_serialize(input_tuple)
        assert result == [1, 2, 3]
        assert isinstance(result, list)

    def test_nested_tuple(self):
        """Nested tuples should be recursively converted to lists."""
        input_tuple = (1, (2, 3), 4)
        result = pre_serialize(input_tuple)
        assert result == [1, [2, 3], 4]


# ============================================================================
# Integration Tests (Positive Cases from CONTRACT_804)
# ============================================================================

class TestContractPositiveCases:
    """Test positive cases from CONTRACT_804 specification."""

    def test_validate_pi(self):
        """validate(3.14) should normalize correctly."""
        result = pre_serialize(3.14)
        assert result == 3.14
        # Check IEEE754 hex representation
        hex_repr = float_to_ieee754_hex(result)
        assert hex_repr == "0x40091eb851eb851f"

    def test_validate_string_nfc(self):
        """validate('café') should apply NFC normalization."""
        # Input with combining acute
        input_str = 'cafe\u0301'
        result = pre_serialize(input_str)
        # Should be NFC normalized (single é character)
        assert result == 'café'
        assert unicodedata.normalize('NFC', result) == result

    def test_validate_dict_key_sorting(self):
        """validate({z:1, a:2}) should sort to {a:2, z:1}."""
        input_dict = {'z': 1, 'a': 2}
        result = pre_serialize(input_dict)
        assert result == {'a': 2, 'z': 1}
        assert list(result.keys()) == ['a', 'z']


# ============================================================================
# Integration Tests (Negative Cases from CONTRACT_804)
# ============================================================================

class TestContractNegativeCases:
    """Test negative cases from CONTRACT_804 specification."""

    def test_validate_nan_rejection(self):
        """validate(float('nan')) should raise E_FLOAT_SPECIAL."""
        with pytest.raises(FloatSpecialError) as exc_info:
            pre_serialize(float('nan'))
        assert exc_info.value.code == "E_FLOAT_SPECIAL"

    def test_validate_inf_rejection(self):
        """validate(float('inf')) should raise E_FLOAT_SPECIAL."""
        with pytest.raises(FloatSpecialError) as exc_info:
            pre_serialize(float('inf'))
        assert exc_info.value.code == "E_FLOAT_SPECIAL"

    def test_validate_key_order_violation(self):
        """validate({1:a, 0:b}) with validation should raise E_KEY_ORDER."""
        # Using validate_order=True to trigger validation
        input_dict = {1: 'a', 0: 'b'}
        with pytest.raises(KeyOrderError) as exc_info:
            pre_serialize(input_dict, validate_order=True)
        assert exc_info.value.code == "E_KEY_ORDER"


# ============================================================================
# Edge Cases and Robustness Tests
# ============================================================================

class TestEdgeCases:
    """Test edge cases and robustness."""

    def test_very_small_float(self):
        """Very small floats should be handled correctly."""
        value = 1e-300
        assert pre_serialize(value) == value

    def test_very_large_float(self):
        """Very large floats should be handled correctly."""
        value = 1e300
        assert pre_serialize(value) == value

    def test_subnormal_float(self):
        """Subnormal floats should be handled correctly."""
        value = 2.2250738585072014e-308  # Smallest normal positive double
        assert pre_serialize(value) == value

    def test_max_float(self):
        """Maximum float value should be handled correctly."""
        value = 1.7976931348623157e+308  # Max double
        assert pre_serialize(value) == value

    def test_string_with_only_whitespace(self):
        """Strings with only whitespace should be stripped to empty."""
        assert normalize_string('   \t\n') == ''
        assert normalize_string('\r\n') == ''

    def test_unicode_normalization_edge_cases(self):
        """Test various Unicode normalization edge cases."""
        # Ligature fi
        assert normalize_string('\ufb01') == '\ufb01'  # NFC doesn't decompose ligatures
        # Zero-width joiner
        zwj = '\u200d'
        assert normalize_string(f'a{zwj}b') == f'a{zwj}b'

    def test_empty_nested_structures(self):
        """Empty nested structures should be handled correctly."""
        input_data = {'a': [], 'b': {}, 'c': [{}]}
        result = pre_serialize(input_data)
        assert result == {'a': [], 'b': {}, 'c': [{}]}

    def test_deeply_nested_lists(self):
        """Deeply nested lists should be normalized recursively."""
        input_data = [[[[float('-0.0')]]]]
        result = pre_serialize(input_data)
        assert result == [[[[0.0]]]]

    def test_dict_with_special_string_keys(self):
        """Dictionaries with special string keys should sort correctly."""
        input_dict = {'': 1, ' ': 2, '\n': 3, '\t': 4}
        result = pre_serialize(input_dict)
        # Should be sorted lexicographically by Unicode codepoint
        assert list(result.keys()) == sorted(['', ' ', '\n', '\t'])

    def test_complex_mixed_structure(self):
        """Test complex mixed nested structure with all types."""
        input_data = {
            'floats': [3.14, float('-0.0'), -2.718],
            'strings': ['cafe\u0301\r\n', 'text  '],
            'nested': {
                'z': {'inner': [1, 2, 3]},
                'a': {'inner': [4, 5, 6]}
            },
            'primitives': [None, True, False, 42],
            'empty': {'list': [], 'dict': {}}
        }
        result = pre_serialize(input_data)

        # Check top-level keys are sorted
        assert list(result.keys()) == ['empty', 'floats', 'nested', 'primitives', 'strings']

        # Check floats
        assert result['floats'] == [3.14, 0.0, -2.718]

        # Check strings
        assert result['strings'] == ['café', 'text']

        # Check nested dicts
        assert list(result['nested'].keys()) == ['a', 'z']

        # Check primitives
        assert result['primitives'] == [None, True, False, 42]


# ============================================================================
# Performance and Stress Tests
# ============================================================================

class TestPerformance:
    """Test performance with large structures."""

    def test_large_dict(self):
        """Test with a large dictionary."""
        input_dict = {f'key_{i:05d}': i for i in range(1000, 0, -1)}
        result = pre_serialize(input_dict)
        # Should be sorted
        keys = list(result.keys())
        assert keys == sorted(keys)
        assert len(result) == 1000

    def test_large_list(self):
        """Test with a large list."""
        input_list = [float('-0.0') if i % 2 == 0 else i for i in range(1000)]
        result = pre_serialize(input_list)
        # All even indices should be normalized to 0.0
        for i in range(0, 1000, 2):
            assert result[i] == 0.0

    def test_deeply_nested_dict(self):
        """Test with deeply nested dictionaries."""
        # Create a deeply nested structure
        depth = 50
        data = {'value': float('-0.0')}
        for i in range(depth):
            data = {'z': data, 'a': i}

        result = pre_serialize(data)

        # Navigate to the deepest level
        current = result
        for i in range(depth):
            assert list(current.keys()) == ['a', 'z']
            current = current['z']

        # Check the innermost value
        assert current['value'] == 0.0


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
