
import os

tokenizer_parser = r"""// HLX SELF-HOSTED COMPILER (Phase 4.2: Flat Context)

// ========================================== 
// 1. TOKENIZER
// ========================================== 

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
                    if c == "\\\"" {
                        i = i + 1;
                        let s = "";
                        loop (i < len and source[i] != "\\\"", 1000000) {
                            if source[i] == "\\\\" and i + 1 < len and source[i+1] == "\\\"" {
                                s = s + "\\\"";
                                i = i + 2;
                            } else {
                                s = s + source[i];
                                i = i + 1;
                            }
                        }
                        i = i + 1;
                        tokens = append(tokens, {"type": "LIT_STR", "val": s});
                    } else {
                        let t = null;
                        if c == "{" { t = "LBRACE"; }
                        if c == "}" { t = "RBRACE"; }
                        if c == "[" { t = "LBRACK"; }
                        if c == "]" { t = "RBRACK"; }
                        if c == "(" { t = "LPAREN"; }
                        if c == ")" { t = "RPAREN"; }
                        if c == ";" { t = "SEMI"; }
                        if c == ":" { t = "COLON"; }
                        if c == "," { t = "COMMA"; }
                        if c == "." { t = "DOT"; }
                        if c == "+" { t = "OP_ADD"; }
                        if c == "-" { t = "OP_SUB"; }
                        if c == "*" { t = "OP_MUL"; }
                        
                        if t != null {
                            tokens = append(tokens, {"type": t, "val": c});
                            i = i + 1;
                        } else {
                            if c == "=" {
                                if i + 1 < len and source[i+1] == "=" {
                                    tokens = append(tokens, {"type": "OP_EQ", "val": "=="}); i = i + 2;
                                } else {
                                    tokens = append(tokens, {"type": "OP_ASSIGN", "val": "="}); i = i + 1;
                                }
                            } else {
                                if c == "!" {
                                    if i + 1 < len and source[i+1] == "=" {
                                        tokens = append(tokens, {"type": "OP_NE", "val": "!="}); i = i + 2;
                                    } else { i = i + 1; }
                                } else {
                                    if c == "<" {
                                        if i + 1 < len and source[i+1] == "=" {
                                            tokens = append(tokens, {"type": "OP_LE", "val": "<="}); i = i + 2;
                                        } else {
                                            tokens = append(tokens, {"type": "OP_LT", "val": "<"}); i = i + 1;
                                        }
                                    } else {
                                        if c == ">" {
                                            if i + 1 < len and source[i+1] == "=" {
                                                tokens = append(tokens, {"type": "OP_GE", "val": ">="}); i = i + 2;
                                            } else {
                                                tokens = append(tokens, {"type": "OP_GT", "val": ">"}); i = i + 1;
                                            }
                                        } else {
                                            i = i + 1;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return tokens;
}

// ========================================== 
// 2. PARSER
// ========================================== 

fn parse_primary(state) -> object {
    let i = state.pos;
    let t = state.tokens[i];
    
    if t.type == "LIT_INT" or t.type == "LIT_STR" or t.type == "LIT_BOOL" or t.type == "LIT_NULL" {
        state.node = {"type": "Literal", "val": t.val, "lit_type": t.type};
        state.pos = i + 1;
    } else {
        if t.type == "LBRACK" {
            state.pos = i + 1;
            let els = [];
            loop (state.tokens[state.pos].type != "RBRACK", 100000) {
                state = parse_expr(state);
                els = append(els, state.node);
                if state.tokens[state.pos].type == "COMMA" { state.pos = state.pos + 1; }
            }
            state.pos = state.pos + 1;
            state.node = {"type": "Array", "elements": els};
        } else {
            if t.type == "LBRACE" {
                state.pos = i + 1;
                let ks = []; let vs = [];
                loop (state.tokens[state.pos].type != "RBRACE", 100000) {
                    let key = state.tokens[state.pos].val;
                    state.pos = state.pos + 2;
                    state = parse_expr(state);
                    ks = append(ks, key); vs = append(vs, state.node);
                    if state.tokens[state.pos].type == "COMMA" { state.pos = state.pos + 1; }
                }
                state.pos = state.pos + 1;
                state.node = {"type": "Object", "keys": ks, "values": vs};
            } else {
                if t.type == "IDENT" {
                    state.node = {"type": "Ident", "name": t.val};
                    state.pos = i + 1;
                    loop (state.pos < len(state.tokens), 100000) {
                        let n = state.tokens[state.pos];
                        if n.type == "LPAREN" {
                            let name = state.node.name; state.pos = state.pos + 1;
                            let args = [];
                            loop (state.tokens[state.pos].type != "RPAREN", 100000) {
                                state = parse_expr(state); args = append(args, state.node);
                                if state.tokens[state.pos].type == "COMMA" { state.pos = state.pos + 1; }
                            }
                            state.pos = state.pos + 1;
                            state.node = {"type": "Call", "func": name, "args": args};
                        } else {
                            if n.type == "LBRACK" {
                                let o = state.node; state.pos = state.pos + 1;
                                state = parse_expr(state); state.pos = state.pos + 1;
                                state.node = {"type": "Index", "object": o, "index": state.node};
                            } else {
                                if n.type == "DOT" {
                                    let o = state.node; state.pos = state.pos + 2;
                                    state.node = {"type": "Field", "object": o, "field": state.tokens[state.pos-1].val};
                                } else { break; }
                            }
                        }
                    }
                }
            }
        }
    }
    return state;
}

fn parse_expr(state) -> object {
    state = parse_primary(state);
    let lhs = state.node;
    if state.pos < len(state.tokens) {
        let op = state.tokens[state.pos];
        let ot = null;
        if op.type == "OP_ADD" { ot = "Add"; }
        if op.type == "OP_SUB" { ot = "Sub"; }
        if op.type == "OP_MUL" { ot = "Mul"; }
        if op.type == "OP_EQ"  { ot = "Eq"; }
        if op.type == "OP_NE"  { ot = "Ne"; }
        if op.type == "OP_LT"  { ot = "Lt"; }
        if op.type == "OP_GT"  { ot = "Gt"; }
        if op.type == "OP_LE"  { ot = "Le"; }
        if op.type == "OP_GE"  { ot = "Ge"; }
        if op.type == "OP_AND" { ot = "And"; }
        if op.type == "OP_OR"  { ot = "Or"; }
        if op.type == "OP_ASSIGN" { ot = "Assign"; }
        if ot != null {
            state.pos = state.pos + 1;
            state = parse_expr(state);
            state.node = {"type": "BinOp", "op": ot, "lhs": lhs, "rhs": state.node};
        }
    }
    return state;
}

fn parse_stmt(state) -> object {
    let t = state.tokens[state.pos];
    if t.type == "KW_LET" {
        let name = state.tokens[state.pos+1].val; state.pos = state.pos + 3;
        state = parse_expr(state);
        state.node = {"type": "Let", "name": name, "value": state.node};
        state.pos = state.pos + 1;
    } else {
        if t.type == "KW_RETURN" {
            state.pos = state.pos + 1;
            state = parse_expr(state);
            state.node = {"type": "Return", "value": state.node};
            state.pos = state.pos + 1;
        } else {
            if t.type == "KW_IF" {
                state.pos = state.pos + 1; state = parse_expr(state);
                let cond = state.node; state.pos = state.pos + 1;
                let then = [];
                loop (state.tokens[state.pos].type != "RBRACE", 100000) {
                    state = parse_stmt(state); then = append(then, state.node);
                }
                state.pos = state.pos + 1;
                state.node = {"type": "If", "cond": cond, "then": then};
            } else {
                if t.type == "KW_LOOP" {
                    state.pos = state.pos + 2; state = parse_expr(state);
                    let cond = state.node; state.pos = state.pos + 1;
                    let mx = state.tokens[state.pos].val; state.pos = state.pos + 3;
                    let bdy = [];
                    loop (state.tokens[state.pos].type != "RBRACE", 100000) {
                        state = parse_stmt(state); bdy = append(bdy, state.node);
                    }
                    state.pos = state.pos + 1;
                    state.node = {"type": "Loop", "cond": cond, "body": bdy, "max": mx};
                } else {
                    if t.type == "KW_BREAK" {
                        state.node = {"type": "Break"};
                        state.pos = state.pos + 2; // break ;
                    } else {
                        if t.type == "KW_CONT" {
                            state.node = {"type": "Continue"};
                            state.pos = state.pos + 2; // continue ;
                        } else {
                            state = parse_expr(state);
                            let n = state.node;
                            if n.type == "BinOp" {
                                if n.op == "Assign" {
                                    state.node = {"type": "Assign", "lhs": n.lhs, "rhs": n.rhs};
                                }
                            }
                            state.pos = state.pos + 1;
                        }
                    }
                }
            }
        }
    }
    return state;
}"""

