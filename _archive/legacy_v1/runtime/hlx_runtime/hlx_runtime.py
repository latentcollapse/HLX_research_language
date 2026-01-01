"""
HLX Runtime - Basic Runic Language Execution

Provides a runtime for executing HLX (Runic) programs without Latent Space operations.
HLX is the Unicode glyph-based track for core language features.

Architecture:
- Tokenizer: Tokenize HLX source into tokens
- Parser: Parse tokens into AST
- Evaluator: Execute AST with environment
- Built-ins: Core operations (print, type, etc.)

Note: This is the BASIC runtime. For Latent Space operations, see hlx_ls_runtime.py

Glyph Reference:
    ∅  (U+2205)  - Null
    ⊤  (U+22A4)  - True
    ⊥  (U+22A5)  - False
    🜃 (U+1F703) - Integer prefix
    🜄 (U+1F704) - Float prefix
    ᛭  (U+16ED)  - Text prefix
    ⟁  (U+27C1)  - Handle reference
    🜊 (U+1F70A) - Contract start
    🜁 (U+1F701) - Field marker
    🜂 (U+1F702) - Contract end
    ⋔  (U+22D4)  - Array
    ⋕  (U+22D5)  - Object
    ⋯  (U+22EF)  - Bind
    ⋅  (U+22C5)  - Separator
    →  (U+2192)  - Arrow (function return)

Reference: glyphs.py, lc_r_codec.py
"""

from typing import Any, Dict, List, Optional, Tuple, Union
from dataclasses import dataclass


# ============================================================================
# Error Definitions
# ============================================================================

E_LC_PARSE = "E_LC_PARSE"
E_CONTRACT_STRUCTURE = "E_CONTRACT_STRUCTURE"
E_HANDLE_UNRESOLVED = "E_HANDLE_UNRESOLVED"
E_INVALID_INPUT = "E_INVALID_INPUT"


class HLXError(Exception):
    """HLX runtime error"""
    pass


# ============================================================================
# Glyph Definitions
# ============================================================================

GLYPH_NULL = '∅'       # U+2205
GLYPH_TRUE = '⊤'       # U+22A4
GLYPH_FALSE = '⊥'      # U+22A5
GLYPH_INT = '🜃'       # U+1F703
GLYPH_FLOAT = '🜄'     # U+1F704
GLYPH_TEXT = '᛭'       # U+16ED
GLYPH_HANDLE = '⟁'     # U+27C1
GLYPH_CONTRACT_START = '🜊'  # U+1F70A
GLYPH_FIELD = '🜁'     # U+1F701
GLYPH_CONTRACT_END = '🜂'    # U+1F702
GLYPH_ARRAY = '⋔'      # U+22D4
GLYPH_OBJECT = '⋕'     # U+22D5
GLYPH_BIND = '⋯'       # U+22EF
GLYPH_SEP = '⋅'        # U+22C5
GLYPH_ARROW = '→'      # U+2192

ALL_GLYPHS = {
    GLYPH_NULL, GLYPH_TRUE, GLYPH_FALSE, GLYPH_INT, GLYPH_FLOAT,
    GLYPH_TEXT, GLYPH_HANDLE, GLYPH_CONTRACT_START, GLYPH_FIELD,
    GLYPH_CONTRACT_END, GLYPH_ARRAY, GLYPH_OBJECT, GLYPH_BIND,
    GLYPH_SEP, GLYPH_ARROW
}


# ============================================================================
# AST Node Types
# ============================================================================

@dataclass
class ASTNode:
    """Base class for AST nodes"""
    pass


@dataclass
class Literal(ASTNode):
    """Literal value: ∅ (null), ⊤ (true), ⊥ (false), 🜃42 (int), 🜄3.14 (float), ᛭"text" """
    value: Any


@dataclass
class Variable(ASTNode):
    """Variable reference"""
    name: str


@dataclass
class Binding(ASTNode):
    """Variable binding: x ⋯ 42"""
    name: str
    value: ASTNode


@dataclass
class FunctionDef(ASTNode):
    """Function definition: fn(a, b) → a + b"""
    params: List[str]
    body: ASTNode


@dataclass
class FunctionCall(ASTNode):
    """Function call: add(1, 2)"""
    func: ASTNode
    args: List[ASTNode]


@dataclass
class Contract(ASTNode):
    """Contract invocation: 🜊902🜁0 "test"🜁1 ⟁shader🜂"""
    contract_id: int
    fields: Dict[int, ASTNode]


