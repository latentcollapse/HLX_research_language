import sys

def search_in_file(filename, targets):
    try:
        with open(filename, "rb") as f:
            data = f.read()
            
        print(f"Scanning {filename} ({len(data)} bytes)...")
        found_any = False
        for t in targets:
            # Search for the raw utf-8 bytes
            b_target = t.encode('utf-8')
            if b_target in data:
                print(f"[FOUND] '{t}' is present in the binary.")
                found_any = True
                # Optional: Context print
                idx = data.find(b_target)
                start = max(0, idx - 10)
                end = min(len(data), idx + len(b_target) + 10)
                print(f"    Context: {data[start:end]}")
            else:
                print(f"[MISSING] '{t}' NOT found.")
        
        if not found_any:
            print("\nNone of the target strings were found.")

    except FileNotFoundError:
        print(f"Error: File {filename} not found.")

if __name__ == "__main__":
    targets = ["ctx", "expr", "state", "rs", "compile_expr"]
    search_in_file("compiler.lcc", targets)

