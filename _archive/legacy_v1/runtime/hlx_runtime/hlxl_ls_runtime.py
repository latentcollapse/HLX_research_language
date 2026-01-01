"""
HLXL-LS Runtime - ASCII Language Execution with Latent Space Operations

Provides a runtime for executing HLXL (ASCII) programs with full Latent Space support.
HLXL is the ASCII track: HLXL → HLXL-LS → LC-T/LC-B

Architecture:
- Tokenizer: Tokenize HLXL source into tokens
- Parser: Parse tokens into AST
- Evaluator: Execute AST with environment and CAS integration
- Built-ins: Latent Space operations (ls.collapse, ls.resolve, etc.)

HLXL Syntax (ASCII equivalents to HLX Runic):
    null            - Null value (∅ in HLX)
    true, false     - Boolean values (⊤, ⊥ in HLX)
    42, 3.14        - Numbers
    "hello"         - Strings
    @handle         - Handle reference (⟁ in HLX)
    
    let x = 42      - Variable binding (x ⋯ 42 in HLX)
    ls.collapse(v)  - Collapse to CAS (⊕ in HLX)
    ls.resolve(h)   - Resolve from CAS (⊖ in HLX)
    
    {14: {@0: 42}}  - Contract (🜊14🜁0 42🜂 in HLX)
    [1, 2, 3]       - Array
    {x: 10}         - Object

Reference: RUNTIME_ARCHITECTURE.md, lc_codec.py, lc_t_codec.py
"""

from typing import Any, Dict, List, Optional, Tuple, Union
from dataclasses import dataclass
import hashlib


# ============================================================================
# Error Definitions
# ============================================================================

E_LC_PARSE = "E_LC_PARSE"
E_CONTRACT_STRUCTURE = "E_CONTRACT_STRUCTURE"
E_HANDLE_UNRESOLVED = "E_HANDLE_UNRESOLVED"
E_INVALID_INPUT = "E_INVALID_INPUT"


class HLXLError(Exception):
    """HLXL runtime error"""
    pass


# ============================================================================
# Simple CAS Store (in-memory for runtime)
# ============================================================================

class SimpleCAS:
    """Simple content-addressed store for runtime use"""
    
    def __init__(self):
        self.store: Dict[str, Any] = {}
    
    def put(self, value: Any) -> str:
        """Store value and return handle"""
        serialized = repr(value).encode('utf-8')
        hash_val = hashlib.sha256(serialized).hexdigest()[:16]
        handle = f"&h_{hash_val}"
        self.store[handle] = value
        return handle
    
    def get(self, handle: str) -> Any:
        """Retrieve value by handle"""
        if handle not in self.store:
            raise HLXLError(f"{E_HANDLE_UNRESOLVED}: Handle '{handle}' not found")
        return self.store[handle]
    
    def has(self, handle: str) -> bool:
        """Check if handle exists"""
        return handle in self.store


# Global CAS instance
_global_cas = SimpleCAS()

def get_cas_store() -> SimpleCAS:
    """Get the global CAS store"""
    return _global_cas


# ============================================================================
# Latent Space Operations
# ============================================================================

def collapse(value: Any, cas: Optional[SimpleCAS] = None) -> str:
    """Collapse a value to CAS, returning handle"""
    cas = cas or get_cas_store()
    return cas.put(value)


def resolve(handle: str, cas: Optional[SimpleCAS] = None) -> Any:
    """Resolve a handle from CAS"""
    cas = cas or get_cas_store()
    return cas.get(handle)


def snapshot(cas: Optional[SimpleCAS] = None) -> Dict[str, Any]:
    """Get a snapshot of CAS state"""
    cas = cas or get_cas_store()
    return dict(cas.store)


# ============================================================================
# AST Node Types
# ============================================================================

@dataclass
class ASTNode:
    """Base class for AST nodes"""
    pass


@dataclass
class Literal(ASTNode):
    """Literal value: null, true, false, 42, 3.14, "text" """
    value: Any


@dataclass
class Variable(ASTNode):
    """Variable reference"""
    name: str


