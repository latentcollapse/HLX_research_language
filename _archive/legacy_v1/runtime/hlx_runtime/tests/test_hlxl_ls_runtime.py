"""
Test suite for HLXL-LS runtime
Verifies ASCII language execution with Latent Space operations
"""

import pytest
import sys
import os

# Add parent directory to path for imports
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

from hlxl_ls_runtime import (
    HLXLRuntime, execute_hlxl,
    HLXLTokenizer, HLXLParser, HLXLEvaluator,
    SimpleCAS, get_cas_store
)


class TestHLXLTokenizer:
    """Test HLXL tokenization"""

    def test_tokenize_literals(self):
        tokenizer = HLXLTokenizer('null true false 42 3.14 "hello"')
        tokens = tokenizer.tokenize()
        assert len(tokens) > 0
        # Should have NULL, TRUE, FALSE, NUMBER, NUMBER, STRING
        token_types = [t[0] for t in tokens]
        assert 'NULL' in token_types
        assert 'TRUE' in token_types
        assert 'FALSE' in token_types
        assert 'NUMBER' in token_types
        assert 'STRING' in token_types

    def test_tokenize_binding(self):
        tokenizer = HLXLTokenizer('let x = 42')
        tokens = tokenizer.tokenize()
        token_types = [t[0] for t in tokens]
        assert 'KEYWORD' in token_types  # 'let'
        assert 'IDENT' in token_types    # 'x'
        assert 'EQ' in token_types       # '='
        assert 'NUMBER' in token_types   # '42'

    def test_tokenize_method_call(self):
        tokenizer = HLXLTokenizer('ls.collapse(42)')
        tokens = tokenizer.tokenize()
        token_types = [t[0] for t in tokens]
        assert 'IDENT' in token_types
        assert 'DOT' in token_types
        assert 'LPAREN' in token_types
        assert 'RPAREN' in token_types

    def test_tokenize_operators(self):
        tokenizer = HLXLTokenizer('a + b * c')
        tokens = tokenizer.tokenize()
        token_types = [t[0] for t in tokens]
        assert 'PLUS' in token_types
        assert 'STAR' in token_types

    def test_tokenize_comparison(self):
        tokenizer = HLXLTokenizer('a == b')
        tokens = tokenizer.tokenize()
        token_types = [t[0] for t in tokens]
        assert 'EQEQ' in token_types

    def test_tokenize_handle(self):
        tokenizer = HLXLTokenizer('@myHandle')
        tokens = tokenizer.tokenize()
        token_types = [t[0] for t in tokens]
        assert 'AT' in token_types
        assert 'IDENT' in token_types


class TestHLXLParser:
    """Test HLXL parsing"""

    def test_parse_literal(self):
        tokenizer = HLXLTokenizer('42')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_binding(self):
        tokenizer = HLXLTokenizer('let x = 10')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_binary_op(self):
        tokenizer = HLXLTokenizer('1 + 2')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_method_call(self):
        tokenizer = HLXLTokenizer('ls.collapse(42)')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_array(self):
        tokenizer = HLXLTokenizer('[1, 2, 3]')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None

    def test_parse_object(self):
        tokenizer = HLXLTokenizer('{x: 10}')
        tokens = tokenizer.tokenize()
        parser = HLXLParser(tokens)
        ast = parser.parse()
        assert ast is not None


