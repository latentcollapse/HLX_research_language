"""
Content-Addressed Store (CAS) implementation.
Reference: CONTRACT_802
"""

from typing import Any, Optional, Dict
from .lc_codec import encode_lcb, decode_lcb, get_type_tag, compute_hash
from .errors import HandleNotFoundError

class CASStore:
    """
    CONTRACT_802: Content-Addressed Store (CAS)
    """
    def __init__(self):
        self._store: Dict[str, bytes] = {}

    def store(self, value: Any) -> str:
        # 1. Encode to LC-B (canonical)
        encoded = encode_lcb(value)
        
        # 2. Compute Hash
        h = compute_hash(encoded)
        
        # 3. Generate Handle
        tag = get_type_tag(value)
        handle = f"&h_{tag}_{h}"
        
        # 4. Store
        self._store[handle] = encoded
        return handle

    def retrieve(self, handle: str) -> Any:
        if handle not in self._store:
            raise HandleNotFoundError(f"Handle not found: {handle}")
        return decode_lcb(self._store[handle])

    def exists(self, handle: str) -> bool:
        return handle in self._store

    def snapshot(self) -> Dict[str, bytes]:
        return self._store.copy()

    def restore(self, snapshot: Dict[str, bytes]):
        self._store = snapshot.copy()

_global_cas = CASStore()

def get_cas_store() -> CASStore:
    return _global_cas