lowerer_expr = r"""fn compile_expr(_context_in, _expr) -> object {
    if _context_in.z_bc == null {
        print("FATAL: compile_expr ENTRY corrupted context! keys:", _context_in);
    }
    let _context = {
        "tokens": _context_in.tokens, "pos": _context_in.pos, "z_idx": _context_in.z_idx,
        "z_sym": _context_in.z_sym, "z_bc": _context_in.z_bc, "node": _context_in.node,
        "result_reg": _context_in.result_reg, "stack": _context_in.stack
    };
    let _t = _expr.type;
    if _t == "Literal" {
        let _r = _context.z_idx;
        let _v = _expr.val;
        if _expr.lit_type == "LIT_BOOL" {
            if _v == "true" { _v = true; } else { _v = false; }
        } else {
            if _expr.lit_type == "LIT_NULL" { _v = null; }
        }
        _context = {
            "tokens": _context.tokens, "pos": _context.pos, "z_idx": _r + 1, "z_sym": _context.z_sym,
            "z_bc": append(_context.z_bc, {"op": "Constant", "out": _r, "val": _v}),
            "node": _context.node, "result_reg": _r, "stack": _context.stack
        };
    } else {
        if _t == "Ident" {
            _context.result_reg = _context.z_sym[_expr.name];
        } else {
            if _t == "BinOp" {
                _context = compile_expr(_context, _expr.lhs);
                _context = {
                    "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx, "z_sym": _context.z_sym,
                    "z_bc": _context.z_bc, "node": _context.node, "result_reg": _context.result_reg,
                    "stack": append(_context.stack, _context.result_reg)
                };
                _context = compile_expr(_context, _expr.rhs);
                _context = {
                    "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 1, "z_sym": _context.z_sym,
                    "z_bc": append(_context.z_bc, {
                        "op": _expr.op, "out": _context.z_idx,
                        "lhs": _context.stack[len(_context.stack)-1], "rhs": _context.result_reg
                    }),
                    "node": _context.node, "result_reg": _context.z_idx,
                    "stack": slice(_context.stack, 0, len(_context.stack) - 1)
                };
            } else {
                if _t == "Call" {
                    let _j = 0;
                    loop (_j < len(_expr.args), 100000) {
                        _context = compile_expr(_context, _expr.args[_j]);
                        _context = {
                            "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx, "z_sym": _context.z_sym,
                            "z_bc": _context.z_bc, "node": _context.node, "result_reg": _context.result_reg,
                            "stack": append(_context.stack, _context.result_reg)
                        };
                        _j = _j + 1;
                    }
                    _context = {
                        "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 1, "z_sym": _context.z_sym,
                        "z_bc": append(_context.z_bc, {"op": "Call", "out": _context.z_idx, "func": _expr.func, "args": slice(_context.stack, len(_context.stack) - len(_expr.args), len(_expr.args))}),
                        "node": _context.node, "result_reg": _context.z_idx,
                        "stack": slice(_context.stack, 0, len(_context.stack) - len(_expr.args))
                    };
                } else {
                    if _t == "Index" {
                        _context = compile_expr(_context, _expr.object);
                        _context = {
                            "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx, "z_sym": _context.z_sym,
                            "z_bc": _context.z_bc, "node": _context.node, "result_reg": _context.result_reg,
                            "stack": append(_context.stack, _context.result_reg)
                        };
                        _context = compile_expr(_context, _expr.index);
                        _context = {
                            "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 1, "z_sym": _context.z_sym,
                            "z_bc": append(_context.z_bc, {"op": "Index", "out": _context.z_idx, "container": _context.stack[len(_context.stack)-1], "index": _context.result_reg}),
                            "node": _context.node, "result_reg": _context.z_idx,
                            "stack": slice(_context.stack, 0, len(_context.stack)-1)
                        };
                    } else {
                        if _t == "Field" {
                            _context = compile_expr(_context, _expr.object);
                            _context = {
                                "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 2, "z_sym": _context.z_sym,
                                "z_bc": append(
                                    append(_context.z_bc, {"op": "Constant", "out": _context.z_idx, "val": _expr.field}),
                                    {"op": "Index", "out": _context.z_idx + 1, "container": _context.result_reg, "index": _context.z_idx}
                                ),
                                "node": _context.node, "result_reg": _context.z_idx + 1, "stack": _context.stack
                            };
                        } else {
                            if _t == "Array" {
                                let _j = 0;
                                loop (_j < len(_expr.elements), 100000) {
                                    _context = compile_expr(_context, _expr.elements[_j]);
                                    _context = {
                                        "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx, "z_sym": _context.z_sym,
                                        "z_bc": _context.z_bc, "node": _context.node, "result_reg": _context.result_reg,
                                        "stack": append(_context.stack, _context.result_reg)
                                    };
                                    _j = _j + 1;
                                }
                                _context = {
                                    "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 1, "z_sym": _context.z_sym,
                                    "z_bc": append(_context.z_bc, {"op": "ArrayCreate", "out": _context.z_idx, "elements": slice(_context.stack, len(_context.stack) - len(_expr.elements), len(_expr.elements))}),
                                    "node": _context.node, "result_reg": _context.z_idx,
                                    "stack": slice(_context.stack, 0, len(_context.stack) - len(_expr.elements))
                                };
                            } else {
                                if _t == "Object" {
                                    let _j = 0;
                                    loop (_j < len(_expr.values), 100000) {
                                        _context = compile_expr(_context, _expr.values[_j]);
                                        _context = {
                                            "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx, "z_sym": _context.z_sym,
                                            "z_bc": _context.z_bc, "node": _context.node, "result_reg": _context.result_reg,
                                            "stack": append(_context.stack, _context.result_reg)
                                        };
                                        _j = _j + 1;
                                    }
                                    _context = {
                                        "tokens": _context.tokens, "pos": _context.pos, "z_idx": _context.z_idx + 1, "z_sym": _context.z_sym,
                                        "z_bc": append(_context.z_bc, {"op": "ObjectCreate", "out": _context.z_idx, "keys": _expr.keys, "values": slice(_context.stack, len(_context.stack) - len(_expr.values), len(_expr.values))}),
                                        "node": _context.node, "result_reg": _context.z_idx,
                                        "stack": slice(_context.stack, 0, len(_context.stack) - len(_expr.values))
                                    };
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    return _context;
}"""