@dataclass
class Binding(ASTNode):
    """Variable binding: let x = 42"""
    name: str
    value: ASTNode


@dataclass
class FunctionDef(ASTNode):
    """Function definition: fn(a, b) { return a + b; }"""
    params: List[str]
    body: ASTNode


@dataclass
class FunctionCall(ASTNode):
    """Function call: add(1, 2)"""
    func: ASTNode
    args: List[ASTNode]


@dataclass
class Contract(ASTNode):
    """Contract invocation: {14: {@0: 42}}"""
    contract_id: int
    fields: Dict[int, ASTNode]


@dataclass
class Array(ASTNode):
    """Array literal: [1, 2, 3]"""
    elements: List[ASTNode]


@dataclass
class Object(ASTNode):
    """Object literal: {x: 10, y: 20}"""
    fields: Dict[str, ASTNode]


@dataclass
class MethodCall(ASTNode):
    """Method call: ls.collapse(value), ls.resolve(handle)"""
    object_name: str  # 'ls', 'cas', etc.
    method: str  # 'collapse', 'resolve', etc.
    args: List[ASTNode]


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

class HLXLTokenizer:
    """Tokenize HLXL source code into tokens"""

    # Token types
    KEYWORD = 'KEYWORD'
    NULL = 'NULL'
    TRUE = 'TRUE'
    FALSE = 'FALSE'
    NUMBER = 'NUMBER'
    STRING = 'STRING'
    IDENT = 'IDENT'
    LPAREN = 'LPAREN'
    RPAREN = 'RPAREN'
    LBRACKET = 'LBRACKET'
    RBRACKET = 'RBRACKET'
    LBRACE = 'LBRACE'
    RBRACE = 'RBRACE'
    COLON = 'COLON'
    COMMA = 'COMMA'
    DOT = 'DOT'
    AT = 'AT'
    HASH = 'HASH'
    PLUS = 'PLUS'
    MINUS = 'MINUS'
    STAR = 'STAR'
    SLASH = 'SLASH'
    EQ = 'EQ'
    EQEQ = 'EQEQ'
    LT = 'LT'
    GT = 'GT'
    EOF = 'EOF'

    KEYWORDS = {'let', 'fn', 'if', 'else', 'return', 'null', 'true', 'false'}

    def __init__(self, source: str):
        """
        Initialize tokenizer with HLXL source

        Args:
            source: HLXL source code (ASCII)
        """
        self.source = source
        self.pos = 0
        self.tokens: List[Tuple[str, str]] = []

    def tokenize(self) -> List[Tuple[str, str]]:
        """
        Tokenize the source code

        Returns:
            List of (token_type, token_value) tuples
        """
        self.tokens = []

        while self.pos < len(self.source):
            self._skip_whitespace()

            if self.pos >= len(self.source):
                break

            char = self.source[self.pos]

            # Parentheses
            if char == '(':
                self.tokens.append((self.LPAREN, char))
                self.pos += 1
            elif char == ')':
                self.tokens.append((self.RPAREN, char))
                self.pos += 1

            # Brackets
            elif char == '[':
                self.tokens.append((self.LBRACKET, char))
                self.pos += 1
            elif char == ']':
                self.tokens.append((self.RBRACKET, char))
                self.pos += 1

            # Braces
            elif char == '{':
                self.tokens.append((self.LBRACE, char))
                self.pos += 1
            elif char == '}':
                self.tokens.append((self.RBRACE, char))
                self.pos += 1

            # Punctuation
            elif char == ':':
                self.tokens.append((self.COLON, char))
                self.pos += 1
            elif char == ',':
                self.tokens.append((self.COMMA, char))
                self.pos += 1
            elif char == '.':
                self.tokens.append((self.DOT, char))
                self.pos += 1
            elif char == '@':
                self.tokens.append((self.AT, char))
                self.pos += 1
            elif char == '#':
                self.tokens.append((self.HASH, char))
                self.pos += 1

            # Operators
            elif char == '+':
                self.tokens.append((self.PLUS, char))
                self.pos += 1
            elif char == '-':
                # Could be minus or negative number
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1].isdigit():
                    self.tokens.append((self.NUMBER, self._read_number()))
                else:
                    self.tokens.append((self.MINUS, char))
                    self.pos += 1
            elif char == '*':
                self.tokens.append((self.STAR, char))
                self.pos += 1
            elif char == '/':
                self.tokens.append((self.SLASH, char))
                self.pos += 1
            elif char == '=':
                if self.pos + 1 < len(self.source) and self.source[self.pos + 1] == '=':
                    self.tokens.append((self.EQEQ, '=='))
                    self.pos += 2
                else:
                    self.tokens.append((self.EQ, char))
                    self.pos += 1
            elif char == '<':
                self.tokens.append((self.LT, char))
                self.pos += 1
            elif char == '>':
                self.tokens.append((self.GT, char))
                self.pos += 1

            # String
            elif char == '"':
                self.tokens.append((self.STRING, self._read_string()))

            # Number
            elif char.isdigit():
                self.tokens.append((self.NUMBER, self._read_number()))

            # Identifier or keyword
            elif char.isalpha() or char == '_':
                ident = self._read_identifier()
                if ident == 'null':
                    self.tokens.append((self.NULL, ident))
                elif ident == 'true':
                    self.tokens.append((self.TRUE, ident))
                elif ident == 'false':
                    self.tokens.append((self.FALSE, ident))
                elif ident in self.KEYWORDS:
                    self.tokens.append((self.KEYWORD, ident))
                else:
                    self.tokens.append((self.IDENT, ident))

            else:
                # Skip unknown characters
                self.pos += 1

        self.tokens.append((self.EOF, ''))
        return self.tokens

    def _peek(self) -> Optional[str]:
        """Peek at current character without advancing"""
        if self.pos < len(self.source):
            return self.source[self.pos]
        return None

    def _advance(self) -> str:
        """Get current character and advance position"""
        char = self.source[self.pos]
        self.pos += 1
        return char

    def _skip_whitespace(self):
        """Skip whitespace characters"""
        while self.pos < len(self.source) and self.source[self.pos].isspace():
            self.pos += 1

    def _read_string(self) -> str:
        """Read a quoted string"""
        self.pos += 1  # Skip opening quote
        result = []

        while self.pos < len(self.source):
            char = self.source[self.pos]

            if char == '"':
                self.pos += 1
                return ''.join(result)

            if char == '\\':
                self.pos += 1
                if self.pos < len(self.source):
                    escaped = self.source[self.pos]
                    if escaped == 'n':
                        result.append('\n')
                    elif escaped == 't':
                        result.append('\t')
                    elif escaped == '"':
                        result.append('"')
                    elif escaped == '\\':
                        result.append('\\')
                    else:
                        result.append(escaped)
                    self.pos += 1
            else:
                result.append(char)
                self.pos += 1

        raise HLXLError(f"{E_LC_PARSE}: Unterminated string")

    def _read_number(self) -> str:
        """Read a number (int or float)"""
        start = self.pos

        # Optional minus
        if self.pos < len(self.source) and self.source[self.pos] == '-':
            self.pos += 1

        # Digits
        while self.pos < len(self.source) and self.source[self.pos].isdigit():
            self.pos += 1

        # Optional decimal
        if self.pos < len(self.source) and self.source[self.pos] == '.':
            self.pos += 1
            while self.pos < len(self.source) and self.source[self.pos].isdigit():
                self.pos += 1

        return self.source[start:self.pos]

    def _read_identifier(self) -> str:
        """Read an identifier"""
        start = self.pos

        while self.pos < len(self.source):
            char = self.source[self.pos]
            if char.isalnum() or char == '_':
                self.pos += 1
            else:
                break

        return self.source[start:self.pos]


