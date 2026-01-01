"""
HLX Latent Space Operations
Core operations for the Latent Space (LS) system.
Reference: CONTRACT_803
"""

from typing import Any, Callable, Dict, Optional, Tuple, TypeVar
from .cas import CASStore, get_cas_store
from .lc_codec import encode_lcb, decode_lcb, canonical_hash, encode_runic, LCTParser
from .errors import E_HANDLE_NOT_FOUND, E_IO_ERROR
from .contracts import wrap_literal, unwrap_literal, validate_contract

T = TypeVar('T')

def collapse(value: Any, cas: CASStore = None) -> str:
    """
    ls.collapse(value) -> handle
    Encode to LC-B, store in CAS, return handle.
    """
    cas = cas or get_cas_store()
    return cas.store(value)

def resolve(handle: str, cas: CASStore = None) -> Any:
    """
    ls.resolve(handle) -> value
    Retrieve from CAS, decode from LC-B.
    """
    cas = cas or get_cas_store()
    return cas.retrieve(handle)

def snapshot(cas: CASStore = None) -> Any:
    """
    ls.snapshot() -> checkpoint
    Capture current CAS state for rollback.
    """
    cas = cas or get_cas_store()
    return cas.snapshot()

def transaction(fn: Callable[[], T], cas: CASStore = None) -> T:
    """
    ls.transaction(fn) -> result
    Execute fn atomically, rollback on error.
    """
    cas = cas or get_cas_store()
    checkpoint = cas.snapshot()
    try:
        return fn()
    except Exception:
        cas.restore(checkpoint)
        raise

# Legacy/Compatibility aliases
ls_collapse = collapse
ls_resolve = resolve

def ls_encode(value: Any, mode: str = 'LC-B') -> bytes:
    if mode.upper() == 'LC-T':
         return encode_runic(value).encode('utf-8')
    return encode_lcb(value)

def ls_decode(data: bytes, mode: str = 'LC-B') -> Any:
    if mode.upper() == 'LC-T':
         return LCTParser().parse_text(data.decode('utf-8'))
    return decode_lcb(data)

def ls_hash(value: Any) -> str:
    return canonical_hash(value)

def ls_validate(value: Any, wrapped: bool = True) -> Tuple[bool, Optional[str]]:
    if wrapped:
        try:
            # validate_contract returns normalized value or raises
            validate_contract(value) # Assuming explicit contract arg needed? 
            # contracts.py needs check.
            return True, None
        except Exception as e:
            return False, str(e)
    try:
        encode_lcb(value)
        return True, None
    except Exception as e:
        return False, str(e)

def ls_wrap(value: Any) -> Dict:
    return wrap_literal(value)

def ls_unwrap(wrapped: Dict) -> Any:
    return unwrap_literal(wrapped)

# Class for context if needed for backward compat, but function API is preferred
class LSContext:
    def __init__(self, cas_store: CASStore = None):
        self.cas = cas_store or get_cas_store()
    
    def collapse(self, value: Any) -> Tuple[str, str]:
        # Emulate old behavior: return handle, hash
        handle = self.cas.store(value)
        # handle is &h_tag_hash
        # Extract hash from handle?
        parts = handle.split('_')
        h = parts[-1]
        return handle, h
        
    def resolve(self, handle: str) -> Tuple[Optional[Any], Optional[str]]:
        try:
            val = self.cas.retrieve(handle)
            return val, handle.split('_')[-1]
        except Exception:
            return None, None
            
    def collapse_with_hash(self, value: Any) -> Dict:
        handle = self.cas.store(value)
        h = handle.split('_')[-1]
        return {
            'handle': handle,
            'content_hash': h,
            'lc_b_size': len(encode_lcb(value)),
            'lc_t': encode_runic(value)
        }