lowerer_stmt = r"""fn compile_stmt(_global_state, _stmt) -> object {
    if _global_state.z_bc == null { print("FATAL: compile_stmt ENTRY corrupted"); }
    if _stmt == null { return _global_state; }
    if _stmt.type == "Let" {
        _global_state = compile_expr(_global_state, _stmt.value);
        let _m = _global_state.z_sym; _m[_stmt.name] = _global_state.result_reg;
        _global_state.z_sym = _m;
    } else {
        if _stmt.type == "Assign" {
            _global_state = compile_expr(_global_state, _stmt.rhs); 
            let _v = _global_state.result_reg;
            if _stmt.lhs.type == "Ident" {
                _global_state.z_bc = append(_global_state.z_bc, {"op": "Move", "out": _global_state.z_sym[_stmt.lhs.name], "src": _v});
            } else {
                if _stmt.lhs.type == "Index" {
                    _global_state = compile_expr(_global_state, _stmt.lhs.object); 
                    let _o = _global_state.result_reg;
                    _global_state = compile_expr(_global_state, _stmt.lhs.index);
                    _global_state.z_bc = append(_global_state.z_bc, {"op": "Store", "container": _o, "index": _global_state.result_reg, "value": _v});
                } else {
                    if _stmt.lhs.type == "Field" {
                        _global_state = compile_expr(_global_state, _stmt.lhs.object); 
                        let _o = _global_state.result_reg;
                        let _k = _global_state.z_idx;
                        _global_state.z_idx = _k + 1;
                        _global_state.z_bc = append(_global_state.z_bc, {"op": "Constant", "out": _k, "val": _stmt.lhs.field});
                        _global_state.z_bc = append(_global_state.z_bc, {"op": "Store", "container": _o, "index": _k, "value": _v});
                    }
                }
            }
        } else {
            if _stmt.type == "Return" {
                _global_state = compile_expr(_global_state, _stmt.value);
                _global_state.z_bc = append(_global_state.z_bc, {"op": "Return", "val": _global_state.result_reg});
            } else {
                if _stmt.type == "If" {
                    _global_state = compile_expr(_global_state, _stmt.cond); 
                    let _c = _global_state.result_reg;
                    let _if_idx = len(_global_state.z_bc); _global_state.z_bc = append(_global_state.z_bc, {"op": "IfPlaceholder"});
                    let _j = 0; loop (_j < len(_stmt.then), 100000) { 
                        _global_state = compile_stmt(_global_state, _stmt.then[_j]); 
                        _j = _j + 1; 
                    }
                    let _is = _global_state.z_bc; _is[_if_idx] = {"op": "If", "cond": _c, "then": _if_idx + 1, "else": len(_is)};
                    _global_state.z_bc = _is;
                } else {
                    if _stmt.type == "Loop" {
                        let _start = len(_global_state.z_bc); 
                        _global_state = compile_expr(_global_state, _stmt.cond); 
                        let _c = _global_state.result_reg;
                        let _if_idx = len(_global_state.z_bc); _global_state.z_bc = append(_global_state.z_bc, {"op": "IfPlaceholder"});
                        let _j = 0; loop (_j < len(_stmt.body), 100000) { 
                            _global_state = compile_stmt(_global_state, _stmt.body[_j]); 
                            _j = _j + 1; 
                        }
                        _global_state.z_bc = append(_global_state.z_bc, {"op": "Jump", "target": _start});
                        let _is = _global_state.z_bc; _is[_if_idx] = {"op": "If", "cond": _c, "then": _if_idx + 1, "else": len(_is)};
                        _global_state.z_bc = _is;
                    } else {
                        if _stmt.type == "Break" {
                            _global_state.z_bc = append(_global_state.z_bc, {"op": "Break"});
                        } else {
                            if _stmt.type == "Continue" {
                                _global_state.z_bc = append(_global_state.z_bc, {"op": "Continue"});
                            }
                        }
                    }
                }
            }
        }
    }
    return _global_state;
}"""

