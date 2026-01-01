"""
HLXL Runtime - Basic ASCII Language Execution

Provides a runtime for executing HLXL (ASCII) programs without Latent Space operations.
HLXL is the human-readable ASCII track for core language features.

Architecture:
- Tokenizer: Tokenize HLXL source into tokens
- Parser: Parse tokens into AST
- Evaluator: Execute AST with environment
- Built-ins: Core operations (print, type, etc.)

Note: This is the BASIC runtime. For Latent Space operations, see hlxl_ls_runtime.py

Syntax Examples:
    let x = 42
    let y = x + 10
    let greeting = "Hello, World!"
    let items = [1, 2, 3]
    let data = { name: "Alice", age: 30 }
    print(x)

Reference: hlxl_ls_runtime.py (for LS-enabled version)
"""

from typing import Any, Dict, List, Optional, Tuple, Union
from dataclasses import dataclass
import re


# ============================================================================
# Error Definitions
# ============================================================================

E_PARSE_ERROR = "E_PARSE_ERROR"
E_SYNTAX_ERROR = "E_SYNTAX_ERROR"
E_RUNTIME_ERROR = "E_RUNTIME_ERROR"
E_TYPE_ERROR = "E_TYPE_ERROR"
E_NAME_ERROR = "E_NAME_ERROR"
E_INVALID_OPERATION = "E_INVALID_OPERATION"


class HLXLError(Exception):
    """Base exception for HLXL runtime errors"""
    def __init__(self, code: str, message: str):
        self.code = code
        self.message = message
        super().__init__(f"[{code}] {message}")


# ============================================================================
# Token Types
# ============================================================================

@dataclass
class Token:
    """Token from HLXL source"""
    type: str
    value: Any
    pos: int


class TokenType:
    """Token type constants"""
    # Literals
    INTEGER = "INTEGER"
    FLOAT = "FLOAT"
    STRING = "STRING"
    TRUE = "TRUE"
    FALSE = "FALSE"
    NULL = "NULL"

    # Identifiers and keywords
    IDENTIFIER = "IDENTIFIER"
    LET = "LET"
    FN = "FN"
    RETURN = "RETURN"
    IF = "IF"
    ELSE = "ELSE"

    # Operators
    PLUS = "PLUS"
    MINUS = "MINUS"
    STAR = "STAR"
    SLASH = "SLASH"
    PERCENT = "PERCENT"
    EQUAL = "EQUAL"
    EQUAL_EQUAL = "EQUAL_EQUAL"
    NOT_EQUAL = "NOT_EQUAL"
    LESS = "LESS"
    LESS_EQUAL = "LESS_EQUAL"
    GREATER = "GREATER"
    GREATER_EQUAL = "GREATER_EQUAL"
    AND = "AND"
    OR = "OR"
    NOT = "NOT"

    # Delimiters
    LPAREN = "LPAREN"
    RPAREN = "RPAREN"
    LBRACE = "LBRACE"
    RBRACE = "RBRACE"
    LBRACKET = "LBRACKET"
    RBRACKET = "RBRACKET"
    COMMA = "COMMA"
    COLON = "COLON"
    SEMICOLON = "SEMICOLON"
    DOT = "DOT"

    # Special
    EOF = "EOF"
    NEWLINE = "NEWLINE"


# ============================================================================
# Tokenizer
# ============================================================================