@dataclass
class Array(ASTNode):
    """Array literal: ⋔[1⋅2⋅3]"""
    elements: List[ASTNode]


@dataclass
class Object(ASTNode):
    """Object literal: ⋕{x⋯10⋅y⋯20}"""
    fields: Dict[str, ASTNode]


@dataclass
class BinaryOp(ASTNode):
    """Binary operation: a + b, a * b, etc."""
    op: str  # '+', '-', '*', '/', '==', '<', '>', etc.
    left: ASTNode
    right: ASTNode


@dataclass
class Block(ASTNode):
    """Block of statements"""
    statements: List[ASTNode]


# ============================================================================
# Tokenizer
# ============================================================================

class HLXTokenizer:
    """Tokenize HLX source code into tokens"""

    # Token types
    GLYPH = 'GLYPH'
    NUMBER = 'NUMBER'
    STRING = 'STRING'
    IDENT = 'IDENT'
    LPAREN = 'LPAREN'
    RPAREN = 'RPAREN'
    LBRACKET = 'LBRACKET'
    RBRACKET = 'RBRACKET'
    LBRACE = 'LBRACE'
    RBRACE = 'RBRACE'
    PLUS = 'PLUS'
    MINUS = 'MINUS'
    STAR = 'STAR'
    SLASH = 'SLASH'
    EQUALS = 'EQUALS'
    LT = 'LT'
    GT = 'GT'
    EOF = 'EOF'

    def __init__(self, source: str):
        """
        Initialize tokenizer with HLX source

        Args:
            source: HLX source code (with runic glyphs)
        """
        self.source = source
        self.pos = 0
        self.current_char = self.source[0] if source else None

    def error(self, msg: str):
        """Raise tokenizer error"""
        raise HLXError(f"{E_LC_PARSE}: {msg} at position {self.pos}")

    def advance(self):
        """Move to next character"""
        self.pos += 1
        if self.pos < len(self.source):
            self.current_char = self.source[self.pos]
        else:
            self.current_char = None

    def peek(self, offset: int = 1) -> Optional[str]:
        """Look ahead without advancing"""
        peek_pos = self.pos + offset
        if peek_pos < len(self.source):
            return self.source[peek_pos]
        return None

    def skip_whitespace(self):
        """Skip whitespace and comments"""
        while self.current_char and self.current_char.isspace():
            self.advance()

    def read_number(self) -> Tuple[str, str]:
        """Read a number (integer or float)"""
        num_str = ''
        has_dot = False

        while self.current_char and (self.current_char.isdigit() or self.current_char == '.'):
            if self.current_char == '.':
                if has_dot:
                    break
                has_dot = True
            num_str += self.current_char
            self.advance()

        return (self.NUMBER, num_str)

    def read_string(self) -> Tuple[str, str]:
        """Read a quoted string"""
        result = ''
        self.advance()  # Skip opening quote

        while self.current_char and self.current_char != '"':
            if self.current_char == '\\':
                self.advance()
                if self.current_char in ('"', '\\', 'n', 't'):
                    if self.current_char == 'n':
                        result += '\n'
                    elif self.current_char == 't':
                        result += '\t'
                    else:
                        result += self.current_char
                    self.advance()
                else:
                    result += self.current_char
                    self.advance()
            else:
                result += self.current_char
                self.advance()

        if self.current_char == '"':
            self.advance()  # Skip closing quote
        else:
            self.error("Unterminated string")

        return (self.STRING, result)

    def read_identifier(self) -> Tuple[str, str]:
        """Read an identifier"""
        result = ''
        while self.current_char and (self.current_char.isalnum() or self.current_char == '_'):
            result += self.current_char
            self.advance()
        return (self.IDENT, result)

    def tokenize(self) -> List[Tuple[str, str]]:
        """
        Tokenize the source code

        Returns:
            List of (token_type, token_value) tuples
        """
        tokens = []

        while self.current_char:
            # Skip whitespace
            if self.current_char.isspace():
                self.skip_whitespace()
                continue

            # Glyphs
            if self.current_char in ALL_GLYPHS:
                tokens.append((self.GLYPH, self.current_char))
                self.advance()
                continue

            # Numbers (including negative)
            if self.current_char.isdigit():
                tokens.append(self.read_number())
                continue

            # Strings
            if self.current_char == '"':
                tokens.append(self.read_string())
                continue

            # Identifiers
            if self.current_char.isalpha() or self.current_char == '_':
                tokens.append(self.read_identifier())
                continue

            # Single-char tokens
            if self.current_char == '(':
                tokens.append((self.LPAREN, '('))
                self.advance()
            elif self.current_char == ')':
                tokens.append((self.RPAREN, ')'))
                self.advance()
            elif self.current_char == '[':
                tokens.append((self.LBRACKET, '['))
                self.advance()
            elif self.current_char == ']':
                tokens.append((self.RBRACKET, ']'))
                self.advance()
            elif self.current_char == '{':
                tokens.append((self.LBRACE, '{'))
                self.advance()
            elif self.current_char == '}':
                tokens.append((self.RBRACE, '}'))
                self.advance()
            elif self.current_char == '+':
                tokens.append((self.PLUS, '+'))
                self.advance()
            elif self.current_char == '-':
                # Could be negative number
                if self.peek() and self.peek().isdigit():
                    self.advance()
                    num_tok = self.read_number()
                    tokens.append((self.NUMBER, '-' + num_tok[1]))
                else:
                    tokens.append((self.MINUS, '-'))
                    self.advance()
            elif self.current_char == '*':
                tokens.append((self.STAR, '*'))
                self.advance()
            elif self.current_char == '/':
                tokens.append((self.SLASH, '/'))
                self.advance()
            elif self.current_char == '=':
                tokens.append((self.EQUALS, '='))
                self.advance()
            elif self.current_char == '<':
                tokens.append((self.LT, '<'))
                self.advance()
            elif self.current_char == '>':
                tokens.append((self.GT, '>'))
                self.advance()
            else:
                self.error(f"Unexpected character: {self.current_char}")

        tokens.append((self.EOF, ''))
        return tokens