compiler_main = r"""fn compile(_tokens) -> object {
    let _global_state = {
        "tokens": _tokens, "pos": 0, "z_idx": 0, "z_sym": {}, "z_bc": [],
        "node": null, "result_reg": 0, "stack": []
    };
    let _lt = len(_tokens);
    loop (_global_state.pos < _lt, 1000000) {
        let _t = _tokens[_global_state.pos];
        if _t.type == "KW_FN" {
            let _name = _tokens[_global_state.pos+1].val;
            _global_state.pos = _global_state.pos + 3; 
            let _params = [];
            loop (_global_state.tokens[_global_state.pos].type != "RPAREN", 1000000) {
                _params = append(_params, _global_state.tokens[_global_state.pos].val);
                _global_state.pos = _global_state.pos + 1;
                if (_global_state.tokens[_global_state.pos].type == "COMMA") { _global_state.pos = _global_state.pos + 1; }
            }
            _global_state.pos = _global_state.pos + 2; 
            let _old_next = _global_state.z_idx;
            let _old_map = _global_state.z_sym;
            _global_state.z_idx = 0;
            _global_state.z_sym = {};
            let _param_regs = [];
            let _j = 0;
            loop (_j < len(_params), 1000000) {
                let _r = _global_state.z_idx;
                let _m = _global_state.z_sym; _m[_params[_j]] = _r; _global_state.z_sym = _m;
                _global_state.z_idx = _r + 1;
                _param_regs = append(_param_regs, _r);
                _j = _j + 1;
            }
            let _start_pc = len(_global_state.z_bc);
            print("Compiling function: " + _name);
            loop (_global_state.tokens[_global_state.pos].type != "RBRACE", 1000000) {
                _global_state = parse_stmt(_global_state);
                _global_state = compile_stmt(_global_state, _global_state.node);
            }
            _global_state.pos = _global_state.pos + 1;
            _global_state.z_bc = append(_global_state.z_bc, {"op": "FuncDef", "name": _name, "params": _param_regs, "body": _start_pc});
            _global_state.z_idx = _old_next;
            _global_state.z_sym = _old_map;
        } else {
            _global_state.pos = _global_state.pos + 1;
        }
    }
    return _global_state.z_bc;
}

fn main() -> object {
    let source = read_file("hlx_compiler/bootstrap/compiler.hlxc");
    let tokens = native_tokenize(source);
    let ir = compile(tokens);
    return ir;
}"""

full_content = tokenizer_parser.strip() + "\n\n" + lowerer_expr.strip() + "\n\n" + lowerer_stmt.strip() + "\n\n" + compiler_main.strip() + "\n"

with open("hlx_compiler/bootstrap/compiler.hlxc", "w") as f:
    f.write(full_content)

print("Successfully rebuilt compiler.hlxc")