# ============================================================================
# Parser
# ============================================================================

class HLXLParser:
    """Parse HLXL tokens into AST"""

    def __init__(self, tokens: List[Tuple[str, str]]):
        """
        Initialize parser with token stream

        Args:
            tokens: List of (token_type, token_value) tuples from tokenizer
        """
        self.tokens = tokens
        self.pos = 0

    def parse(self) -> ASTNode:
        """
        Parse the token stream into an AST

        Returns:
            Root AST node

        Raises:
            HLXLError: If parsing fails
        """
        statements = []

        while not self._is_at_end():
            stmt = self._parse_statement()
            if stmt:
                statements.append(stmt)

        if len(statements) == 1:
            return statements[0]
        return Block(statements=statements)

    def _parse_statement(self) -> Optional[ASTNode]:
        """Parse a single statement"""
        if self._is_at_end():
            return None

        # Let binding
        if self._check(HLXLTokenizer.KEYWORD) and self._peek()[1] == 'let':
            return self._parse_let_binding()

        return self._parse_expression()

    def _parse_let_binding(self) -> Binding:
        """Parse: let x = value"""
        self._advance()  # Consume 'let'

        # Variable name
        name_token = self._expect(HLXLTokenizer.IDENT)
        name = name_token[1]

        # Expect '='
        self._expect(HLXLTokenizer.EQ)

        # Value
        value = self._parse_expression()

        return Binding(name=name, value=value)

    def _parse_expression(self) -> ASTNode:
        """Parse an expression"""
        return self._parse_binary()

    def _parse_binary(self) -> ASTNode:
        """Parse binary operations"""
        left = self._parse_unary()

        while self._check_any([HLXLTokenizer.PLUS, HLXLTokenizer.MINUS,
                               HLXLTokenizer.STAR, HLXLTokenizer.SLASH,
                               HLXLTokenizer.EQEQ, HLXLTokenizer.LT, HLXLTokenizer.GT]):
            op_token = self._advance()
            op = op_token[1]
            right = self._parse_unary()
            left = BinaryOp(op=op, left=left, right=right)

        return left

    def _parse_unary(self) -> ASTNode:
        """Parse unary operations and primary expressions"""
        return self._parse_call()

    def _parse_call(self) -> ASTNode:
        """Parse function calls and method calls"""
        expr = self._parse_primary()

        while True:
            if self._check(HLXLTokenizer.DOT):
                # Method call: obj.method(args)
                self._advance()  # Consume .
                method_token = self._expect(HLXLTokenizer.IDENT)
                method = method_token[1]

                # Check for arguments
                args = []
                if self._check(HLXLTokenizer.LPAREN):
                    self._advance()  # Consume (

                    if not self._check(HLXLTokenizer.RPAREN):
                        args.append(self._parse_expression())
                        while self._check(HLXLTokenizer.COMMA):
                            self._advance()
                            args.append(self._parse_expression())

                    self._expect(HLXLTokenizer.RPAREN)

                # Get object name
                if isinstance(expr, Variable):
                    expr = MethodCall(object_name=expr.name, method=method, args=args)
                else:
                    raise HLXLError(f"{E_LC_PARSE}: Method call on non-variable")

            elif self._check(HLXLTokenizer.LPAREN):
                # Function call
                self._advance()  # Consume (
                args = []

                if not self._check(HLXLTokenizer.RPAREN):
                    args.append(self._parse_expression())
                    while self._check(HLXLTokenizer.COMMA):
                        self._advance()
                        args.append(self._parse_expression())

                self._expect(HLXLTokenizer.RPAREN)
                expr = FunctionCall(func=expr, args=args)

            else:
                break

        return expr

    def _parse_primary(self) -> ASTNode:
        """Parse primary expressions"""
        if self._is_at_end():
            raise HLXLError(f"{E_LC_PARSE}: Unexpected end of input")

        token = self._peek()
        token_type, token_value = token

        # Null
        if token_type == HLXLTokenizer.NULL:
            self._advance()
            return Literal(value=None)

        # True
        if token_type == HLXLTokenizer.TRUE:
            self._advance()
            return Literal(value=True)

        # False
        if token_type == HLXLTokenizer.FALSE:
            self._advance()
            return Literal(value=False)

        # Number
        if token_type == HLXLTokenizer.NUMBER:
            self._advance()
            if '.' in token_value:
                return Literal(value=float(token_value))
            return Literal(value=int(token_value))

        # String
        if token_type == HLXLTokenizer.STRING:
            self._advance()
            return Literal(value=token_value)

        # Handle reference: @name
        if token_type == HLXLTokenizer.AT:
            self._advance()  # Consume @
            name_token = self._expect(HLXLTokenizer.IDENT)
            return Literal(value="&h_" + name_token[1])

        # Identifier
        if token_type == HLXLTokenizer.IDENT:
            self._advance()
            return Variable(name=token_value)

        # Parenthesized expression
        if token_type == HLXLTokenizer.LPAREN:
            self._advance()
            expr = self._parse_expression()
            self._expect(HLXLTokenizer.RPAREN)
            return expr

        # Array
        if token_type == HLXLTokenizer.LBRACKET:
            return self._parse_array_literal()

        # Object or Contract
        if token_type == HLXLTokenizer.LBRACE:
            return self._parse_brace()

        raise HLXLError(f"{E_LC_PARSE}: Unexpected token {token_type}: {token_value}")

    def _parse_brace(self) -> ASTNode:
        """Parse content inside braces - either contract or object"""
        self._advance()  # Consume {

        # Empty object
        if self._check(HLXLTokenizer.RBRACE):
            self._advance()
            return Object(fields={})

        # Check if it's a contract: {id: {...}}
        if self._check(HLXLTokenizer.NUMBER):
            saved_pos = self.pos
            num_token = self._advance()

            if self._check(HLXLTokenizer.COLON):
                # This is a contract
                self._advance()  # Consume :
                return self._parse_contract_body(int(num_token[1]))

            # Not a contract, restore and parse as object
            self.pos = saved_pos

        # Parse as object
        return self._parse_object_body()

    def _parse_contract_body(self, contract_id: int) -> Contract:
        """Parse contract body after {id:"""
        # Expect { for fields
        self._expect(HLXLTokenizer.LBRACE)

        fields: Dict[int, ASTNode] = {}

        while not self._check(HLXLTokenizer.RBRACE) and not self._is_at_end():
            # Expect @field_idx
            self._expect(HLXLTokenizer.AT)
            field_idx_token = self._expect(HLXLTokenizer.NUMBER)
            field_idx = int(field_idx_token[1])

            # Expect :
            self._expect(HLXLTokenizer.COLON)

            # Parse value
            value = self._parse_expression()
            fields[field_idx] = value

            # Optional comma
            if self._check(HLXLTokenizer.COMMA):
                self._advance()

        self._expect(HLXLTokenizer.RBRACE)  # Close inner brace
        self._expect(HLXLTokenizer.RBRACE)  # Close outer brace

        return Contract(contract_id=contract_id, fields=fields)

    def _parse_object_body(self) -> Object:
        """Parse object body"""
        fields: Dict[str, ASTNode] = {}

        while not self._check(HLXLTokenizer.RBRACE) and not self._is_at_end():
            # Key
            key_token = self._expect(HLXLTokenizer.IDENT)
            key = key_token[1]

            # Expect :
            self._expect(HLXLTokenizer.COLON)

            # Value
            value = self._parse_expression()
            fields[key] = value

            # Optional comma
            if self._check(HLXLTokenizer.COMMA):
                self._advance()

        self._expect(HLXLTokenizer.RBRACE)

        return Object(fields=fields)

    def _parse_array_literal(self) -> Array:
        """Parse an array: [elem, elem]"""
        self._advance()  # Consume [

        elements = []

        while not self._check(HLXLTokenizer.RBRACKET) and not self._is_at_end():
            elements.append(self._parse_expression())

            if self._check(HLXLTokenizer.COMMA):
                self._advance()
            elif not self._check(HLXLTokenizer.RBRACKET):
                break

        self._expect(HLXLTokenizer.RBRACKET)

        return Array(elements=elements)

    def _expect(self, token_type: str) -> Tuple[str, str]:
        """Expect a specific token type, consume and return it"""
        if self._is_at_end():
            raise HLXLError(f"{E_LC_PARSE}: Unexpected EOF, expected {token_type}")

        token = self.tokens[self.pos]
        if token[0] != token_type:
            raise HLXLError(f"{E_LC_PARSE}: Expected {token_type}, got {token[0]}: {token[1]}")

        self.pos += 1
        return token

    def _peek(self) -> Tuple[str, str]:
        """Peek at current token without consuming"""
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return (HLXLTokenizer.EOF, '')

    def _check(self, token_type: str) -> bool:
        """Check if current token is of given type"""
        if self._is_at_end():
            return False
        return self.tokens[self.pos][0] == token_type

    def _check_any(self, token_types: List[str]) -> bool:
        """Check if current token is any of the given types"""
        return any(self._check(t) for t in token_types)

    def _advance(self) -> Tuple[str, str]:
        """Consume and return current token"""
        token = self.tokens[self.pos]
        self.pos += 1
        return token

    def _is_at_end(self) -> bool:
        """Check if we've reached the end of tokens"""
        return self.pos >= len(self.tokens) or self.tokens[self.pos][0] == HLXLTokenizer.EOF


