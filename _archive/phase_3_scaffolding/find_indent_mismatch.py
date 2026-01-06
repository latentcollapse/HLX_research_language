
with open("hlx_compiler/bootstrap/compiler.hlxc", "r") as f:
    lines = f.readlines()

expected_indent = 0
for i, line in enumerate(lines):
    s = line.strip()
    if not s: continue
    
    current_indent = len(line) - len(line.lstrip())
    
    # Adjust expected indent for closing brace
    if s.startswith("}"):
        expected_indent -= 4
    
    if current_indent != expected_indent:
        print(f"Line {i+1}: Indent mismatch. Expected {expected_indent}, Got {current_indent}. Content: {s}")
    
    # Adjust expected indent for opening brace
    if s.endswith("{"):
        expected_indent += 4
    
    # Special handling for "else {" lines which have both
    # if s == "} else {": expected_indent += 4 (already subtracted 4, so net 0 change? No. } closes, { opens. Net 0 change. Correct.)
