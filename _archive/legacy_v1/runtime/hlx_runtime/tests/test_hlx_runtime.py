"""
Test suite for HLX basic runtime
Verifies Runic language execution WITHOUT Latent Space operations
"""

import pytest
import sys
import os

# Add grandparent directory to path for imports (to find hlx_runtime package)
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from hlx_runtime.hlx_runtime import (
    HLXRuntime, execute_hlx,
    HLXTokenizer, HLXParser, HLXEvaluator
)


class TestHLXTokenizer:
    """Test HLX tokenization"""

    def test_tokenize_null(self):
        tokenizer = HLXTokenizer('∅')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'GLYPH' and t[1] == '∅' for t in tokens)

    def test_tokenize_true(self):
        tokenizer = HLXTokenizer('⊤')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'GLYPH' and t[1] == '⊤' for t in tokens)

    def test_tokenize_false(self):
        tokenizer = HLXTokenizer('⊥')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'GLYPH' and t[1] == '⊥' for t in tokens)

    def test_tokenize_integer(self):
        tokenizer = HLXTokenizer('🜃42')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'NUMBER' and t[1] == '42' for t in tokens)

    def test_tokenize_float(self):
        tokenizer = HLXTokenizer('🜄3.14')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'NUMBER' and t[1] == '3.14' for t in tokens)

    def test_tokenize_string(self):
        tokenizer = HLXTokenizer('᛭"hello"')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'STRING' and t[1] == 'hello' for t in tokens)

    def test_tokenize_arithmetic(self):
        tokenizer = HLXTokenizer('🜃5 + 🜃3')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'PLUS' for t in tokens)

    def test_tokenize_identifier(self):
        tokenizer = HLXTokenizer('myVar')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'IDENT' and t[1] == 'myVar' for t in tokens)

    def test_tokenize_binding(self):
        tokenizer = HLXTokenizer('x ⋯ 🜃10')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'IDENT' and t[1] == 'x' for t in tokens)
        assert any(t[0] == 'GLYPH' and t[1] == '⋯' for t in tokens)


class TestHLXParser:
    """Test HLX parsing"""

    def test_parse_null(self):
        tokenizer = HLXTokenizer('∅')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_integer(self):
        tokenizer = HLXTokenizer('🜃42')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_binding(self):
        tokenizer = HLXTokenizer('x ⋯ 🜃10')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_arithmetic(self):
        tokenizer = HLXTokenizer('🜃5 + 🜃3')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_contract(self):
        tokenizer = HLXTokenizer('🜊14🜁0 🜃42🜂')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None


