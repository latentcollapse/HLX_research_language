with open('part1_debug.hlxc', 'wb') as f:
    content = b'''// HLX SELF-HOSTED COMPILER (DEBUG)

fn is_digit(c) -> bool {
    let code = ord(c);
    return code >= 48 and code <= 57;
}

fn is_alpha(c) -> bool {
    let code = ord(c);
    if code >= 97 and code <= 122 { return true; }
    if code >= 65 and code <= 90 { return true; }
    if code == 95 { return true; }
    return false;
}

fn tokenize(source) -> object {
    let tokens = [];
    let i = 0;
    let len = len(source);
    
    loop (i < len, 1000000) {
        let c = source[i];
        
        if c == " " or c == "\n" or c == "\t" or c == "\r" {
            i = i + 1;
        } else {
            if is_alpha(c) {
                let ident = "";
                loop (i < len and is_alpha(source[i]), 100) {
                    ident = ident + source[i];
                    i = i + 1;
                }
                let type = "IDENT";
                if ident == "fn" { type = "KW_FN"; }
                if ident == "let" { type = "KW_LET"; }
                if ident == "return" { type = "KW_RETURN"; }
                if ident == "if" { type = "KW_IF"; }
                if ident == "else" { type = "KW_ELSE"; }
                if ident == "loop" { type = "KW_LOOP"; }
                if ident == "break" { type = "KW_BREAK"; }
                if ident == "continue" { type = "KW_CONT"; }
                if ident == "and" { type = "OP_AND"; }
                if ident == "or" { type = "OP_OR"; }
                if ident == "true" { type = "LIT_BOOL"; }
                if ident == "false" { type = "LIT_BOOL"; }
                if ident == "null" { type = "LIT_NULL"; }
                tokens = append(tokens, {"type": type, "val": ident});
            } else {
                if is_digit(c) {
                    let num = "";
                    loop (i < len and is_digit(source[i]), 100) {
                        num = num + source[i];
                        i = i + 1;
                    }
                    tokens = append(tokens, {"type": "LIT_INT", "val": num});
                } else {
                    if c == "\"" {
                        i = i + 1;
                        let s = "";
                        loop (i < len and source[i] != "\"", 1000000) {
                            if source[i] == "\"" and i + 1 < len and source[i+1] == "\"" {
                                s = s + "\"";
                                i = i + 2;
                            } else {
                                s = s + source[i];
                                i = i + 1;
                            }
                        }
                        i = i + 1;
                        tokens = append(tokens, {"type": "LIT_STR", "val": s});
                    } else {
                        // DUMMY REST
                        i = i + 1;
                    }
                }
            }
        }
    }
    return tokens;
}

fn parse_primary(state) -> object {
    return state;
}
'''
    f.write(content)
    f.write(b'\n')

print("Generated part1_debug.hlxc with HEX ESCAPES and single quotes")