class HLXLTokenizer:
    """Tokenize HLXL (ASCII) source code"""

    KEYWORDS = {
        'let', 'fn', 'return', 'if', 'else',
        'true', 'false', 'null',
        'and', 'or', 'not'
    }

    def __init__(self, source: str):
        self.source = source
        self.pos = 0
        self.tokens: List[Token] = []

    def tokenize(self) -> List[Token]:
        """Tokenize entire source"""
        while self.pos < len(self.source):
            self._skip_whitespace_and_comments()
            if self.pos >= len(self.source):
                break

            ch = self.source[self.pos]

            # Numbers
            if ch.isdigit():
                self._read_number()
            # Strings
            elif ch == '"':
                self._read_string()
            # Identifiers and keywords
            elif ch.isalpha() or ch == '_':
                self._read_identifier()
            # Operators and delimiters
            elif ch == '+':
                self._add_token(TokenType.PLUS, ch)
                self.pos += 1
            elif ch == '-':
                self._add_token(TokenType.MINUS, ch)
                self.pos += 1
            elif ch == '*':
                self._add_token(TokenType.STAR, ch)
                self.pos += 1
            elif ch == '/':
                self._add_token(TokenType.SLASH, ch)
                self.pos += 1
            elif ch == '%':
                self._add_token(TokenType.PERCENT, ch)
                self.pos += 1
            elif ch == '=':
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1] == '=':
                    self._add_token(TokenType.EQUAL_EQUAL, '==')
                    self.pos += 2
                else:
                    self._add_token(TokenType.EQUAL, ch)
                    self.pos += 1
            elif ch == '!':
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1] == '=':
                    self._add_token(TokenType.NOT_EQUAL, '!=')
                    self.pos += 2
                else:
                    raise HLXLError(E_SYNTAX_ERROR, f"Unexpected character '!' at position {self.pos}")
            elif ch == '<':
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1] == '=':
                    self._add_token(TokenType.LESS_EQUAL, '<=')
                    self.pos += 2
                else:
                    self._add_token(TokenType.LESS, ch)
                    self.pos += 1
            elif ch == '>':
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1] == '=':
                    self._add_token(TokenType.GREATER_EQUAL, '>=')
                    self.pos += 2
                else:
                    self._add_token(TokenType.GREATER, ch)
                    self.pos += 1
            elif ch == '(':
                self._add_token(TokenType.LPAREN, ch)
                self.pos += 1
            elif ch == ')':
                self._add_token(TokenType.RPAREN, ch)
                self.pos += 1
            elif ch == '{':
                self._add_token(TokenType.LBRACE, ch)
                self.pos += 1
            elif ch == '}':
                self._add_token(TokenType.RBRACE, ch)
                self.pos += 1
            elif ch == '[':
                self._add_token(TokenType.LBRACKET, ch)
                self.pos += 1
            elif ch == ']':
                self._add_token(TokenType.RBRACKET, ch)
                self.pos += 1
            elif ch == ',':
                self._add_token(TokenType.COMMA, ch)
                self.pos += 1
            elif ch == ':':
                self._add_token(TokenType.COLON, ch)
                self.pos += 1
            elif ch == ';':
                self._add_token(TokenType.SEMICOLON, ch)
                self.pos += 1
            elif ch == '.':
                self._add_token(TokenType.DOT, ch)
                self.pos += 1
            elif ch == '\n':
                self._add_token(TokenType.NEWLINE, ch)
                self.pos += 1
            else:
                raise HLXLError(E_SYNTAX_ERROR, f"Unexpected character '{ch}' at position {self.pos}")

        self._add_token(TokenType.EOF, None)
        return self.tokens

    def _skip_whitespace_and_comments(self):
        """Skip whitespace and comments"""
        while self.pos < len(self.source):
            ch = self.source[self.pos]
            if ch in ' \t\r':
                self.pos += 1
            elif ch == '#':
                # Skip comment until end of line
                while self.pos < len(self.source) and self.source[self.pos] != '\n':
                    self.pos += 1
            else:
                break

    def _read_number(self):
        """Read numeric literal"""
        start = self.pos
        has_dot = False

        while self.pos < len(self.source):
            ch = self.source[self.pos]
            if ch.isdigit():
                self.pos += 1
            elif ch == '.' and not has_dot:
                has_dot = True
                self.pos += 1
            else:
                break

        text = self.source[start:self.pos]
        if has_dot:
            self._add_token(TokenType.FLOAT, float(text))
        else:
            self._add_token(TokenType.INTEGER, int(text))

    def _read_string(self):
        """Read string literal"""
        self.pos += 1  # Skip opening quote
        start = self.pos

        while self.pos < len(self.source) and self.source[self.pos] != '"':
            if self.source[self.pos] == '\\' and self.pos + 1 < len(self.source):
                self.pos += 2  # Skip escape sequence
            else:
                self.pos += 1

        if self.pos >= len(self.source):
            raise HLXLError(E_SYNTAX_ERROR, "Unterminated string literal")

        text = self.source[start:self.pos]
        self.pos += 1  # Skip closing quote

        # Process escape sequences
        text = text.replace('\\n', '\n').replace('\\t', '\t').replace('\\"', '"').replace('\\\\', '\\')

        self._add_token(TokenType.STRING, text)

    def _read_identifier(self):
        """Read identifier or keyword"""
        start = self.pos

        while self.pos < len(self.source):
            ch = self.source[self.pos]
            if ch.isalnum() or ch == '_':
                self.pos += 1
            else:
                break

        text = self.source[start:self.pos]

        # Check if it's a keyword
        if text == 'let':
            self._add_token(TokenType.LET, text)
        elif text == 'fn':
            self._add_token(TokenType.FN, text)
        elif text == 'return':
            self._add_token(TokenType.RETURN, text)
        elif text == 'if':
            self._add_token(TokenType.IF, text)
        elif text == 'else':
            self._add_token(TokenType.ELSE, text)
        elif text == 'true':
            self._add_token(TokenType.TRUE, True)
        elif text == 'false':
            self._add_token(TokenType.FALSE, False)
        elif text == 'null':
            self._add_token(TokenType.NULL, None)
        elif text == 'and':
            self._add_token(TokenType.AND, text)
        elif text == 'or':
            self._add_token(TokenType.OR, text)
        elif text == 'not':
            self._add_token(TokenType.NOT, text)
        else:
            self._add_token(TokenType.IDENTIFIER, text)

    def _add_token(self, type: str, value: Any):
        """Add token to list"""
        self.tokens.append(Token(type=type, value=value, pos=self.pos))


