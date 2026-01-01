"""
HLX-LS Runtime - Runic Language Execution with Latent Space Operations

Provides a runtime for executing HLX (Runic) programs with full Latent Space support.
HLX is the Unicode glyph-based track: HLX → HLX-LS → LC-R

Architecture:
- Tokenizer: Tokenize HLX source into tokens
- Parser: Parse tokens into AST
- Evaluator: Execute AST with environment and CAS integration
- Built-ins: Latent Space operations (collapse, resolve, etc.)

Glyph Reference (from glyphs.py):
    ∅  (U+2205) - Null
    ⊤  (U+22A4) - True
    ⊥  (U+22A5) - False
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
    ⊕  (U+2295)  - Collapse to CAS
    ⊖  (U+2296)  - Resolve from CAS
    →  (U+2192)  - Arrow (function return)

Reference: RUNTIME_ARCHITECTURE.md, glyphs.py, lc_r_codec.py
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


class HLXError(Exception):
    """HLX runtime error"""
    pass


# ============================================================================
# Glyph Definitions (subset for parser)
# ============================================================================

GLYPH_NULL = '∅'       # U+2205
GLYPH_TRUE = '⊤'       # U+22A4
GLYPH_FALSE = '⊥'      # U+22A5
GLYPH_INT = '🜃'       # U+1F703
GLYPH_FLOAT = '🜄'     # U+1F704
GLYPH_TEXT = '᛭'       # U+16ED (also accepts regular ")
GLYPH_HANDLE = '⟁'     # U+27C1
GLYPH_CONTRACT_START = '🜊'  # U+1F70A
GLYPH_FIELD = '🜁'     # U+1F701
GLYPH_CONTRACT_END = '🜂'    # U+1F702
GLYPH_ARRAY = '⋔'      # U+22D4
GLYPH_OBJECT = '⋕'     # U+22D5
GLYPH_BIND = '⋯'       # U+22EF
GLYPH_SEP = '⋅'        # U+22C5
GLYPH_COLLAPSE = '⊕'   # U+2295
GLYPH_RESOLVE = '⊖'    # U+2296
GLYPH_ARROW = '→'      # U+2192

ALL_GLYPHS = {
    GLYPH_NULL, GLYPH_TRUE, GLYPH_FALSE, GLYPH_INT, GLYPH_FLOAT,
    GLYPH_TEXT, GLYPH_HANDLE, GLYPH_CONTRACT_START, GLYPH_FIELD,
    GLYPH_CONTRACT_END, GLYPH_ARRAY, GLYPH_OBJECT, GLYPH_BIND,
    GLYPH_SEP, GLYPH_COLLAPSE, GLYPH_RESOLVE, GLYPH_ARROW
}


# ============================================================================
# Simple CAS Store (in-memory for runtime)
# ============================================================================

class SimpleCAS:
    """Simple content-addressed store for runtime use"""
    
    def __init__(self):
        self.store: Dict[str, Any] = {}
    
    def put(self, value: Any) -> str:
        """Store value and return handle"""
        # Serialize value for hashing
        serialized = repr(value).encode('utf-8')
        hash_val = hashlib.sha256(serialized).hexdigest()[:16]
        handle = f"&h_{hash_val}"
        self.store[handle] = value
        return handle
    
    def get(self, handle: str) -> Any:
        """Retrieve value by handle"""
        if handle not in self.store:
            raise HLXError(f"{E_HANDLE_UNRESOLVED}: Handle '{handle}' not found")
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
class Collapse(ASTNode):
    """Latent Space collapse: ⊕(value)"""
    value: ASTNode


@dataclass
class Resolve(ASTNode):
    """Latent Space resolve: ⊖(&handle)"""
    handle: ASTNode


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
    EQ = 'EQ'
    EQEQ = 'EQEQ'
    LT = 'LT'
    GT = 'GT'
    COMMA = 'COMMA'
    EOF = 'EOF'

    def __init__(self, source: str):
        """
        Initialize tokenizer with HLX source

        Args:
            source: HLX source code (with runic glyphs)
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
            
            # Check for glyphs
            if char in ALL_GLYPHS:
                self.tokens.append((self.GLYPH, char))
                self.pos += 1
            
            # Special handling for multi-char glyphs (surrogate pairs)
            elif ord(char) >= 0xD800:
                # Try to read as surrogate pair
                if self.pos + 1 < len(self.source):
                    two_char = self.source[self.pos:self.pos+2]
                    if len(two_char) == 2:
                        try:
                            # Check if it's one of our multi-byte glyphs
                            code_point = ord(two_char[0])
                            if code_point >= 0x1F700:  # Alchemical symbols range
                                self.tokens.append((self.GLYPH, two_char))
                                self.pos += 2
                                continue
                        except:
                            pass
                self.pos += 1
            
            # Parentheses
            elif char == '(':
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
            elif char == ',':
                self.tokens.append((self.COMMA, char))
                self.pos += 1
            
            # String
            elif char == '"':
                self.tokens.append((self.STRING, self._read_string()))
            
            # Number
            elif char.isdigit():
                self.tokens.append((self.NUMBER, self._read_number()))
            
            # Identifier
            elif char.isalpha() or char == '_':
                self.tokens.append((self.IDENT, self._read_identifier()))
            
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
        
        raise HLXError(f"{E_LC_PARSE}: Unterminated string")

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

    def parse(self) -> ASTNode:
        """
        Parse the token stream into an AST

        Returns:
            Root AST node

        Raises:
            HLXError: If parsing fails
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
        
        # Try to parse as binding (x ⋯ value)
        if self._check(HLXTokenizer.IDENT):
            return self._try_parse_binding()
        
        return self._parse_expression()

    def _try_parse_binding(self) -> ASTNode:
        """Try to parse as binding, fall back to expression"""
        # Save position
        saved_pos = self.pos
        
        # Read identifier
        name_token = self._advance()
        name = name_token[1]
        
        # Check for bind operator
        if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_BIND:
            self._advance()  # Consume ⋯
            value = self._parse_expression()
            return Binding(name=name, value=value)
        
        # Not a binding - restore position and parse as expression
        self.pos = saved_pos
        return self._parse_expression()

    def _parse_expression(self) -> ASTNode:
        """Parse an expression"""
        return self._parse_binary()

    def _parse_binary(self) -> ASTNode:
        """Parse binary operations"""
        left = self._parse_unary()
        
        while self._check_any([HLXTokenizer.PLUS, HLXTokenizer.MINUS,
                               HLXTokenizer.STAR, HLXTokenizer.SLASH,
                               HLXTokenizer.EQEQ, HLXTokenizer.LT, HLXTokenizer.GT]):
            op_token = self._advance()
            op = op_token[1]
            right = self._parse_unary()
            left = BinaryOp(op=op, left=left, right=right)
        
        return left

    def _parse_unary(self) -> ASTNode:
        """Parse unary operations and primary expressions"""
        return self._parse_call()

    def _parse_call(self) -> ASTNode:
        """Parse function calls"""
        expr = self._parse_primary()
        
        while self._check(HLXTokenizer.LPAREN):
            self._advance()  # Consume (
            args = []
            
            if not self._check(HLXTokenizer.RPAREN):
                args.append(self._parse_expression())
                while self._check(HLXTokenizer.COMMA):
                    self._advance()
                    args.append(self._parse_expression())
            
            self._expect(HLXTokenizer.RPAREN)
            expr = FunctionCall(func=expr, args=args)
        
        return expr

    def _parse_primary(self) -> ASTNode:
        """Parse primary expressions"""
        if self._is_at_end():
            raise HLXError(f"{E_LC_PARSE}: Unexpected end of input")
        
        token = self._peek()
        token_type, token_value = token
        
        # Glyphs
        if token_type == HLXTokenizer.GLYPH:
            return self._parse_glyph()
        
        # Number
        if token_type == HLXTokenizer.NUMBER:
            self._advance()
            if '.' in token_value:
                return Literal(value=float(token_value))
            return Literal(value=int(token_value))
        
        # String
        if token_type == HLXTokenizer.STRING:
            self._advance()
            return Literal(value=token_value)
        
        # Identifier
        if token_type == HLXTokenizer.IDENT:
            self._advance()
            return Variable(name=token_value)
        
        # Parenthesized expression
        if token_type == HLXTokenizer.LPAREN:
            self._advance()
            expr = self._parse_expression()
            self._expect(HLXTokenizer.RPAREN)
            return expr
        
        # Array
        if token_type == HLXTokenizer.LBRACKET:
            return self._parse_array_literal()
        
        raise HLXError(f"{E_LC_PARSE}: Unexpected token {token_type}: {token_value}")

    def _parse_glyph(self) -> ASTNode:
        """Parse a glyph token"""
        token = self._advance()
        glyph = token[1]
        
        # Null
        if glyph == GLYPH_NULL:
            return Literal(value=None)
        
        # True
        if glyph == GLYPH_TRUE:
            return Literal(value=True)
        
        # False
        if glyph == GLYPH_FALSE:
            return Literal(value=False)
        
        # Integer
        if glyph == GLYPH_INT:
            num_token = self._expect(HLXTokenizer.NUMBER)
            return Literal(value=int(num_token[1]))
        
        # Float
        if glyph == GLYPH_FLOAT:
            num_token = self._expect(HLXTokenizer.NUMBER)
            return Literal(value=float(num_token[1]))
        
        # Text
        if glyph == GLYPH_TEXT:
            str_token = self._expect(HLXTokenizer.STRING)
            return Literal(value=str_token[1])
        
        # Handle reference
        if glyph == GLYPH_HANDLE:
            name_token = self._expect(HLXTokenizer.IDENT)
            return Literal(value="&h_" + name_token[1])
        
        # Collapse
        if glyph == GLYPH_COLLAPSE:
            self._expect(HLXTokenizer.LPAREN)
            value = self._parse_expression()
            self._expect(HLXTokenizer.RPAREN)
            return Collapse(value=value)
        
        # Resolve
        if glyph == GLYPH_RESOLVE:
            self._expect(HLXTokenizer.LPAREN)
            handle = self._parse_expression()
            self._expect(HLXTokenizer.RPAREN)
            return Resolve(handle=handle)
        
        # Contract
        if glyph == GLYPH_CONTRACT_START:
            return self._parse_contract()
        
        # Array
        if glyph == GLYPH_ARRAY:
            return self._parse_array_literal()
        
        # Object
        if glyph == GLYPH_OBJECT:
            return self._parse_object_literal()
        
        raise HLXError(f"{E_LC_PARSE}: Unknown glyph: {glyph}")

    def _parse_contract(self) -> Contract:
        """Parse a contract: 🜊id🜁field value🜂"""
        # Contract ID
        id_token = self._expect(HLXTokenizer.NUMBER)
        contract_id = int(id_token[1])
        
        fields: Dict[int, ASTNode] = {}
        
        # Parse fields until contract end
        while not self._is_at_end():
            if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_CONTRACT_END:
                self._advance()
                break
            
            if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_FIELD:
                self._advance()  # Consume field marker
                field_idx_token = self._expect(HLXTokenizer.NUMBER)
                field_idx = int(field_idx_token[1])
                field_value = self._parse_expression()
                fields[field_idx] = field_value
            else:
                # Just parse value directly
                value = self._parse_expression()
                # Auto-assign to next field index
                next_idx = max(fields.keys()) + 1 if fields else 0
                fields[next_idx] = value
        
        return Contract(contract_id=contract_id, fields=fields)

    def _parse_array_literal(self) -> Array:
        """Parse an array: [elem, elem] or ⋔[elem⋅elem]"""
        if self._check(HLXTokenizer.LBRACKET):
            self._advance()  # Consume [
        
        elements = []
        
        while not self._check(HLXTokenizer.RBRACKET) and not self._is_at_end():
            elements.append(self._parse_expression())
            
            # Check for separator
            if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_SEP:
                self._advance()
            elif self._check(HLXTokenizer.COMMA):
                self._advance()
            elif not self._check(HLXTokenizer.RBRACKET):
                break
        
        if self._check(HLXTokenizer.RBRACKET):
            self._advance()
        
        return Array(elements=elements)

    def _parse_object_literal(self) -> Object:
        """Parse an object: ⋕{x⋯10⋅y⋯20}"""
        if self._check(HLXTokenizer.LBRACE):
            self._advance()  # Consume {
        
        fields: Dict[str, ASTNode] = {}
        
        while not self._check(HLXTokenizer.RBRACE) and not self._is_at_end():
            # Key
            key_token = self._expect(HLXTokenizer.IDENT)
            key = key_token[1]
            
            # Expect bind operator
            if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_BIND:
                self._advance()
            
            # Value
            value = self._parse_expression()
            fields[key] = value
            
            # Separator
            if self._check(HLXTokenizer.GLYPH) and self._peek()[1] == GLYPH_SEP:
                self._advance()
            elif self._check(HLXTokenizer.COMMA):
                self._advance()
        
        if self._check(HLXTokenizer.RBRACE):
            self._advance()
        
        return Object(fields=fields)

    def _expect(self, token_type: str) -> Tuple[str, str]:
        """Expect a specific token type, consume and return it"""
        if self._is_at_end():
            raise HLXError(f"{E_LC_PARSE}: Unexpected EOF, expected {token_type}")

        token = self.tokens[self.pos]
        if token[0] != token_type:
            raise HLXError(f"{E_LC_PARSE}: Expected {token_type}, got {token[0]}: {token[1]}")

        self.pos += 1
        return token

    def _peek(self) -> Tuple[str, str]:
        """Peek at current token without consuming"""
        if self.pos < len(self.tokens):
            return self.tokens[self.pos]
        return (HLXTokenizer.EOF, '')

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
        return self.pos >= len(self.tokens) or self.tokens[self.pos][0] == HLXTokenizer.EOF


# ============================================================================
# Evaluator
# ============================================================================

class HLXEvaluator:
    """Evaluate HLX AST with environment and CAS integration"""

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

    def eval(self, node: ASTNode) -> Any:
        """
        Evaluate an AST node

        Args:
            node: AST node to evaluate

        Returns:
            Evaluated value

        Raises:
            HLXError: If evaluation fails
        """
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

        elif isinstance(node, FunctionDef):
            # Store function in environment
            return node

        elif isinstance(node, FunctionCall):
            func = self.eval(node.func)
            args = [self.eval(arg) for arg in node.args]

            if callable(func):
                # Built-in function
                return func(*args)
            elif isinstance(func, FunctionDef):
                # User-defined function
                return self._call_user_function(func, args)
            else:
                raise HLXError(f"{E_INVALID_INPUT}: {func} is not callable")

        elif isinstance(node, Contract):
            # Evaluate contract fields
            fields = {idx: self.eval(val) for idx, val in node.fields.items()}
            return {
                'contract_id': node.contract_id,
                **{f'field_{idx}': val for idx, val in fields.items()}
            }

        elif isinstance(node, Array):
            return [self.eval(elem) for elem in node.elements]

        elif isinstance(node, Object):
            return {key: self.eval(val) for key, val in node.fields.items()}

        elif isinstance(node, Collapse):
            value = self.eval(node.value)
            return collapse(value, self.cas)

        elif isinstance(node, Resolve):
            handle = self.eval(node.handle)
            if not isinstance(handle, str) or not handle.startswith('&'):
                raise HLXError(f"{E_INVALID_INPUT}: Resolve requires handle (got {handle})")
            return resolve(handle, self.cas)

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
        """Call a user-defined function with arguments"""
        if len(args) != len(func.params):
            raise HLXError(f"{E_INVALID_INPUT}: Function expects {len(func.params)} args, got {len(args)}")

        # Create new environment scope
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

    def _builtin_collapse(self, value: Any) -> str:
        """Built-in: collapse(value) → handle"""
        return collapse(value, self.cas)

    def _builtin_resolve(self, handle: str) -> Any:
        """Built-in: resolve(handle) → value"""
        return resolve(handle, self.cas)

    def _builtin_snapshot(self) -> Any:
        """Built-in: snapshot() → checkpoint"""
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

class HLXRuntime:
    """HLX-LS Runtime - Execute Runic programs with Latent Space operations"""

    def __init__(self, cas: Optional[SimpleCAS] = None):
        """
        Initialize HLX runtime

        Args:
            cas: Content-addressed store (default: global CAS)
        """
        self.cas = cas or get_cas_store()
        self.evaluator = HLXEvaluator(self.cas)

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
        """
        Execute HLX source file

        Args:
            filepath: Path to .hlx file

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