# ============================================================================
# Parser
# ============================================================================

class HLXParser:
    """Parse HLX tokens into AST"""

    def __init__(self, tokens: List[Tuple[str, str]]):
        """
        Initialize parser with token stream

        Args:
            tokens: List of (token_type, token_value) tuples from tokenizer
        """
        self.tokens = tokens
        self.pos = 0
        self.current_token = tokens[0] if tokens else (HLXTokenizer.EOF, '')

    def error(self, msg: str):
        """Raise parser error"""
        raise HLXError(f"{E_LC_PARSE}: {msg}")

    def advance(self):
        """Move to next token"""
        self.pos += 1
        if self.pos < len(self.tokens):
            self.current_token = self.tokens[self.pos]
        else:
            self.current_token = (HLXTokenizer.EOF, '')

    def expect(self, token_type: str) -> str:
        """Expect a specific token type and advance"""
        if self.current_token[0] != token_type:
            self.error(f"Expected {token_type}, got {self.current_token[0]}")
        value = self.current_token[1]
        self.advance()
        return value

    def parse(self) -> ASTNode:
        """Parse the token stream into an AST"""
        statements = []
        while self.current_token[0] != HLXTokenizer.EOF:
            stmt = self.parse_statement()
            if stmt:
                statements.append(stmt)

        if len(statements) == 0:
            return Literal(None)
        elif len(statements) == 1:
            return statements[0]
        else:
            return Block(statements=statements)

    def parse_statement(self) -> Optional[ASTNode]:
        """Parse a single statement"""
        # Check if this is a variable binding: x ⋯ value
        if self.current_token[0] == HLXTokenizer.IDENT:
            # Look ahead to see if this is a binding
            if self.pos + 1 < len(self.tokens):
                next_token = self.tokens[self.pos + 1]
                if next_token[0] == HLXTokenizer.GLYPH and next_token[1] == GLYPH_BIND:
                    # This is a binding
                    name = self.current_token[1]
                    self.advance()  # Consume identifier
                    self.advance()  # Consume bind glyph
                    value = self.parse_expression()
                    return Binding(name=name, value=value)
        
        # Otherwise, parse as expression
        return self.parse_expression()

    def parse_expression(self) -> ASTNode:
        """Parse an expression"""
        return self.parse_additive()

    def parse_additive(self) -> ASTNode:
        """Parse addition/subtraction"""
        left = self.parse_multiplicative()

        while self.current_token[0] in (HLXTokenizer.PLUS, HLXTokenizer.MINUS):
            op = '+' if self.current_token[0] == HLXTokenizer.PLUS else '-'
            self.advance()
            right = self.parse_multiplicative()
            left = BinaryOp(op=op, left=left, right=right)

        return left

    def parse_multiplicative(self) -> ASTNode:
        """Parse multiplication/division"""
        left = self.parse_comparison()

        while self.current_token[0] in (HLXTokenizer.STAR, HLXTokenizer.SLASH):
            op = '*' if self.current_token[0] == HLXTokenizer.STAR else '/'
            self.advance()
            right = self.parse_comparison()
            left = BinaryOp(op=op, left=left, right=right)

        return left

    def parse_comparison(self) -> ASTNode:
        """Parse comparison operators"""
        left = self.parse_primary()

        while self.current_token[0] in (HLXTokenizer.EQUALS, HLXTokenizer.LT, HLXTokenizer.GT):
            if self.current_token[0] == HLXTokenizer.EQUALS:
                op = '=='
            elif self.current_token[0] == HLXTokenizer.LT:
                op = '<'
            else:
                op = '>'
            self.advance()
            right = self.parse_primary()
            left = BinaryOp(op=op, left=left, right=right)

        return left

    def parse_primary(self) -> ASTNode:
        """Parse primary expressions"""
        token_type, token_value = self.current_token

        # Glyph-prefixed literals
        if token_type == HLXTokenizer.GLYPH:
            # Null
            if token_value == GLYPH_NULL:
                self.advance()
                return Literal(None)

            # Boolean
            elif token_value == GLYPH_TRUE:
                self.advance()
                return Literal(True)

            elif token_value == GLYPH_FALSE:
                self.advance()
                return Literal(False)

            # Integer
            elif token_value == GLYPH_INT:
                self.advance()
                num_str = self.expect(HLXTokenizer.NUMBER)
                return Literal(int(num_str))

            # Float
            elif token_value == GLYPH_FLOAT:
                self.advance()
                num_str = self.expect(HLXTokenizer.NUMBER)
                return Literal(float(num_str))

            # Text
            elif token_value == GLYPH_TEXT:
                self.advance()
                text = self.expect(HLXTokenizer.STRING)
                return Literal(text)

            # Handle
            elif token_value == GLYPH_HANDLE:
                self.advance()
                handle = self.expect(HLXTokenizer.IDENT)
                return Literal(f"&{handle}")

            # Contract
            elif token_value == GLYPH_CONTRACT_START:
                return self.parse_contract()

            # Array
            elif token_value == GLYPH_ARRAY:
                return self.parse_array()

            # Object
            elif token_value == GLYPH_OBJECT:
                return self.parse_object()

            else:
                self.error(f"Unexpected glyph: {token_value}")

        # Identifier (variable or function)
        elif token_type == HLXTokenizer.IDENT:
            name = token_value
            self.advance()

            # Function call
            if self.current_token[0] == HLXTokenizer.LPAREN:
                self.advance()
                args = []

                while self.current_token[0] != HLXTokenizer.RPAREN:
                    args.append(self.parse_expression())
                    if self.current_token[0] == HLXTokenizer.GLYPH and self.current_token[1] == GLYPH_SEP:
                        self.advance()

                self.expect(HLXTokenizer.RPAREN)
                return FunctionCall(func=Variable(name), args=args)

            # Just a variable
            return Variable(name)

        # Number literal (without glyph prefix)
        elif token_type == HLXTokenizer.NUMBER:
            num_str = token_value
            self.advance()
            if '.' in num_str:
                return Literal(float(num_str))
            else:
                return Literal(int(num_str))

        # String literal (without glyph prefix)
        elif token_type == HLXTokenizer.STRING:
            text = token_value
            self.advance()
            return Literal(text)

        # Parenthesized expression
        elif token_type == HLXTokenizer.LPAREN:
            self.advance()
            expr = self.parse_expression()
            self.expect(HLXTokenizer.RPAREN)
            return expr

        else:
            self.error(f"Unexpected token: {token_type}")

    def parse_contract(self) -> Contract:
        """Parse a contract: 🜊id🜁field value🜂"""
        self.expect(HLXTokenizer.GLYPH)  # CONTRACT_START

        # Read contract ID
        contract_id = int(self.expect(HLXTokenizer.NUMBER))

        # Read fields
        fields = {}
        while self.current_token[0] == HLXTokenizer.GLYPH and self.current_token[1] == GLYPH_FIELD:
            self.advance()  # Skip field marker
            field_idx = int(self.expect(HLXTokenizer.NUMBER))
            value = self.parse_expression()
            fields[field_idx] = value

        # Expect contract end
        if self.current_token[0] != HLXTokenizer.GLYPH or self.current_token[1] != GLYPH_CONTRACT_END:
            self.error("Expected contract end marker")
        self.advance()

        return Contract(contract_id=contract_id, fields=fields)

    def parse_array(self) -> Array:
        """Parse an array: ⋔[elem⋅elem]"""
        self.expect(HLXTokenizer.GLYPH)  # ARRAY
        self.expect(HLXTokenizer.LBRACKET)

        elements = []
        while self.current_token[0] != HLXTokenizer.RBRACKET:
            elements.append(self.parse_expression())
            if self.current_token[0] == HLXTokenizer.GLYPH and self.current_token[1] == GLYPH_SEP:
                self.advance()

        self.expect(HLXTokenizer.RBRACKET)
        return Array(elements=elements)

    def parse_object(self) -> Object:
        """Parse an object: ⋕{key⋯val⋅key⋯val}"""
        self.expect(HLXTokenizer.GLYPH)  # OBJECT
        self.expect(HLXTokenizer.LBRACE)

        fields = {}
        while self.current_token[0] != HLXTokenizer.RBRACE:
            # Read key
            key = self.expect(HLXTokenizer.IDENT)

            # Expect bind
            if self.current_token[0] != HLXTokenizer.GLYPH or self.current_token[1] != GLYPH_BIND:
                self.error("Expected bind in object")
            self.advance()

            # Read value
            value = self.parse_expression()
            fields[key] = value

            # Optional separator
            if self.current_token[0] == HLXTokenizer.GLYPH and self.current_token[1] == GLYPH_SEP:
                self.advance()

        self.expect(HLXTokenizer.RBRACE)
        return Object(fields=fields)