# ============================================================================
# AST Nodes
# ============================================================================

@dataclass
class ASTNode:
    """Base AST node"""
    pass


@dataclass
class Literal(ASTNode):
    """Literal value"""
    value: Any


@dataclass
class Identifier(ASTNode):
    """Variable reference"""
    name: str


@dataclass
class BinaryOp(ASTNode):
    """Binary operation"""
    op: str
    left: ASTNode
    right: ASTNode


@dataclass
class UnaryOp(ASTNode):
    """Unary operation"""
    op: str
    operand: ASTNode


@dataclass
class Call(ASTNode):
    """Function call"""
    func: ASTNode
    args: List[ASTNode]


@dataclass
class ArrayLiteral(ASTNode):
    """Array literal"""
    elements: List[ASTNode]


@dataclass
class ObjectLiteral(ASTNode):
    """Object/contract literal"""
    fields: Dict[str, ASTNode]


@dataclass
class Let(ASTNode):
    """Variable binding"""
    name: str
    value: ASTNode


@dataclass
class Block(ASTNode):
    """Block of statements"""
    statements: List[ASTNode]


# ============================================================================
# Parser
# ============================================================================

class HLXLParser:
    """Parse HLXL tokens into AST"""

    def __init__(self, tokens: List[Token]):
        self.tokens = tokens
        self.pos = 0

    def parse(self) -> List[ASTNode]:
        """Parse all statements"""
        statements = []
        while not self._is_at_end():
            # Skip newlines
            while self._match(TokenType.NEWLINE):
                pass
            if self._is_at_end():
                break
            statements.append(self._parse_statement())
        return statements

    def _parse_statement(self) -> ASTNode:
        """Parse a single statement"""
        # Skip optional newlines
        while self._match(TokenType.NEWLINE):
            pass

        if self._match(TokenType.LET):
            return self._parse_let()
        else:
            expr = self._parse_expression()
            # Consume optional semicolon or newline
            self._match(TokenType.SEMICOLON, TokenType.NEWLINE)
            return expr

    def _parse_let(self) -> Let:
        """Parse let binding"""
        if not self._check(TokenType.IDENTIFIER):
            raise HLXLError(E_PARSE_ERROR, "Expected identifier after 'let'")
        name = self._advance().value

        if not self._match(TokenType.EQUAL):
            raise HLXLError(E_PARSE_ERROR, "Expected '=' after identifier in let binding")

        value = self._parse_expression()

        # Consume optional semicolon or newline
        self._match(TokenType.SEMICOLON, TokenType.NEWLINE)

        return Let(name=name, value=value)

    def _parse_expression(self) -> ASTNode:
        """Parse expression"""
        return self._parse_or()

    def _parse_or(self) -> ASTNode:
        """Parse logical OR"""
        left = self._parse_and()
        while self._match(TokenType.OR):
            op = 'or'
            right = self._parse_and()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_and(self) -> ASTNode:
        """Parse logical AND"""
        left = self._parse_equality()
        while self._match(TokenType.AND):
            op = 'and'
            right = self._parse_equality()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_equality(self) -> ASTNode:
        """Parse equality operators"""
        left = self._parse_comparison()
        while self._match(TokenType.EQUAL_EQUAL, TokenType.NOT_EQUAL):
            op = '==' if self._previous().type == TokenType.EQUAL_EQUAL else '!='
            right = self._parse_comparison()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_comparison(self) -> ASTNode:
        """Parse comparison operators"""
        left = self._parse_addition()
        while self._match(TokenType.LESS, TokenType.LESS_EQUAL, TokenType.GREATER, TokenType.GREATER_EQUAL):
            token = self._previous()
            op = {
                TokenType.LESS: '<',
                TokenType.LESS_EQUAL: '<=',
                TokenType.GREATER: '>',
                TokenType.GREATER_EQUAL: '>='
            }[token.type]
            right = self._parse_addition()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_addition(self) -> ASTNode:
        """Parse addition and subtraction"""
        left = self._parse_multiplication()
        while self._match(TokenType.PLUS, TokenType.MINUS):
            op = '+' if self._previous().type == TokenType.PLUS else '-'
            right = self._parse_multiplication()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_multiplication(self) -> ASTNode:
        """Parse multiplication, division, modulo"""
        left = self._parse_unary()
        while self._match(TokenType.STAR, TokenType.SLASH, TokenType.PERCENT):
            token = self._previous()
            op = {
                TokenType.STAR: '*',
                TokenType.SLASH: '/',
                TokenType.PERCENT: '%'
            }[token.type]
            right = self._parse_unary()
            left = BinaryOp(op=op, left=left, right=right)
        return left

    def _parse_unary(self) -> ASTNode:
        """Parse unary operators"""
        if self._match(TokenType.MINUS, TokenType.NOT):
            op = '-' if self._previous().type == TokenType.MINUS else 'not'
            operand = self._parse_unary()
            return UnaryOp(op=op, operand=operand)
        return self._parse_call()

    def _parse_call(self) -> ASTNode:
        """Parse function call"""
        expr = self._parse_primary()

        while True:
            if self._match(TokenType.LPAREN):
                args = []
                if not self._check(TokenType.RPAREN):
                    args.append(self._parse_expression())
                    while self._match(TokenType.COMMA):
                        args.append(self._parse_expression())

                if not self._match(TokenType.RPAREN):
                    raise HLXLError(E_PARSE_ERROR, "Expected ')' after function arguments")

                expr = Call(func=expr, args=args)
            else:
                break

        return expr

    def _parse_primary(self) -> ASTNode:
        """Parse primary expression"""
        # Literals
        if self._match(TokenType.INTEGER, TokenType.FLOAT, TokenType.STRING):
            return Literal(value=self._previous().value)

        if self._match(TokenType.TRUE, TokenType.FALSE):
            return Literal(value=self._previous().value)

        if self._match(TokenType.NULL):
            return Literal(value=None)

        # Identifier
        if self._match(TokenType.IDENTIFIER):
            return Identifier(name=self._previous().value)

        # Array literal
        if self._match(TokenType.LBRACKET):
            elements = []
            if not self._check(TokenType.RBRACKET):
                elements.append(self._parse_expression())
                while self._match(TokenType.COMMA):
                    elements.append(self._parse_expression())

            if not self._match(TokenType.RBRACKET):
                raise HLXLError(E_PARSE_ERROR, "Expected ']' after array elements")

            return ArrayLiteral(elements=elements)

        # Object literal
        if self._match(TokenType.LBRACE):
            fields = {}
            # Skip any newlines after opening brace
            while self._match(TokenType.NEWLINE):
                pass
            if not self._check(TokenType.RBRACE):
                # Parse first field
                if not self._check(TokenType.IDENTIFIER):
                    raise HLXLError(E_PARSE_ERROR, "Expected field name in object literal")
                key = self._advance().value

                if not self._match(TokenType.COLON):
                    raise HLXLError(E_PARSE_ERROR, "Expected ':' after field name")

                value = self._parse_expression()
                fields[key] = value

                # Skip any newlines
                while self._match(TokenType.NEWLINE):
                    pass

                # Parse remaining fields
                while self._match(TokenType.COMMA):
                    # Skip any newlines after comma
                    while self._match(TokenType.NEWLINE):
                        pass
                    if self._check(TokenType.RBRACE):
                        break  # Allow trailing comma
                    if not self._check(TokenType.IDENTIFIER):
                        raise HLXLError(E_PARSE_ERROR, "Expected field name in object literal")
                    key = self._advance().value

                    if not self._match(TokenType.COLON):
                        raise HLXLError(E_PARSE_ERROR, "Expected ':' after field name")

                    value = self._parse_expression()
                    fields[key] = value

                    # Skip any newlines
                    while self._match(TokenType.NEWLINE):
                        pass

            # Skip any newlines before closing brace
            while self._match(TokenType.NEWLINE):
                pass

            if not self._match(TokenType.RBRACE):
                raise HLXLError(E_PARSE_ERROR, "Expected '}' after object fields")

            return ObjectLiteral(fields=fields)

        # Parenthesized expression
        if self._match(TokenType.LPAREN):
            expr = self._parse_expression()
            if not self._match(TokenType.RPAREN):
                raise HLXLError(E_PARSE_ERROR, "Expected ')' after expression")
            return expr

        raise HLXLError(E_PARSE_ERROR, f"Unexpected token: {self._peek().type}")

    # Parser utilities
    def _match(self, *types: str) -> bool:
        """Check if current token matches any of the given types"""
        for type in types:
            if self._check(type):
                self._advance()
                return True
        return False

    def _check(self, type: str) -> bool:
        """Check if current token is of given type"""
        if self._is_at_end():
            return False
        return self._peek().type == type

    def _advance(self) -> Token:
        """Consume current token and return it"""
        if not self._is_at_end():
            self.pos += 1
        return self._previous()

    def _is_at_end(self) -> bool:
        """Check if at end of tokens"""
        return self._peek().type == TokenType.EOF

    def _peek(self) -> Token:
        """Return current token without consuming"""
        return self.tokens[self.pos]

    def _previous(self) -> Token:
        """Return previous token"""
        return self.tokens[self.pos - 1]