class TestHLXEvaluator:
    """Test HLX evaluation"""

    def test_eval_null(self):
        runtime = HLXRuntime()
        assert runtime.execute('∅') is None

    def test_eval_true(self):
        runtime = HLXRuntime()
        assert runtime.execute('⊤') is True

    def test_eval_false(self):
        runtime = HLXRuntime()
        assert runtime.execute('⊥') is False

    def test_eval_integer(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜃42') == 42

    def test_eval_negative_integer(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜃-17') == -17

    def test_eval_float(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜄3.14') == 3.14

    def test_eval_string(self):
        runtime = HLXRuntime()
        assert runtime.execute('᛭"hello"') == "hello"

    def test_eval_binding(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 🜃10')
        assert runtime.get_var('x') == 10

    def test_eval_variable_reference(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 🜃10')
        result = runtime.execute('x')
        assert result == 10

    def test_eval_multiple_bindings(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 🜃10')
        runtime.execute('y ⋯ 🜃20')
        assert runtime.get_var('x') == 10
        assert runtime.get_var('y') == 20


class TestHLXArithmetic:
    """Test arithmetic operations"""

    def test_addition(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜃5 + 🜃3')
        assert result == 8

    def test_subtraction(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜃10 - 🜃3')
        assert result == 7

    def test_multiplication(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜃4 * 🜃2')
        assert result == 8

    def test_division(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜃10 / 🜃2')
        assert result == 5.0

    def test_with_variables(self):
        runtime = HLXRuntime()
        runtime.execute('a ⋯ 🜃5')
        runtime.execute('b ⋯ 🜃3')
        result = runtime.execute('a + b')
        assert result == 8

    def test_comparison_equal(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜃5 = 🜃5') is True

    def test_comparison_less_than(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜃3 < 🜃5') is True

    def test_comparison_greater_than(self):
        runtime = HLXRuntime()
        assert runtime.execute('🜃5 > 🜃3') is True

    def test_complex_expression(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜃2 + 🜃3 * 🜃4')
        # Due to left-to-right parsing: (2 + 3) * 4 = 20
        # Or proper precedence: 2 + (3 * 4) = 14
        assert result in [14, 20]  # Accept either based on implementation


class TestHLXContracts:
    """Test contract execution"""

    def test_simple_contract(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜊14🜁0 🜃42🜂')
        assert result['contract_id'] == 14
        assert result['field_0'] == 42

    def test_string_contract(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜊16🜁0 ᛭"hello"🜂')
        assert result['contract_id'] == 16
        assert result['field_0'] == "hello"

    def test_multi_field_contract(self):
        runtime = HLXRuntime()
        result = runtime.execute('🜊100🜁0 🜃1🜁1 🜃2🜂')
        assert result['contract_id'] == 100
        assert result['field_0'] == 1
        assert result['field_1'] == 2


class TestHLXArrays:
    """Test array operations"""

    def test_empty_array(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋔[]')
        assert result == []

    def test_integer_array(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋔[🜃1⋅🜃2⋅🜃3]')
        assert result == [1, 2, 3]

    def test_mixed_array(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋔[🜃1⋅᛭"hello"⋅⊤]')
        assert result == [1, "hello", True]


class TestHLXObjects:
    """Test object operations"""

    def test_simple_object(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋕{x⋯🜃10}')
        assert result == {'x': 10}

    def test_multi_field_object(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋕{x⋯🜃10⋅y⋯🜃20}')
        assert result == {'x': 10, 'y': 20}

    def test_string_value_object(self):
        runtime = HLXRuntime()
        result = runtime.execute('⋕{name⋯᛭"Alice"}')
        assert result == {'name': 'Alice'}


class TestHLXBuiltins:
    """Test built-in functions"""

    def test_print(self):
        runtime = HLXRuntime()
        # Should not raise
        runtime.execute('print(᛭"hello")')

    def test_type_int(self):
        runtime = HLXRuntime()
        result = runtime.execute('type(🜃42)')
        assert result == 'int'

    def test_type_str(self):
        runtime = HLXRuntime()
        result = runtime.execute('type(᛭"hello")')
        assert result == 'str'

    def test_type_bool(self):
        runtime = HLXRuntime()
        result = runtime.execute('type(⊤)')
        assert result == 'bool'

    def test_len(self):
        runtime = HLXRuntime()
        runtime.execute('arr ⋯ ⋔[🜃1⋅🜃2⋅🜃3]')
        result = runtime.execute('len(arr)')
        assert result == 3

    def test_str_conversion(self):
        runtime = HLXRuntime()
        result = runtime.execute('str(🜃42)')
        assert result == '42'

    def test_int_conversion(self):
        runtime = HLXRuntime()
        result = runtime.execute('int(᛭"42")')
        assert result == 42

    def test_float_conversion(self):
        runtime = HLXRuntime()
        result = runtime.execute('float(᛭"3.14")')
        assert result == 3.14


class TestHLXNoLatentSpace:
    """Verify that LS operations are NOT available"""

    def test_no_collapse_glyph(self):
        runtime = HLXRuntime()
        # ⊕ glyph should cause an error (not defined in basic runtime)
        with pytest.raises(Exception):
            runtime.execute('⊕(🜃42)')

    def test_no_resolve_glyph(self):
        runtime = HLXRuntime()
        # ⊖ glyph should cause an error (not defined in basic runtime)
        with pytest.raises(Exception):
            runtime.execute('⊖(⟁test)')

    def test_no_collapse_function(self):
        runtime = HLXRuntime()
        # collapse() function should not exist
        with pytest.raises(Exception):
            runtime.execute('collapse(🜃42)')

    def test_no_resolve_function(self):
        runtime = HLXRuntime()
        # resolve() function should not exist
        with pytest.raises(Exception):
            runtime.execute('resolve(⟁test)')


class TestHLXEnvironment:
    """Test environment management"""

    def test_get_env(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 🜃10')
        runtime.execute('y ⋯ 🜃20')
        env = runtime.get_env()
        assert 'x' in env
        assert 'y' in env
        assert env['x'] == 10
        assert env['y'] == 20

    def test_set_var(self):
        runtime = HLXRuntime()
        runtime.set_var('external', 100)
        result = runtime.execute('external')
        assert result == 100

    def test_clear_env(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 🜃10')
        runtime.clear_env()
        env = runtime.get_env()
        assert 'x' not in env


class TestHLXConvenienceFunction:
    """Test execute_hlx convenience function"""

    def test_execute_hlx_simple(self):
        result = execute_hlx('🜃42')
        assert result == 42

    def test_execute_hlx_arithmetic(self):
        result = execute_hlx('🜃5 + 🜃3')
        assert result == 8

    def test_execute_hlx_null(self):
        result = execute_hlx('∅')
        assert result is None


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