class TestHLXLEvaluator:
    """Test HLXL evaluation"""

    def test_eval_null(self):
        runtime = HLXLRuntime()
        assert runtime.execute('null') is None

    def test_eval_true(self):
        runtime = HLXLRuntime()
        assert runtime.execute('true') is True

    def test_eval_false(self):
        runtime = HLXLRuntime()
        assert runtime.execute('false') is False

    def test_eval_integer(self):
        runtime = HLXLRuntime()
        assert runtime.execute('42') == 42

    def test_eval_negative_integer(self):
        runtime = HLXLRuntime()
        assert runtime.execute('-17') == -17

    def test_eval_float(self):
        runtime = HLXLRuntime()
        assert runtime.execute('3.14') == 3.14

    def test_eval_string(self):
        runtime = HLXLRuntime()
        assert runtime.execute('"hello"') == "hello"

    def test_eval_binding(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        assert runtime.get_var('x') == 10

    def test_eval_variable_reference(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        result = runtime.execute('x')
        assert result == 10


class TestHLXLLatentSpace:
    """Test Latent Space operations in HLXL"""

    def test_collapse(self):
        runtime = HLXLRuntime()
        handle = runtime.execute('ls.collapse(42)')
        assert isinstance(handle, str)
        assert handle.startswith('&h_')

    def test_resolve(self):
        runtime = HLXLRuntime()
        handle = runtime.execute('ls.collapse(42)')
        runtime.set_var('h', handle)
        value = runtime.execute('ls.resolve(h)')
        assert value == 42

    def test_collapse_resolve_roundtrip(self):
        runtime = HLXLRuntime()
        runtime.execute('let val = 42')
        handle = runtime.execute('ls.collapse(val)')
        runtime.set_var('h', handle)
        recovered = runtime.execute('ls.resolve(h)')
        assert recovered == 42

    def test_collapse_string(self):
        runtime = HLXLRuntime()
        runtime.execute('let s = "hello"')
        handle = runtime.execute('ls.collapse(s)')
        runtime.set_var('h', handle)
        recovered = runtime.execute('ls.resolve(h)')
        assert recovered == "hello"

    def test_snapshot(self):
        runtime = HLXLRuntime()
        runtime.execute('ls.collapse(1)')
        runtime.execute('ls.collapse(2)')
        snapshot = runtime.execute('ls.snapshot()')
        assert isinstance(snapshot, dict)
        assert len(snapshot) >= 2


class TestHLXLContracts:
    """Test contract execution"""

    def test_simple_contract(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{14: {@0: 42}}')
        assert result['contract_id'] == 14
        assert result['field_0'] == 42

    def test_string_contract(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{16: {@0: "hello"}}')
        assert result['contract_id'] == 16
        assert result['field_0'] == "hello"


class TestHLXLArithmetic:
    """Test arithmetic operations"""

    def test_addition(self):
        runtime = HLXLRuntime()
        result = runtime.execute('5 + 3')
        assert result == 8

    def test_subtraction(self):
        runtime = HLXLRuntime()
        result = runtime.execute('10 - 3')
        assert result == 7

    def test_multiplication(self):
        runtime = HLXLRuntime()
        result = runtime.execute('4 * 3')
        assert result == 12

    def test_division(self):
        runtime = HLXLRuntime()
        result = runtime.execute('12 / 4')
        assert result == 3.0

    def test_with_variables(self):
        runtime = HLXLRuntime()
        runtime.execute('let a = 5')
        runtime.execute('let b = 3')
        result = runtime.execute('a + b')
        assert result == 8

    def test_comparison_equal(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 == 5') is True
        assert runtime.execute('5 == 3') is False

    def test_comparison_less_than(self):
        runtime = HLXLRuntime()
        assert runtime.execute('3 < 5') is True
        assert runtime.execute('5 < 3') is False

    def test_comparison_greater_than(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 > 3') is True
        assert runtime.execute('3 > 5') is False


class TestHLXLArrays:
    """Test array operations"""

    def test_empty_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[]')
        assert result == []

    def test_integer_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[1, 2, 3]')
        assert result == [1, 2, 3]

    def test_mixed_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[1, "hello", true]')
        assert result == [1, "hello", True]

    def test_nested_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[[1, 2], [3, 4]]')
        assert result == [[1, 2], [3, 4]]


class TestHLXLObjects:
    """Test object operations"""

    def test_empty_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{}')
        assert result == {}

    def test_simple_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{x: 10}')
        assert result == {'x': 10}

    def test_multi_field_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{x: 10, y: 20}')
        assert result == {'x': 10, 'y': 20}

    def test_string_value_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{name: "Alice"}')
        assert result == {'name': 'Alice'}


class TestHLXLBuiltins:
    """Test built-in functions"""

    def test_print(self):
        runtime = HLXLRuntime()
        # Should not raise
        runtime.execute('print("hello")')

    def test_type_int(self):
        runtime = HLXLRuntime()
        result = runtime.execute('type(42)')
        assert result == 'int'

    def test_type_str(self):
        runtime = HLXLRuntime()
        result = runtime.execute('type("hello")')
        assert result == 'str'

    def test_type_bool(self):
        runtime = HLXLRuntime()
        result = runtime.execute('type(true)')
        assert result == 'bool'

    def test_type_list(self):
        runtime = HLXLRuntime()
        result = runtime.execute('type([1, 2, 3])')
        assert result == 'list'


class TestHLXLEnvironment:
    """Test environment management"""

    def test_get_env(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.execute('let y = 20')
        env = runtime.get_env()
        assert 'x' in env
        assert 'y' in env
        assert env['x'] == 10
        assert env['y'] == 20

    def test_set_var(self):
        runtime = HLXLRuntime()
        runtime.set_var('external', 100)
        result = runtime.execute('external')
        assert result == 100

    def test_clear_env(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.clear_env()
        env = runtime.get_env()
        assert 'x' not in env


class TestHLXLConvenienceFunction:
    """Test execute_hlxl convenience function"""

    def test_execute_hlxl(self):
        result = execute_hlxl('42')
        assert result == 42

    def test_execute_hlxl_with_cas(self):
        cas = SimpleCAS()
        result = execute_hlxl('ls.collapse(42)', cas)
        assert isinstance(result, str)
        assert result.startswith('&h_')


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
