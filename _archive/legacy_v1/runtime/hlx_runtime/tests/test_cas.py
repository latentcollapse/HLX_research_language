
import unittest
import sys
import os

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from hlx_runtime.cas import CASStore, get_cas_store
from hlx_runtime.errors import HandleNotFoundError

class TestCAS(unittest.TestCase):
    def test_store_retrieve(self):
        cas = CASStore()
        val = 123
        handle = cas.store(val)
        # Handle format: &h_<tag>_<hash>
        self.assertTrue(handle.startswith("&h_int_"))
        self.assertEqual(cas.retrieve(handle), val)
        self.assertTrue(cas.exists(handle))

    def test_idempotence(self):
        cas = CASStore()
        h1 = cas.store("hello")
        h2 = cas.store("hello")
        self.assertEqual(h1, h2)
        
        # Test diff value diff handle
        h3 = cas.store("world")
        self.assertNotEqual(h1, h3)

    def test_missing(self):
        cas = CASStore()
        with self.assertRaises(HandleNotFoundError):
            cas.retrieve("&h_missing")

    def test_global_cas(self):
        cas = get_cas_store()
        h = cas.store("global")
        self.assertTrue(cas.exists(h))

if __name__ == '__main__':
    unittest.main()
