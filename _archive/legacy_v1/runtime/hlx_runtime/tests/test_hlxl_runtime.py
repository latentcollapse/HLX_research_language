"""
Test suite for HLXL basic runtime
Verifies ASCII language execution WITHOUT Latent Space operations
"""

import pytest
import sys
import os

# Add grandparent directory to path for imports (to find hlx_runtime package)
sys.path.insert(0, os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__)))))

from hlx_runtime.hlxl_runtime import (
    HLXLRuntime, HLXLError, execute_hlxl_basic,
    HLXLTokenizer, HLXLParser, HLXLEvaluator,
    TokenType
)


class TestBasicLiterals:
    """Test basic literal values"""

    def test_integer(self):
        runtime = HLXLRuntime()
        assert runtime.execute('42') == 42

    def test_negative_integer(self):
        runtime = HLXLRuntime()
        assert runtime.execute('-17') == -17

    def test_float(self):
        runtime = HLXLRuntime()
        assert runtime.execute('3.14') == 3.14

    def test_string(self):
        runtime = HLXLRuntime()
        assert runtime.execute('"hello"') == "hello"

    def test_empty_string(self):
        runtime = HLXLRuntime()
        assert runtime.execute('""') == ""

    def test_string_with_escapes(self):
        runtime = HLXLRuntime()
        assert runtime.execute('"hello\\nworld"') == "hello\nworld"

    def test_boolean_true(self):
        runtime = HLXLRuntime()
        assert runtime.execute('true') is True

    def test_boolean_false(self):
        runtime = HLXLRuntime()
        assert runtime.execute('false') is False

    def test_null(self):
        runtime = HLXLRuntime()
        assert runtime.execute('null') is None


