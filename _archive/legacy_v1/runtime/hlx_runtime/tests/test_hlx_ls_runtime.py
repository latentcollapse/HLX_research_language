"""
Test suite for HLX-LS runtime
Verifies Runic language execution with Latent Space operations
"""

import pytest
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from hlx_ls_runtime import (
    HLXRuntime, execute_hlx,
    HLXTokenizer, HLXParser, HLXEvaluator,
    SimpleCAS, get_cas_store
)


class TestHLXTokenizer:
    """Test HLX tokenization"""

    def test_tokenize_number(self):
        tokenizer = HLXTokenizer('42')
        tokens = tokenizer.tokenize()
        assert len(tokens) >= 1
        assert any(t[0] == 'NUMBER' for t in tokens)

    def test_tokenize_string(self):
        tokenizer = HLXTokenizer('"hello"')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'STRING' for t in tokens)

    def test_tokenize_identifier(self):
        tokenizer = HLXTokenizer('myVar')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'IDENT' for t in tokens)

    def test_tokenize_operators(self):
        tokenizer = HLXTokenizer('a + b')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'PLUS' for t in tokens)

    def test_tokenize_parentheses(self):
        tokenizer = HLXTokenizer('(1 + 2)')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'LPAREN' for t in tokens)
        assert any(t[0] == 'RPAREN' for t in tokens)

    def test_tokenize_glyphs(self):
        tokenizer = HLXTokenizer('∅')
        tokens = tokenizer.tokenize()
        assert any(t[0] == 'GLYPH' for t in tokens)


class TestHLXParser:
    """Test HLX parsing"""

    def test_parse_number(self):
        tokenizer = HLXTokenizer('42')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_string(self):
        tokenizer = HLXTokenizer('"hello"')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_binary_op(self):
        tokenizer = HLXTokenizer('1 + 2')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_binding(self):
        tokenizer = HLXTokenizer('x ⋯ 10')
        tokens = tokenizer.tokenize()
        parser = HLXParser(tokens)
        ast = parser.parse()
        assert ast is not None


class TestHLXEvaluator:
    """Test HLX evaluation"""

    def test_eval_integer(self):
        runtime = HLXRuntime()
        assert runtime.execute('42') == 42

    def test_eval_negative_integer(self):
        runtime = HLXRuntime()
        assert runtime.execute('-17') == -17

    def test_eval_float(self):
        runtime = HLXRuntime()
        assert runtime.execute('3.14') == 3.14

    def test_eval_string(self):
        runtime = HLXRuntime()
        assert runtime.execute('"hello"') == "hello"

    def test_eval_null_glyph(self):
        runtime = HLXRuntime()
        assert runtime.execute('∅') is None

    def test_eval_true_glyph(self):
        runtime = HLXRuntime()
        assert runtime.execute('⊤') is True

    def test_eval_false_glyph(self):
        runtime = HLXRuntime()
        assert runtime.execute('⊥') is False

    def test_eval_binding(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 10')
        assert runtime.get_var('x') == 10

    def test_eval_variable_reference(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 10')
        result = runtime.execute('x')
        assert result == 10


class TestHLXArithmetic:
    """Test arithmetic operations"""

    def test_addition(self):
        runtime = HLXRuntime()
        result = runtime.execute('5 + 3')
        assert result == 8

    def test_subtraction(self):
        runtime = HLXRuntime()
        result = runtime.execute('10 - 3')
        assert result == 7

    def test_multiplication(self):
        runtime = HLXRuntime()
        result = runtime.execute('4 * 3')
        assert result == 12

    def test_division(self):
        runtime = HLXRuntime()
        result = runtime.execute('12 / 4')
        assert result == 3.0

    def test_with_variables(self):
        runtime = HLXRuntime()
        runtime.execute('a ⋯ 5')
        runtime.execute('b ⋯ 3')
        result = runtime.execute('a + b')
        assert result == 8

    def test_comparison(self):
        runtime = HLXRuntime()
        assert runtime.execute('5 > 3') is True
        assert runtime.execute('3 > 5') is False
        assert runtime.execute('5 < 3') is False


class TestHLXLatentSpace:
    """Test Latent Space operations in HLX"""

    def test_collapse(self):
        runtime = HLXRuntime()
        handle = runtime.execute('⊕(42)')
        assert isinstance(handle, str)
        assert handle.startswith('&h_')

    def test_resolve(self):
        runtime = HLXRuntime()
        handle = runtime.execute('⊕(42)')
        runtime.set_var('h', handle)
        value = runtime.execute('⊖(h)')
        assert value == 42

    def test_collapse_resolve_roundtrip(self):
        runtime = HLXRuntime()
        original = 42
        runtime.set_var('val', original)

        handle = runtime.execute('⊕(val)')
        runtime.set_var('h', handle)

        recovered = runtime.execute('⊖(h)')
        assert recovered == original

    def test_collapse_string(self):
        runtime = HLXRuntime()
        runtime.set_var('s', "hello world")
        handle = runtime.execute('⊕(s)')
        runtime.set_var('h', handle)
        recovered = runtime.execute('⊖(h)')
        assert recovered == "hello world"

    def test_collapse_array(self):
        runtime = HLXRuntime()
        runtime.set_var('arr', [1, 2, 3])
        handle = runtime.execute('⊕(arr)')
        runtime.set_var('h', handle)
        recovered = runtime.execute('⊖(h)')
        assert recovered == [1, 2, 3]


class TestHLXBuiltins:
    """Test built-in functions"""

    def test_print(self):
        runtime = HLXRuntime()
        # Should not raise
        runtime.set_var('msg', "hello")
        runtime.execute('print(msg)')

    def test_type_int(self):
        runtime = HLXRuntime()
        runtime.set_var('x', 42)
        result = runtime.execute('type(x)')
        assert result == 'int'

    def test_type_str(self):
        runtime = HLXRuntime()
        runtime.set_var('x', "hello")
        result = runtime.execute('type(x)')
        assert result == 'str'

    def test_type_list(self):
        runtime = HLXRuntime()
        runtime.set_var('x', [1, 2, 3])
        result = runtime.execute('type(x)')
        assert result == 'list'


class TestHLXEnvironment:
    """Test environment management"""

    def test_get_env(self):
        runtime = HLXRuntime()
        runtime.execute('x ⋯ 10')
        runtime.execute('y ⋯ 20')
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
        runtime.execute('x ⋯ 10')
        runtime.clear_env()
        env = runtime.get_env()
        assert 'x' not in env


class TestHLXConvenienceFunction:
    """Test execute_hlx convenience function"""

    def test_execute_hlx(self):
        result = execute_hlx('42')
        assert result == 42

    def test_execute_hlx_with_cas(self):
        cas = SimpleCAS()
        result = execute_hlx('⊕(42)', cas)
        assert isinstance(result, str)
        assert result.startswith('&h_')


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
