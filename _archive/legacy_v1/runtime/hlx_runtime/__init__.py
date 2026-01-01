"""
HLX Runtime - Complete Latent Space System

This package provides the complete HLX ecosystem:

**Language Runtimes:**
- HLX: Basic Runic language runtime (no LS operations)
- HLX-LS: Runic language runtime with Latent Space operations
- HLXL: Basic ASCII language runtime (no LS operations)
- HLXL-LS: ASCII language runtime with Latent Space operations

**Wire Formats:**
- LC-B: Binary wire format (compact, deterministic)
- LC-R: Runic wire format (Unicode glyphs)
- LC-T: Text wire format (ASCII-safe)

**Core Infrastructure:**
- CAS: Content-Addressed Storage
- Contracts: Contract validation and wrapping
- LS Operations: Collapse, resolve, snapshot

Version: 2.0.0 - Complete Runtime Package
"""

__version__ = '2.0.0'

# ============================================================================
# Wire Format Codecs
# ============================================================================

# LC-B: Binary wire format
from .lc_codec import (
    encode_lcb, decode_lcb, encode_lct,
    compute_hash, canonical_hash, verify_bijection,
    wrap_contract, unwrap_contract,
    LCCodecError, LCEncodeError, LCDecodeError,
)

# LC-R: Runic wire format
from .lc_r_codec import (
    encode_lcr, decode_lcr, compression_ratio,
    LCREncoder, LCRDecoder,
)

# LC-T: Text wire format (ASCII-safe)
from .lc_t_codec import (
    encode_lct as encode_lct_new, decode_lct, verify_lct_bijection,
    LCTEncoder, LCTDecoder,
)

# ============================================================================
# Language Runtimes
# ============================================================================

# Basic HLX Runtime (Runic without LS operations)
from .hlx_runtime import (
    HLXRuntime as HLXBasicRuntime,
    HLXTokenizer as HLXBasicTokenizer,
    HLXParser as HLXBasicParser,
    HLXEvaluator as HLXBasicEvaluator,
)

# HLX-LS Runtime (Runic with LS operations)
from .hlx_ls_runtime import (
    HLXRuntime, HLXTokenizer, HLXParser, HLXEvaluator,
    ASTNode, Literal, Variable, Binding, FunctionDef, FunctionCall,
    Contract, Array, Object, Collapse, Resolve, BinaryOp, Block,
    SimpleCAS, get_cas_store as get_cas_store_hlx, collapse, resolve, snapshot,
)

# Basic HLXL Runtime (ASCII without LS operations)
from .hlxl_runtime import (
    HLXLRuntime as HLXLBasicRuntime,
    HLXLTokenizer as HLXLBasicTokenizer,
    HLXLParser as HLXLBasicParser,
    HLXLEvaluator as HLXLBasicEvaluator,
)

# HLXL-LS Runtime (ASCII with LS operations)
from .hlxl_ls_runtime import (
    HLXLRuntime, HLXLTokenizer, HLXLParser, HLXLEvaluator,
    MethodCall,
)

# ============================================================================
# Core Infrastructure
# ============================================================================

# Glyphs and symbols
from .glyphs import (
    LC_R_GLYPHS, GLYPH_TO_NAME, ALL_GLYPHS, ALL_GLYPH_TO_NAME,
    CELTIC_GLYPHS, ELDER_FUTHARK, ALCHEMICAL_GLYPHS, MATH_OPERATORS,
    is_lc_r_glyph, get_glyph_name, format_lc_r,
)

# Errors
from .errors import (
    E_LC_PARSE, E_CONTRACT_STRUCTURE, E_HANDLE_UNRESOLVED,
    E_ENV_PAYLOAD_HASH_MISMATCH, E_VALIDATION_FAIL, E_INVALID_INPUT,
    E_CAS_READ_FAIL, E_CAS_WRITE_FAIL,
    E_LC_ENCODE, E_LC_DECODE,
    HLXError,
)

# Contracts
from .contracts import (
    CONTRACT_IDS, is_contract_wrapped,
    wrap_literal, unwrap_literal, validate_contract,
)

# Content-Addressed Storage
from .cas import CASStore, get_cas_store

# Data structures
from .tables import MerkleTree, StateTable

# Latent Space operations
from .ls_ops import (
    LSContext, ls_collapse, ls_resolve,
    ls_encode, ls_decode, ls_hash,
    ls_validate, ls_wrap, ls_unwrap,
)

# ============================================================================
# Exports
# ============================================================================

__all__ = [
    # Version
    '__version__',

    # Wire Format Codecs
    'encode_lcb', 'decode_lcb', 'encode_lct',  # LC-B
    'encode_lcr', 'decode_lcr', 'compression_ratio', 'LCREncoder', 'LCRDecoder',  # LC-R
    'encode_lct_new', 'decode_lct', 'verify_lct_bijection', 'LCTEncoder', 'LCTDecoder',  # LC-T
    'compute_hash', 'canonical_hash', 'verify_bijection',
    'wrap_contract', 'unwrap_contract',
    'LCCodecError', 'LCEncodeError', 'LCDecodeError',

    # Basic HLX Runtime (no LS)
    'HLXBasicRuntime', 'HLXBasicTokenizer', 'HLXBasicParser', 'HLXBasicEvaluator',

    # HLX-LS Runtime
    'HLXRuntime', 'HLXTokenizer', 'HLXParser', 'HLXEvaluator',
    'ASTNode', 'Literal', 'Variable', 'Binding', 'FunctionDef', 'FunctionCall',
    'Contract', 'Array', 'Object', 'Collapse', 'Resolve', 'BinaryOp', 'Block',
    'SimpleCAS', 'get_cas_store_hlx', 'collapse', 'resolve', 'snapshot',

    # Basic HLXL Runtime (no LS)
    'HLXLBasicRuntime', 'HLXLBasicTokenizer', 'HLXLBasicParser', 'HLXLBasicEvaluator',

    # HLXL-LS Runtime
    'HLXLRuntime', 'HLXLTokenizer', 'HLXLParser', 'HLXLEvaluator',
    'MethodCall',

    # Glyphs
    'LC_R_GLYPHS', 'GLYPH_TO_NAME', 'ALL_GLYPHS', 'ALL_GLYPH_TO_NAME',
    'CELTIC_GLYPHS', 'ELDER_FUTHARK', 'ALCHEMICAL_GLYPHS', 'MATH_OPERATORS',
    'is_lc_r_glyph', 'get_glyph_name', 'format_lc_r',

    # Errors
    'HLXError',
    'E_LC_PARSE', 'E_LC_ENCODE', 'E_LC_DECODE',
    'E_CONTRACT_STRUCTURE', 'E_HANDLE_UNRESOLVED', 'E_INVALID_INPUT',
    'E_ENV_PAYLOAD_HASH_MISMATCH', 'E_VALIDATION_FAIL',
    'E_CAS_READ_FAIL', 'E_CAS_WRITE_FAIL',

    # Contracts
    'CONTRACT_IDS', 'is_contract_wrapped',
    'wrap_literal', 'unwrap_literal', 'validate_contract',

    # CAS
    'CASStore', 'get_cas_store',

    # Data structures
    'MerkleTree', 'StateTable',

    # LS Operations
    'LSContext', 'ls_collapse', 'ls_resolve',
    'ls_encode', 'ls_decode', 'ls_hash',
    'ls_validate', 'ls_wrap', 'ls_unwrap',
]