# ============================================================================
# Evaluator
# ============================================================================

class HLXLEvaluator:
    """Evaluate HLXL AST with environment and CAS integration"""

    def __init__(self, cas: Optional[SimpleCAS] = None):
        """
        Initialize evaluator

        Args:
            cas: Content-addressed store (default: global CAS)
        """
        self.cas = cas or get_cas_store()
        self.env: Dict[str, Any] = {}  # Variable environment
        self.functions: Dict[str, FunctionDef] = {}  # User-defined functions

        # Built-in functions
        self.builtins = {
            'collapse': self._builtin_collapse,
            'resolve': self._builtin_resolve,
            'snapshot': self._builtin_snapshot,
            'print': self._builtin_print,
            'type': self._builtin_type,
        }

        # Built-in objects
        self.objects = {
            'ls': {
                'collapse': self._builtin_collapse,
                'resolve': self._builtin_resolve,
                'snapshot': self._builtin_snapshot,
            },
            'cas': {
                'put': self._builtin_collapse,
                'get': self._builtin_resolve,
            }
        }

    def eval(self, node: ASTNode) -> Any:
        """
        Evaluate an AST node

        Args:
            node: AST node to evaluate

        Returns:
            Evaluated value

        Raises:
            HLXLError: If evaluation fails
        """
        if isinstance(node, Literal):
            return node.value

        elif isinstance(node, Variable):
            if node.name in self.env:
                return self.env[node.name]
            elif node.name in self.builtins:
                return self.builtins[node.name]
            elif node.name in self.objects:
                return self.objects[node.name]
            else:
                raise HLXLError(f"{E_HANDLE_UNRESOLVED}: Variable '{node.name}' not found")

        elif isinstance(node, Binding):
            value = self.eval(node.value)
            self.env[node.name] = value
            return value

        elif isinstance(node, FunctionDef):
            return node

        elif isinstance(node, FunctionCall):
            func = self.eval(node.func)
            args = [self.eval(arg) for arg in node.args]

            if callable(func):
                return func(*args)
            elif isinstance(func, FunctionDef):
                return self._call_user_function(func, args)
            else:
                raise HLXLError(f"{E_INVALID_INPUT}: {func} is not callable")

        elif isinstance(node, MethodCall):
            # Get the object
            if node.object_name not in self.objects:
                raise HLXLError(f"{E_HANDLE_UNRESOLVED}: Object '{node.object_name}' not found")

            obj = self.objects[node.object_name]
            if node.method not in obj:
                raise HLXLError(f"{E_INVALID_INPUT}: Method '{node.method}' not found on '{node.object_name}'")

            method = obj[node.method]
            args = [self.eval(arg) for arg in node.args]
            return method(*args)

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
            raise HLXLError(f"{E_INVALID_INPUT}: Unknown AST node type: {type(node)}")

    def _call_user_function(self, func: FunctionDef, args: List[Any]) -> Any:
        """Call a user-defined function with arguments"""
        if len(args) != len(func.params):
            raise HLXLError(f"{E_INVALID_INPUT}: Function expects {len(func.params)} args, got {len(args)}")

        old_env = self.env.copy()

        for param, arg in zip(func.params, args):
            self.env[param] = arg

        result = self.eval(func.body)
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
            raise HLXLError(f"{E_INVALID_INPUT}: Unknown operator: {op}")

    # Built-in functions

    def _builtin_collapse(self, value: Any) -> str:
        """Built-in: ls.collapse(value) → handle"""
        return collapse(value, self.cas)

    def _builtin_resolve(self, handle: str) -> Any:
        """Built-in: ls.resolve(handle) → value"""
        return resolve(handle, self.cas)

    def _builtin_snapshot(self) -> Any:
        """Built-in: ls.snapshot() → checkpoint"""
        return snapshot(self.cas)

    def _builtin_print(self, *args) -> None:
        """Built-in: print(*args)"""
        print(*args)
        return None

    def _builtin_type(self, value: Any) -> str:
        """Built-in: type(value) → type_name"""
        return type(value).__name__