def execute_hlx(source: str, cas: Optional[SimpleCAS] = None) -> Any:
    """
    Execute HLX source code

    Args:
        source: HLX source code (with runic glyphs)
        cas: Content-addressed store (default: global CAS)

    Returns:
        Result of evaluation

    Example:
        >>> execute_hlx('🜃42')
        42
        >>> execute_hlx('⊕(🜃42)')  # Collapse 42 to CAS
        '&h_...'
    """
    runtime = HLXRuntime(cas)
    return runtime.execute(source)


# ============================================================================
# Main (Test)
# ============================================================================

if __name__ == '__main__':
    print("HLX-LS Runtime Test Suite\n")

    runtime = HLXRuntime()

    # Test literals
    print("=== Literals ===")
    print(f"Null (∅): {runtime.execute('∅')}")
    print(f"True (⊤): {runtime.execute('⊤')}")
    print(f"False (⊥): {runtime.execute('⊥')}")
    print(f"Int (42): {runtime.execute('42')}")
    print(f"Float (3.14): {runtime.execute('3.14')}")
    print(f"String: {runtime.execute('"hello"')}")

    # Test bindings
    print("\n=== Bindings ===")
    runtime.execute('x ⋯ 10')
    print(f"x = {runtime.get_var('x')}")  # 10

    # Test arithmetic
    print("\n=== Arithmetic ===")
    runtime.execute('a ⋯ 5')
    runtime.execute('b ⋯ 3')
    result = runtime.execute('a + b')
    print(f"a + b = {result}")

    # Test collapse/resolve
    print("\n=== Latent Space ===")
    handle = runtime.execute('⊕(42)')
    print(f"Collapsed 42: {handle}")
    runtime.set_var('h', handle)
    value = runtime.execute('⊖(h)')
    print(f"Resolved: {value}")

    print("\n✨ HLX-LS runtime ready!")
