import sys
import argparse
import os
from .lc_codec import LCTParser, decode_lcb, encode_runic
from .ls_ops import collapse, resolve
from .errors import HLXError

def main():
    parser = argparse.ArgumentParser(description="HLX Runtime CLI")
    parser.add_argument('command', choices=['run', 'collapse', 'resolve'])
    parser.add_argument('file', help="Input file")
    parser.add_argument('--format', choices=['lct', 'lcb'], default='lct', help="Input format")
    
    args = parser.parse_args()
    
    try:
        with open(args.file, 'rb') as f:
            data = f.read()

        if args.command == 'collapse':
            if args.format == 'lct':
                # Decode text
                text = data.decode('utf-8')
                val = LCTParser().parse_text(text)
            else:
                val = decode_lcb(data)
            
            handle = collapse(val)
            print(f"Collapsed: {handle}")
            print(f"Runic: {encode_runic(val)}")

        elif args.command == 'resolve':
            # Resolve a handle (file contains handle string?)
            handle = data.decode('utf-8').strip()
            val = resolve(handle)
            print(f"Resolved: {val}")
            print(f"Runic: {encode_runic(val)}")
            
    except Exception as e:
        print(f"Error: {e}")
        sys.exit(1)

if __name__ == '__main__':
    main()