# ============================================================================
# Evaluator
# ============================================================================

class HLXLEvaluator:
    """Evaluate HLXL AST"""

    def __init__(self):
        self.env: Dict[str, Any] = {}
        self._setup_builtins()

    def _setup_builtins(self):
        """Setup built-in functions"""
        self.env['print'] = lambda *args: print(*args)
        self.env['type'] = lambda x: type(x).__name__
        self.env['len'] = len
        self.env['str'] = str
        self.env['int'] = int
        self.env['float'] = float

    def evaluate(self, node: ASTNode) -> Any:
        """Evaluate AST node"""
        if isinstance(node, Literal):
            return node.value

        elif isinstance(node, Identifier):
            if node.name not in self.env:
                raise HLXLError(E_NAME_ERROR, f"Undefined variable: {node.name}")
            return self.env[node.name]

        elif isinstance(node, BinaryOp):
            left = self.evaluate(node.left)
            right = self.evaluate(node.right)
            return self._eval_binary_op(node.op, left, right)

        elif isinstance(node, UnaryOp):
            operand = self.evaluate(node.operand)
            return self._eval_unary_op(node.op, operand)

        elif isinstance(node, Call):
            func = self.evaluate(node.func)
            if not callable(func):
                raise HLXLError(E_TYPE_ERROR, f"Cannot call non-function: {type(func).__name__}")
            args = [self.evaluate(arg) for arg in node.args]
            return func(*args)

        elif isinstance(node, ArrayLiteral):
            return [self.evaluate(elem) for elem in node.elements]

        elif isinstance(node, ObjectLiteral):
            return {key: self.evaluate(val) for key, val in node.fields.items()}

        elif isinstance(node, Let):
            value = self.evaluate(node.value)
            self.env[node.name] = value
            return value

        elif isinstance(node, Block):
            result = None
            for stmt in node.statements:
                result = self.evaluate(stmt)
            return result

        else:
            raise HLXLError(E_RUNTIME_ERROR, f"Unknown AST node type: {type(node).__name__}")

    def _eval_binary_op(self, op: str, left: Any, right: Any) -> Any:
        """Evaluate binary operation"""
        if op == '+':
            return left + right
        elif op == '-':
            return left - right
        elif op == '*':
            return left * right
        elif op == '/':
            if right == 0:
                raise HLXLError(E_RUNTIME_ERROR, "Division by zero")
            return left / right
        elif op == '%':
            return left % right
        elif op == '==':
            return left == right
        elif op == '!=':
            return left != right
        elif op == '<':
            return left < right
        elif op == '<=':
            return left <= right
        elif op == '>':
            return left > right
        elif op == '>=':
            return left >= right
        elif op == 'and':
            return left and right
        elif op == 'or':
            return left or right
        else:
            raise HLXLError(E_INVALID_OPERATION, f"Unknown binary operator: {op}")

    def _eval_unary_op(self, op: str, operand: Any) -> Any:
        """Evaluate unary operation"""
        if op == '-':
            return -operand
        elif op == 'not':
            return not operand
        else:
            raise HLXLError(E_INVALID_OPERATION, f"Unknown unary operator: {op}")