# ============================================================================
# Main Runtime Interface
# ============================================================================

class HLXLRuntime:
    """HLXL-LS Runtime - Execute ASCII programs with Latent Space operations"""

    def __init__(self, cas: Optional[SimpleCAS] = None):
        """
        Initialize HLXL runtime

        Args:
            cas: Content-addressed store (default: global CAS)
        """
        self.cas = cas or get_cas_store()
        self.evaluator = HLXLEvaluator(self.cas)

    def execute(self, source: str) -> Any:
        """
        Execute HLXL source code

        Args:
            source: HLXL source code (ASCII)

        Returns:
            Result of evaluation

        Raises:
            HLXLError: If parsing or execution fails

        Example:
            >>> runtime = HLXLRuntime()
            >>> runtime.execute('42')
            42
            >>> runtime.execute('let x = 10')  # Bind x to 10
            10
            >>> runtime.execute('x')  # Reference x
            10
        """
        # Tokenize
        tokenizer = HLXLTokenizer(source)
        tokens = tokenizer.tokenize()

        # Parse
        parser = HLXLParser(tokens)
        ast = parser.parse()

        # Evaluate
        result = self.evaluator.eval(ast)

        return result

    def execute_file(self, filepath: str) -> Any:
        """
        Execute HLXL source file

        Args:
            filepath: Path to .hlxl file

        Returns:
            Result of evaluation
        """
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
# Convenience functions
# ============================================================================

