
quote = b'"'
backslash = b'\\'
lit_quote = quote + backslash + quote + quote
lit_backslash = quote + backslash + backslash + quote
newline = b'\n'

with open('part1.hlxc', 'rb') as f:
    lines = f.readlines()

new_lines = []
for line in lines:
    if b'if source[i] == ' in line:
        parts = [
            b'                            if source[i] == ',
            lit_backslash,
            b' and i + 1 < len and source[i+1] == ',
            lit_quote,
            b' {',
            newline
        ]
        new_lines.append(b''.join(parts))
    else:
        new_lines.append(line)

with open('part1.hlxc', 'wb') as f:
    f.writelines(new_lines)
print("Fixed logic in part1.hlxc")