# ============================================================================
# Runtime Interface
# ============================================================================

class HLXLRuntime:
    """Main HLXL runtime interface"""

    def __init__(self):
        self.evaluator = HLXLEvaluator()

    def execute(self, source: str) -> Any:
        """Execute HLXL source code"""
        # Tokenize
        tokenizer = HLXLTokenizer(source)
        tokens = tokenizer.tokenize()

        # Parse
        parser = HLXLParser(tokens)
        ast = parser.parse()

        # Evaluate
        result = None
        for node in ast:
            result = self.evaluator.evaluate(node)

        return result

    def set_var(self, name: str, value: Any):
        """Set variable in environment"""
        self.evaluator.env[name] = value

    def get_var(self, name: str) -> Any:
        """Get variable from environment"""
        if name not in self.evaluator.env:
            raise HLXLError(E_NAME_ERROR, f"Undefined variable: {name}")
        return self.evaluator.env[name]

    def get_env(self) -> Dict[str, Any]:
        """Get entire environment"""
        return self.evaluator.env.copy()

    def clear_env(self):
        """Clear environment (keeping builtins)"""
        self.evaluator.env.clear()
        self.evaluator._setup_builtins()


# ============================================================================
# Convenience Function
# ============================================================================

def execute_hlxl_basic(source: str) -> Any:
    """
    Execute HLXL source code (convenience function)
    
    Args:
        source: HLXL source code
        
    Returns:
        Result of evaluation
        
    Example:
        >>> execute_hlxl_basic('42')
        42
        >>> execute_hlxl_basic('5 + 3')
        8
    """
    runtime = HLXLRuntime()
    return runtime.execute(source)