def execute_hlxl(source: str, cas: Optional[SimpleCAS] = None) -> Any:
    """
    Execute HLXL source code

    Args:
        source: HLXL source code (ASCII)
        cas: Content-addressed store (default: global CAS)

    Returns:
        Result of evaluation

    Example:
        >>> execute_hlxl('42')
        42
        >>> execute_hlxl('ls.collapse(42)')  # Collapse 42 to CAS
        '&h_...'
    """
    runtime = HLXLRuntime(cas)
    return runtime.execute(source)


# ============================================================================
# Main (Test)
# ============================================================================

if __name__ == '__main__':
    print("HLXL-LS Runtime Test Suite\n")

    runtime = HLXLRuntime()

    # Test literals
    print("=== Literals ===")
    print(f"Null: {runtime.execute('null')}")
    print(f"True: {runtime.execute('true')}")
    print(f"False: {runtime.execute('false')}")
    print(f"Int: {runtime.execute('42')}")
    print(f"Float: {runtime.execute('3.14')}")
    print(f"String: {runtime.execute('"hello"')}")

    # Test bindings
    print("\n=== Bindings ===")
    runtime.execute('let x = 10')
    print(f"x = {runtime.get_var('x')}")

    # Test arithmetic
    print("\n=== Arithmetic ===")
    runtime.execute('let a = 5')
    runtime.execute('let b = 3')
    result = runtime.execute('a + b')
    print(f"a + b = {result}")

    # Test collapse/resolve
    print("\n=== Latent Space ===")
    handle = runtime.execute('ls.collapse(42)')
    print(f"Collapsed: {handle}")
    runtime.set_var('h', handle)
    value = runtime.execute('ls.resolve(h)')
    print(f"Resolved: {value}")

    print("\n✨ HLXL-LS runtime ready!")
