"""
HLX Merkle Tables
Implements Merkle tree structure for state integrity.
"""

from typing import Any, Dict, List, Optional
from .lc_codec import encode_lcb, compute_hash


class MerkleNode:
    def __init__(self, hash_value: str, children: List['MerkleNode'] = None, data: Any = None):
        self.hash = hash_value
        self.children = children or []
        self.data = data

    def is_leaf(self) -> bool:
        return len(self.children) == 0

    def to_dict(self) -> Dict:
        result = {'hash': self.hash}
        if self.children:
            result['children'] = [c.to_dict() for c in self.children]
        return result


class MerkleTree:
    FANOUT = 16

    def __init__(self):
        self.root: Optional[MerkleNode] = None
        self.leaves: List[MerkleNode] = []

    def _hash_combine(self, hashes: List[str]) -> str:
        combined = ''.join(sorted(hashes))
        return compute_hash(combined.encode('utf-8'))

    def add_leaf(self, value: Any) -> str:
        lcb_bytes = encode_lcb(value)
        leaf_hash = compute_hash(lcb_bytes)
        leaf = MerkleNode(leaf_hash, data=value)
        self.leaves.append(leaf)
        return leaf_hash

    def build(self):
        if not self.leaves:
            self.root = None
            return

        current_level = self.leaves.copy()

        while len(current_level) > 1:
            next_level = []
            for i in range(0, len(current_level), self.FANOUT):
                children = current_level[i:i + self.FANOUT]
                child_hashes = [c.hash for c in children]
                while len(child_hashes) < self.FANOUT:
                    child_hashes.append('0' * 64)
                parent_hash = self._hash_combine(child_hashes)
                parent = MerkleNode(parent_hash, children=children)
                next_level.append(parent)
            current_level = next_level

        self.root = current_level[0] if current_level else None

    def get_root_hash(self) -> Optional[str]:
        return self.root.hash if self.root else None

    def to_dict(self) -> Dict:
        return {
            'root': self.root.to_dict() if self.root else None,
            'leaf_count': len(self.leaves),
            'fanout': self.FANOUT
        }


class StateTable:
    def __init__(self):
        self.entries: Dict[str, tuple] = {}
        self.merkle = MerkleTree()
        self._dirty = False

    def set(self, handle: str, value: Any) -> str:
        lcb_bytes = encode_lcb(value)
        value_hash = compute_hash(lcb_bytes)
        self.entries[handle] = (value, value_hash)
        self._dirty = True
        return value_hash

    def get(self, handle: str) -> Optional[Any]:
        entry = self.entries.get(handle)
        return entry[0] if entry else None

    def get_hash(self, handle: str) -> Optional[str]:
        entry = self.entries.get(handle)
        return entry[1] if entry else None

    def delete(self, handle: str) -> bool:
        if handle in self.entries:
            del self.entries[handle]
            self._dirty = True
            return True
        return False

    def rebuild_merkle(self):
        self.merkle = MerkleTree()
        for handle in sorted(self.entries.keys()):
            value, _ = self.entries[handle]
            self.merkle.add_leaf(value)
        self.merkle.build()
        self._dirty = False

    def get_state_hash(self) -> Optional[str]:
        if self._dirty:
            self.rebuild_merkle()
        return self.merkle.get_root_hash()

    def verify_integrity(self) -> bool:
        for handle, (value, stored_hash) in self.entries.items():
            lcb_bytes = encode_lcb(value)
            if compute_hash(lcb_bytes) != stored_hash:
                return False
        return True

    def snapshot(self) -> Dict:
        if self._dirty:
            self.rebuild_merkle()
        return {
            'entries': {h: {'value': v, 'hash': hsh} for h, (v, hsh) in self.entries.items()},
            'merkle_root': self.get_state_hash(),
            'entry_count': len(self.entries)
        }

    def restore(self, snapshot: Dict) -> bool:
        try:
            self.entries = {}
            for handle, entry in snapshot.get('entries', {}).items():
                self.entries[handle] = (entry['value'], entry['hash'])
            self._dirty = True
            return True
        except (KeyError, TypeError):
            return False