# ============================================================================
# Exports
# ============================================================================

__all__ = [
    'HLXLRuntime',
    'HLXLTokenizer',
    'HLXLParser',
    'HLXLEvaluator',
    'HLXLError',
    'execute_hlxl_basic',
    'TokenType',
    'Token',
    'ASTNode',
    'Literal',
    'Identifier',
    'BinaryOp',
    'UnaryOp',
    'Call',
    'ArrayLiteral',
    'ObjectLiteral',
    'Let',
    'Block',
]


# ============================================================================
# Demo
# ============================================================================

if __name__ == '__main__':
    print("HLXL Basic Runtime Demo\n")
    
    runtime = HLXLRuntime()
    
    # Test literals
    print("=== Literals ===")
    print(f"Integer: {runtime.execute('42')}")
    print(f"Float: {runtime.execute('3.14')}")
    print(f"String: {runtime.execute('\"hello\"')}")
    print(f"True: {runtime.execute('true')}")
    print(f"False: {runtime.execute('false')}")
    print(f"Null: {runtime.execute('null')}")
    
    # Test variables
    print("\n=== Variables ===")
    runtime.execute('let x = 42')
    print(f"x = {runtime.get_var('x')}")
    runtime.execute('let y = x + 10')
    print(f"y = x + 10 = {runtime.get_var('y')}")
    
    # Test arithmetic
    print("\n=== Arithmetic ===")
    print(f"5 + 3 = {runtime.execute('5 + 3')}")
    print(f"10 - 4 = {runtime.execute('10 - 4')}")
    print(f"6 * 7 = {runtime.execute('6 * 7')}")
    print(f"15 / 3 = {runtime.execute('15 / 3')}")
    print(f"17 % 5 = {runtime.execute('17 % 5')}")
    print(f"2 + 3 * 4 = {runtime.execute('2 + 3 * 4')}")
    print(f"(2 + 3) * 4 = {runtime.execute('(2 + 3) * 4')}")
    
    # Test arrays and objects
    print("\n=== Data Structures ===")
    print(f"Array: {runtime.execute('[1, 2, 3]')}")
    print(f"Object: {runtime.execute('{ name: \"Alice\", age: 30 }')}")
    
    # Test built-ins
    print("\n=== Built-ins ===")
    print(f"type(42) = {runtime.execute('type(42)')}")
    print(f"len([1, 2, 3]) = {runtime.execute('len([1, 2, 3])')}")
    
    print("\n✨ HLXL basic runtime ready!")
