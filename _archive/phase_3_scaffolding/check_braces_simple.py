
with open("hlx_compiler/bootstrap/compiler.hlxc", "r") as f:
    lines = f.readlines()

balance = 0
for i, line in enumerate(lines):
    prev_balance = balance
    for char in line:
        if char == '{':
            balance += 1
        elif char == '}':
            balance -= 1
    
    # Print if balance changes
    # if balance != prev_balance:
    #     print(f"{i+1}: {balance} | {line.strip()}")
    
    if balance < 0:
        print(f"Error: Negative balance at line {i+1}: {line.strip()}")
        # break

print(f"Final balance: {balance}")