# ============================================================================
# Evaluator
# ============================================================================

class HLXEvaluator:
    """Evaluate HLX AST"""

    def __init__(self):
        """Initialize evaluator"""
        self.env: Dict[str, Any] = {}

        # Built-in functions
        self.builtins = {
            'print': self._builtin_print,
            'type': self._builtin_type,
            'len': self._builtin_len,
            'str': self._builtin_str,
            'int': self._builtin_int,
            'float': self._builtin_float,
        }

    def eval(self, node: ASTNode) -> Any:
        """Evaluate an AST node"""

        if isinstance(node, Literal):
            return node.value

        elif isinstance(node, Variable):
            if node.name in self.env:
                return self.env[node.name]
            elif node.name in self.builtins:
                return self.builtins[node.name]
            else:
                raise HLXError(f"{E_HANDLE_UNRESOLVED}: Variable '{node.name}' not found")

        elif isinstance(node, Binding):
            value = self.eval(node.value)
            self.env[node.name] = value
            return value

        elif isinstance(node, FunctionCall):
            func = self.eval(node.func)
            args = [self.eval(arg) for arg in node.args]

            if callable(func):
                return func(*args)
            elif isinstance(func, FunctionDef):
                return self._call_user_function(func, args)
            else:
                raise HLXError(f"{E_INVALID_INPUT}: {func} is not callable")

        elif isinstance(node, Contract):
            fields = {idx: self.eval(val) for idx, val in node.fields.items()}
            return {
                'contract_id': node.contract_id,
                **{f'field_{idx}': val for idx, val in fields.items()}
            }

        elif isinstance(node, Array):
            return [self.eval(elem) for elem in node.elements]

        elif isinstance(node, Object):
            return {key: self.eval(val) for key, val in node.fields.items()}

        elif isinstance(node, BinaryOp):
            left = self.eval(node.left)
            right = self.eval(node.right)
            return self._eval_binary_op(node.op, left, right)

        elif isinstance(node, Block):
            result = None
            for stmt in node.statements:
                result = self.eval(stmt)
            return result

        else:
            raise HLXError(f"{E_INVALID_INPUT}: Unknown AST node type: {type(node)}")

    def _call_user_function(self, func: FunctionDef, args: List[Any]) -> Any:
        """Call a user-defined function"""
        if len(args) != len(func.params):
            raise HLXError(f"{E_INVALID_INPUT}: Function expects {len(func.params)} args, got {len(args)}")

        # Save current environment
        old_env = self.env.copy()

        # Bind parameters
        for param, arg in zip(func.params, args):
            self.env[param] = arg

        # Evaluate function body
        result = self.eval(func.body)

        # Restore environment
        self.env = old_env

        return result

    def _eval_binary_op(self, op: str, left: Any, right: Any) -> Any:
        """Evaluate binary operation"""
        if op == '+':
            return left + right
        elif op == '-':
            return left - right
        elif op == '*':
            return left * right
        elif op == '/':
            return left / right
        elif op == '==':
            return left == right
        elif op == '!=':
            return left != right
        elif op == '<':
            return left < right
        elif op == '>':
            return left > right
        elif op == '<=':
            return left <= right
        elif op == '>=':
            return left >= right
        else:
            raise HLXError(f"{E_INVALID_INPUT}: Unknown operator: {op}")

    # Built-in functions

    def _builtin_print(self, *args) -> None:
        """Built-in: print(*args)"""
        print(*args)
        return None

    def _builtin_type(self, value: Any) -> str:
        """Built-in: type(value) → type_name"""
        return type(value).__name__

    def _builtin_len(self, value: Any) -> int:
        """Built-in: len(value) → length"""
        return len(value)

    def _builtin_str(self, value: Any) -> str:
        """Built-in: str(value) → string"""
        return str(value)

    def _builtin_int(self, value: Any) -> int:
        """Built-in: int(value) → integer"""
        return int(value)

    def _builtin_float(self, value: Any) -> float:
        """Built-in: float(value) → float"""
        return float(value)