class TestVariables:
    """Test variable binding and access"""

    def test_let_binding(self):
        runtime = HLXLRuntime()
        result = runtime.execute('let x = 42')
        assert result == 42
        assert runtime.get_var('x') == 42

    def test_variable_access(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 42')
        assert runtime.execute('x') == 42

    def test_multiple_bindings(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.execute('let y = 20')
        assert runtime.get_var('x') == 10
        assert runtime.get_var('y') == 20

    def test_variable_in_expression(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        result = runtime.execute('x + 5')
        assert result == 15

    def test_reassignment(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.execute('let x = 20')
        assert runtime.get_var('x') == 20


class TestArithmetic:
    """Test arithmetic operations"""

    def test_addition(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 + 3') == 8

    def test_subtraction(self):
        runtime = HLXLRuntime()
        assert runtime.execute('10 - 4') == 6

    def test_multiplication(self):
        runtime = HLXLRuntime()
        assert runtime.execute('6 * 7') == 42

    def test_division(self):
        runtime = HLXLRuntime()
        assert runtime.execute('15 / 3') == 5.0

    def test_modulo(self):
        runtime = HLXLRuntime()
        assert runtime.execute('17 % 5') == 2

    def test_operator_precedence(self):
        runtime = HLXLRuntime()
        # Multiplication before addition
        assert runtime.execute('2 + 3 * 4') == 14

    def test_parentheses(self):
        runtime = HLXLRuntime()
        assert runtime.execute('(2 + 3) * 4') == 20

    def test_chained_operations(self):
        runtime = HLXLRuntime()
        assert runtime.execute('1 + 2 + 3 + 4') == 10

    def test_unary_minus(self):
        runtime = HLXLRuntime()
        assert runtime.execute('-5') == -5
        assert runtime.execute('10 + -3') == 7


class TestComparison:
    """Test comparison operations"""

    def test_equal(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 == 5') is True
        assert runtime.execute('5 == 3') is False

    def test_not_equal(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 != 3') is True
        assert runtime.execute('5 != 5') is False

    def test_less_than(self):
        runtime = HLXLRuntime()
        assert runtime.execute('3 < 5') is True
        assert runtime.execute('5 < 3') is False
        assert runtime.execute('5 < 5') is False

    def test_less_equal(self):
        runtime = HLXLRuntime()
        assert runtime.execute('3 <= 5') is True
        assert runtime.execute('5 <= 5') is True
        assert runtime.execute('6 <= 5') is False

    def test_greater_than(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 > 3') is True
        assert runtime.execute('3 > 5') is False
        assert runtime.execute('5 > 5') is False

    def test_greater_equal(self):
        runtime = HLXLRuntime()
        assert runtime.execute('5 >= 3') is True
        assert runtime.execute('5 >= 5') is True
        assert runtime.execute('4 >= 5') is False


class TestLogical:
    """Test logical operations"""

    def test_and(self):
        runtime = HLXLRuntime()
        assert runtime.execute('true and true') is True
        assert runtime.execute('true and false') is False
        assert runtime.execute('false and true') is False
        assert runtime.execute('false and false') is False

    def test_or(self):
        runtime = HLXLRuntime()
        assert runtime.execute('true or true') is True
        assert runtime.execute('true or false') is True
        assert runtime.execute('false or true') is True
        assert runtime.execute('false or false') is False

    def test_not(self):
        runtime = HLXLRuntime()
        assert runtime.execute('not true') is False
        assert runtime.execute('not false') is True

    def test_combined_logical(self):
        runtime = HLXLRuntime()
        assert runtime.execute('true and not false') is True
        assert runtime.execute('false or not false') is True


class TestArrays:
    """Test array literals"""

    def test_empty_array(self):
        runtime = HLXLRuntime()
        assert runtime.execute('[]') == []

    def test_integer_array(self):
        runtime = HLXLRuntime()
        assert runtime.execute('[1, 2, 3]') == [1, 2, 3]

    def test_string_array(self):
        runtime = HLXLRuntime()
        assert runtime.execute('["a", "b", "c"]') == ["a", "b", "c"]

    def test_mixed_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[1, "hello", true]')
        assert result == [1, "hello", True]

    def test_nested_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[[1, 2], [3, 4]]')
        assert result == [[1, 2], [3, 4]]

    def test_array_with_expressions(self):
        runtime = HLXLRuntime()
        result = runtime.execute('[1 + 1, 2 * 2, 3 - 1]')
        assert result == [2, 4, 2]


class TestObjects:
    """Test object literals"""

    def test_empty_object(self):
        runtime = HLXLRuntime()
        assert runtime.execute('{}') == {}

    def test_simple_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{ x: 42, y: "test" }')
        assert result == {'x': 42, 'y': 'test'}

    def test_nested_object(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{ outer: { inner: 42 } }')
        assert result == {'outer': {'inner': 42}}

    def test_object_with_expressions(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{ sum: 1 + 2, product: 3 * 4 }')
        assert result == {'sum': 3, 'product': 12}

    def test_object_with_array(self):
        runtime = HLXLRuntime()
        result = runtime.execute('{ items: [1, 2, 3] }')
        assert result == {'items': [1, 2, 3]}


class TestBuiltins:
    """Test built-in functions"""

    def test_type_int(self):
        runtime = HLXLRuntime()
        assert runtime.execute('type(42)') == 'int'

    def test_type_str(self):
        runtime = HLXLRuntime()
        assert runtime.execute('type("hello")') == 'str'

    def test_type_bool(self):
        runtime = HLXLRuntime()
        assert runtime.execute('type(true)') == 'bool'

    def test_type_list(self):
        runtime = HLXLRuntime()
        assert runtime.execute('type([1, 2, 3])') == 'list'

    def test_len_array(self):
        runtime = HLXLRuntime()
        assert runtime.execute('len([1, 2, 3])') == 3

    def test_len_string(self):
        runtime = HLXLRuntime()
        assert runtime.execute('len("hello")') == 5

    def test_str_conversion(self):
        runtime = HLXLRuntime()
        assert runtime.execute('str(42)') == '42'

    def test_int_conversion(self):
        runtime = HLXLRuntime()
        assert runtime.execute('int("42")') == 42

    def test_float_conversion(self):
        runtime = HLXLRuntime()
        assert runtime.execute('float("3.14")') == 3.14

    def test_print(self):
        runtime = HLXLRuntime()
        # Should not raise
        runtime.execute('print("hello")')


class TestErrors:
    """Test error handling"""

    def test_undefined_variable(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError) as exc_info:
            runtime.execute('x')
        assert 'E_NAME_ERROR' in str(exc_info.value)

    def test_division_by_zero(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError) as exc_info:
            runtime.execute('5 / 0')
        assert 'E_RUNTIME_ERROR' in str(exc_info.value)

    def test_call_non_function(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 42')
        with pytest.raises(HLXLError) as exc_info:
            runtime.execute('x()')
        assert 'E_TYPE_ERROR' in str(exc_info.value)

    def test_unterminated_string(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError) as exc_info:
            runtime.execute('"hello')
        assert 'E_SYNTAX_ERROR' in str(exc_info.value)


class TestNoLatentSpace:
    """Verify that LS operations are NOT available"""

    def test_no_ls_collapse(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError):
            runtime.execute('ls.collapse(42)')

    def test_no_ls_resolve(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError):
            runtime.execute('ls.resolve("test")')

    def test_no_collapse_function(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError):
            runtime.execute('collapse(42)')

    def test_no_resolve_function(self):
        runtime = HLXLRuntime()
        with pytest.raises(HLXLError):
            runtime.execute('resolve("test")')


class TestComments:
    """Test comment handling"""

    def test_single_line_comment(self):
        runtime = HLXLRuntime()
        result = runtime.execute('42 # this is a comment')
        assert result == 42

    def test_comment_on_own_line(self):
        runtime = HLXLRuntime()
        result = runtime.execute('''
# This is a comment
42
''')
        assert result == 42


class TestTokenizer:
    """Test tokenizer directly"""

    def test_tokenize_integer(self):
        tokenizer = HLXLTokenizer('42')
        tokens = tokenizer.tokenize()
        assert any(t.type == TokenType.INTEGER and t.value == 42 for t in tokens)

    def test_tokenize_float(self):
        tokenizer = HLXLTokenizer('3.14')
        tokens = tokenizer.tokenize()
        assert any(t.type == TokenType.FLOAT and t.value == 3.14 for t in tokens)

    def test_tokenize_string(self):
        tokenizer = HLXLTokenizer('"hello"')
        tokens = tokenizer.tokenize()
        assert any(t.type == TokenType.STRING and t.value == 'hello' for t in tokens)

    def test_tokenize_keywords(self):
        tokenizer = HLXLTokenizer('let true false null')
        tokens = tokenizer.tokenize()
        assert any(t.type == TokenType.LET for t in tokens)
        assert any(t.type == TokenType.TRUE for t in tokens)
        assert any(t.type == TokenType.FALSE for t in tokens)
        assert any(t.type == TokenType.NULL for t in tokens)

    def test_tokenize_operators(self):
        tokenizer = HLXLTokenizer('+ - * / == != < > <= >=')
        tokens = tokenizer.tokenize()
        assert any(t.type == TokenType.PLUS for t in tokens)
        assert any(t.type == TokenType.MINUS for t in tokens)
        assert any(t.type == TokenType.EQUAL_EQUAL for t in tokens)


class TestIntegration:
    """Integration tests"""

    def test_fibonacci_iterative(self):
        runtime = HLXLRuntime()
        runtime.execute('let a = 0')
        runtime.execute('let b = 1')
        runtime.execute('let c = a + b')
        assert runtime.get_var('c') == 1

    def test_complex_program(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.execute('let y = 20')
        runtime.execute('let sum = x + y')
        runtime.execute('let product = x * y')
        assert runtime.get_var('sum') == 30
        assert runtime.get_var('product') == 200

    def test_nested_expressions(self):
        runtime = HLXLRuntime()
        result = runtime.execute('(1 + 2) * (3 + 4)')
        assert result == 21

    def test_complex_data_structure(self):
        runtime = HLXLRuntime()
        result = runtime.execute('''
{
    name: "Alice",
    age: 30,
    scores: [95, 87, 92],
    active: true
}
''')
        assert result == {
            'name': 'Alice',
            'age': 30,
            'scores': [95, 87, 92],
            'active': True
        }


class TestConvenienceFunction:
    """Test execute_hlxl_basic convenience function"""

    def test_simple(self):
        result = execute_hlxl_basic('42')
        assert result == 42

    def test_arithmetic(self):
        result = execute_hlxl_basic('5 + 3')
        assert result == 8

    def test_null(self):
        result = execute_hlxl_basic('null')
        assert result is None


class TestEnvironment:
    """Test environment management"""

    def test_set_var(self):
        runtime = HLXLRuntime()
        runtime.set_var('external', 100)
        result = runtime.execute('external')
        assert result == 100

    def test_get_env(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.execute('let y = 20')
        env = runtime.get_env()
        assert 'x' in env
        assert 'y' in env
        assert env['x'] == 10
        assert env['y'] == 20

    def test_clear_env(self):
        runtime = HLXLRuntime()
        runtime.execute('let x = 10')
        runtime.clear_env()
        # Builtins should still be there
        assert 'print' in runtime.get_env()
        # But user vars should be gone
        assert 'x' not in runtime.get_env()


if __name__ == '__main__':
    pytest.main([__file__, '-v'])
