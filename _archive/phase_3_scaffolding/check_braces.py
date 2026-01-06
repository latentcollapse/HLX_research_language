
with open("hlx_compiler/bootstrap/compiler.hlxc", "r") as f:
    content = f.read()

balance = 0
for i, char in enumerate(content):
    if char == '{':
        balance += 1
    elif char == '}':
        balance -= 1
    
    if balance < 0:
        print(f"Error: Negative balance at index {i}")
        break

print(f"Final balance: {balance}")