# ============================================================================
# Main Runtime Interface
# ============================================================================

class HLXRuntime:
    """HLX Runtime - Execute basic Runic programs"""

    def __init__(self):
        """Initialize HLX runtime"""
        self.evaluator = HLXEvaluator()

    def execute(self, source: str) -> Any:
        """
        Execute HLX source code

        Args:
            source: HLX source code (with runic glyphs)

        Returns:
            Result of evaluation

        Raises:
            HLXError: If parsing or execution fails

        Example:
            >>> runtime = HLXRuntime()
            >>> runtime.execute('🜃42')
            42
            >>> runtime.execute('x ⋯ 🜃10')  # Bind x to 10
            10
            >>> runtime.execute('x')  # Reference x
            10
        """
        # Tokenize
        tokenizer = HLXTokenizer(source)
        tokens = tokenizer.tokenize()

        # Parse
        parser = HLXParser(tokens)
        ast = parser.parse()

        # Evaluate
        result = self.evaluator.eval(ast)

        return result

    def execute_file(self, filepath: str) -> Any:
        """Execute HLX source file"""
        with open(filepath, 'r', encoding='utf-8') as f:
            source = f.read()
        return self.execute(source)

    def get_env(self) -> Dict[str, Any]:
        """Get current environment (variables)"""
        return self.evaluator.env.copy()

    def set_var(self, name: str, value: Any):
        """Set a variable in the environment"""
        self.evaluator.env[name] = value

    def get_var(self, name: str) -> Any:
        """Get a variable from the environment"""
        return self.evaluator.env.get(name)

    def clear_env(self):
        """Clear the environment"""
        self.evaluator.env.clear()


