
def check_braces(file_path):
    with open(file_path, 'r') as f:
        content = f.read()
    
    balance = 0
    in_string = False
    in_comment = False
    escaped = False
    
    for i, char in enumerate(content):
        if escaped:
            escaped = False
            continue
        
        if in_string:
            if char == '\\':
                escaped = True
            elif char == '"':
                in_string = False
            continue
        
        if in_comment:
            if char == '\n':
                in_comment = False
            continue
        
        if char == '"':
            in_string = True
        elif char == '/' and i + 1 < len(content) and content[i+1] == '/':
            in_comment = True
        elif char == '{':
            balance += 1
        elif char == '}':
            balance -= 1
            if balance < 0:
                # Find line number
                line_no = content[:i].count('\n') + 1
                print(f"Error: Negative balance at line {line_no}")
                return balance
                
    return balance

bal = check_braces('hlx_compiler/bootstrap/compiler.hlxc')
print(f"Final balance: {bal}")
