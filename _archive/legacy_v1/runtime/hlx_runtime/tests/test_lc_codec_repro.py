
import unittest
import math
import sys
import os

# Add parent directory to path to import hlx_runtime
sys.path.append(os.path.abspath(os.path.join(os.path.dirname(__file__), '../..')))

from hlx_runtime.lc_codec import encode_lcb, decode_lcb, encode_runic, LCEncodeError, LCDecodeError, LCBParser, LCTParser
from hlx_runtime.errors import E_FLOAT_SPECIAL

class TestLCB(unittest.TestCase):
    def test_primitives(self):
        self.assertEqual(decode_lcb(encode_lcb(None)), None)
        self.assertEqual(decode_lcb(encode_lcb(True)), True)
        self.assertEqual(decode_lcb(encode_lcb(False)), False)
        self.assertEqual(decode_lcb(encode_lcb(123)), 123)
        self.assertEqual(decode_lcb(encode_lcb(-123)), -123)
        self.assertEqual(decode_lcb(encode_lcb(3.14)), 3.14)
        self.assertEqual(decode_lcb(encode_lcb("hello")), "hello")
        self.assertEqual(decode_lcb(encode_lcb(b"bytes")), b"bytes")

    def test_compound(self):
        arr = [1, "two", 3.0]
        self.assertEqual(decode_lcb(encode_lcb(arr)), arr)
        obj = {"a": 1, "b": 2}
        self.assertEqual(decode_lcb(encode_lcb(obj)), obj)

    def test_float_special(self):
        with self.assertRaises(LCEncodeError) as cm:
            encode_lcb(float('nan'))
        self.assertIn(E_FLOAT_SPECIAL, str(cm.exception))

        with self.assertRaises(LCEncodeError) as cm:
            encode_lcb(float('inf'))
        self.assertIn(E_FLOAT_SPECIAL, str(cm.exception))

        with self.assertRaises(LCEncodeError) as cm:
            encode_lcb(float('-inf'))
        self.assertIn(E_FLOAT_SPECIAL, str(cm.exception))

    def test_recursion_depth(self):
        deep = []
        for _ in range(70):
            deep = [deep]
        
        with self.assertRaises(LCEncodeError) as cm:
            encode_lcb(deep)
    
    def test_parser_class(self):
        parser = LCBParser()
        data = parser.encode(123)
        self.assertEqual(parser.parse(data), 123)

    def test_lct_parser(self):
        parser = LCTParser()
        # Test primitives - note that to_text wraps in []
        self.assertEqual(parser.parse_text('[INT(123)]'), 123)
        self.assertEqual(parser.parse_text('[STRING("hello")]'), "hello")
        
        # Test object
        obj_text = '[OBJ_START, FIELD_0, INT(123), OBJ_END]'
        self.assertEqual(parser.parse_text(obj_text), {"0": 123})
        
        # Test roundtrip
        obj = {"0": 123, "key": "val"}
        text = parser.to_text(obj)
        # Should look something like: [OBJ_START, FIELD_0, INT(123), KEY("key"), STRING("val"), OBJ_END]
        self.assertEqual(parser.parse_text(text), obj)

if __name__ == '__main__':
    unittest.main()