# ============================================================================
# Convenience Functions
# ============================================================================

def execute_hlx(source: str) -> Any:
    """
    Execute HLX source code

    Args:
        source: HLX source code (with runic glyphs)

    Returns:
        Result of evaluation

    Example:
        >>> execute_hlx('🜃42')
        42
        >>> execute_hlx('🜃5 + 🜃3')
        8
    """
    runtime = HLXRuntime()
    return runtime.execute(source)


# ============================================================================
# Test/Demo
# ============================================================================

if __name__ == '__main__':
    print("HLX Basic Runtime Test Suite\n")

    runtime = HLXRuntime()

    # Test literals
    print("=== Literals ===")
    print(f"Null: {runtime.execute('∅')}")           # None
    print(f"True: {runtime.execute('⊤')}")           # True
    print(f"False: {runtime.execute('⊥')}")          # False
    print(f"Int: {runtime.execute('🜃42')}")         # 42
    print(f"Float: {runtime.execute('🜄3.14')}")     # 3.14
    print(f"Text: {runtime.execute('᛭\"hello\"')}")  # "hello"

    # Test bindings
    print("\n=== Bindings ===")
    runtime.execute('x ⋯ 🜃10')
    print(f"x = {runtime.get_var('x')}")  # 10

    # Test arithmetic
    print("\n=== Arithmetic ===")
    result = runtime.execute('🜃5 + 🜃3')
    print(f"5 + 3 = {result}")  # 8

    # Test variables in arithmetic
    runtime.execute('a ⋯ 🜃5')
    runtime.execute('b ⋯ 🜃3')
    result = runtime.execute('a + b')
    print(f"a + b = {result}")  # 8

    # Test contract
    print("\n=== Contract ===")
    result = runtime.execute('🜊14🜁0 🜃42🜂')
    print(f"Contract: {result}")

    print("\n✨ HLX basic runtime ready!")
