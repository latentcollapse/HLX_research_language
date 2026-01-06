
with open('part1.hlxc', 'r') as f:
    lines = f.readlines()

# To get " in the file, we need \" in a Python string.
# To get \ in the file, we need \\ in a Python string.

lines[60] = '                    if c == "\"" {\n'
lines[63] = '                        loop (i < len and source[i] != "\"", 1000000) {\n'
lines[64] = '                            if source[i] == "\\" and i + 1 < len and source[i+1] == "\"" {\n'
lines[65] = '                                s = s + "\"";\n'

with open('part1.hlxc', 'w') as f:
    f.writelines(lines)

print("Fixed part1.hlxc with triple backslashes")

