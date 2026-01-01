
import unittest
import sys
import os

sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from hlx_runtime.lc_codec import LCTParser
from hlx_runtime.ls_ops import collapse, resolve, transaction
from hlx_runtime.cas import get_cas_store

class TestEndToEnd(unittest.TestCase):
    def test_flow(self):
        # 1. Parse text
        text = '[OBJ_START, FIELD_0, INT(123), OBJ_END]'
        val = LCTParser().parse_text(text)
        
        # 2. Collapse
        handle = collapse(val)
        self.assertTrue(handle.startswith('&h_'))
        
        # 3. Resolve
        val2 = resolve(handle)
        self.assertEqual(val, val2)
        
    def test_transaction(self):
        # 4. Transaction
        store_before = get_cas_store().snapshot()
        
        def txn():
             collapse("new_data_transaction")
             raise ValueError("Abort")
             
        try:
            transaction(txn)
        except ValueError:
            pass
            
        store_after = get_cas_store().snapshot()
        
        # Verify rollback
        # We need to ensure "new_data_transaction" is NOT in store_after
        # Check keys.
        self.assertEqual(store_before.keys(), store_after.keys())

if __name__ == '__main__':
    unittest.main()
