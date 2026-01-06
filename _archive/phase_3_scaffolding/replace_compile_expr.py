
import os

file_path = "hlx_compiler/bootstrap/compiler.hlxc"

new_compile_expr = r"""
fn compile_expr(_context_in, _expr) -> object {
    // Sanity check entry
    if _context_in.z_bc == null {
        print("FATAL: compile_expr ENTRY corrupted context! keys:", _context_in);
    }

    // Manual clone to break COW aliasing ghosts
    let _context = {
        "tokens": _context_in.tokens,
        "pos": _context_in.pos,
        "z_idx": _context_in.z_idx,
        "z_sym": _context_in.z_sym,
        "z_bc": _context_in.z_bc,
        "node": _context_in.node,
        "result_reg": _context_in.result_reg,
        "stack": _context_in.stack
    };
    
    // print("DEBUG: compile_expr keys=", len(_context));
    // print("DEBUG: z_sym len entry=", len(_context.z_sym));

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
            "tokens": _context.tokens,
            "pos": _context.pos,
            "z_idx": _r + 1,
            "z_sym": _context.z_sym,
            "z_bc": append(_context.z_bc, {"op": "Constant", "out": _r, "val": _v}),
            "node": _context.node,
            "result_reg": _r,
            "stack": _context.stack
        };
    } else {
        if _t == "Ident" {
            _context.result_reg = _context.z_sym[_expr.name];
        } else {
            if _t == "BinOp" {
                _context = compile_expr(_context, _expr.lhs); 
                // Push LHS result
                _context = {
                    "tokens": _context.tokens,
                    "pos": _context.pos,
                    "z_idx": _context.z_idx,
                    "z_sym": _context.z_sym,
                    "z_bc": _context.z_bc,
                    "node": _context.node,
                    "result_reg": _context.result_reg,
                    "stack": append(_context.stack, _context.result_reg)
                };
                
                _context = compile_expr(_context, _expr.rhs); 
                
                // Use stack top (LHS) and result_reg (RHS)
                _context = {
                    "tokens": _context.tokens,
                    "pos": _context.pos,
                    "z_idx": _context.z_idx + 1,
                    "z_sym": _context.z_sym,
                    "z_bc": append(_context.z_bc, {
                        "op": _expr.op, 
                        "out": _context.z_idx, 
                        "lhs": _context.stack[len(_context.stack)-1], 
                        "rhs": _context.result_reg
                    }),
                    "node": _context.node,
                    "result_reg": _context.z_idx,
                    "stack": slice(_context.stack, 0, len(_context.stack) - 1) // Pop LHS
                };
            } else {
                if _t == "Call" {
                    let _j = 0;
                    loop (_j < len(_expr.args), 100000) {
                        _context = compile_expr(_context, _expr.args[_j]); 
                        _context = {
                            "tokens": _context.tokens,
                            "pos": _context.pos,
                            "z_idx": _context.z_idx,
                            "z_sym": _context.z_sym,
                            "z_bc": _context.z_bc,
                            "node": _context.node,
                            "result_reg": _context.result_reg,
                            "stack": append(_context.stack, _context.result_reg)
                        };
                        _j = _j + 1;
                    }
                    
                    _context = {
                        "tokens": _context.tokens,
                        "pos": _context.pos,
                        "z_idx": _context.z_idx + 1,
                        "z_sym": _context.z_sym,
                        "z_bc": append(_context.z_bc, {"op": "Call", "out": _context.z_idx, "func": _expr.func, "args": slice(_context.stack, len(_context.stack) - len(_expr.args), len(_expr.args))}),
                        "node": _context.node,
                        "result_reg": _context.z_idx,
                        "stack": slice(_context.stack, 0, len(_context.stack) - len(_expr.args))
                    };
                } else {
                    if _t == "Index" {
                        _context = compile_expr(_context, _expr.object); 
                        
                        _context = {
                            "tokens": _context.tokens,
                            "pos": _context.pos,
                            "z_idx": _context.z_idx,
                            "z_sym": _context.z_sym,
                            "z_bc": _context.z_bc,
                            "node": _context.node,
                            "result_reg": _context.result_reg,
                            "stack": append(_context.stack, _context.result_reg)
                        };
                        
                        _context = compile_expr(_context, _expr.index); 
                        
                        _context = {
                            "tokens": _context.tokens,
                            "pos": _context.pos,
                            "z_idx": _context.z_idx + 1,
                            "z_sym": _context.z_sym,
                            "z_bc": append(_context.z_bc, {"op": "Index", "out": _context.z_idx, "container": _context.stack[len(_context.stack)-1], "index": _context.result_reg}),
                            "node": _context.node,
                            "result_reg": _context.z_idx,
                            "stack": slice(_context.stack, 0, len(_context.stack)-1)
                        };
                    } else {
                        if _t == "Field" {
                            _context = compile_expr(_context, _expr.object); 
                            
                            _context = {
                                "tokens": _context.tokens,
                                "pos": _context.pos,
                                "z_idx": _context.z_idx + 2,
                                "z_sym": _context.z_sym,
                                "z_bc": append(
                                    append(_context.z_bc, {"op": "Constant", "out": _context.z_idx, "val": _expr.field}),
                                    {"op": "Index", "out": _context.z_idx + 1, "container": _context.result_reg, "index": _context.z_idx}
                                ),
                                "node": _context.node,
                                "result_reg": _context.z_idx + 1,
                                "stack": _context.stack
                            };
                        } else {
                            if _t == "Array" {
                                let _j = 0;
                                loop (_j < len(_expr.elements), 100000) {
                                    _context = compile_expr(_context, _expr.elements[_j]); 
                                    _context = {
                                        "tokens": _context.tokens,
                                        "pos": _context.pos,
                                        "z_idx": _context.z_idx,
                                        "z_sym": _context.z_sym,
                                        "z_bc": _context.z_bc,
                                        "node": _context.node,
                                        "result_reg": _context.result_reg,
                                        "stack": append(_context.stack, _context.result_reg)
                                    };
                                    _j = _j + 1;
                                }
                                
                                _context = {
                                    "tokens": _context.tokens,
                                    "pos": _context.pos,
                                    "z_idx": _context.z_idx + 1,
                                    "z_sym": _context.z_sym,
                                    "z_bc": append(_context.z_bc, {"op": "ArrayCreate", "out": _context.z_idx, "elements": slice(_context.stack, len(_context.stack) - len(_expr.elements), len(_expr.elements))}),
                                    "node": _context.node,
                                    "result_reg": _context.z_idx,
                                    "stack": slice(_context.stack, 0, len(_context.stack) - len(_expr.elements))
                                };
                            } else {
                                if _t == "Object" {
                                    let _j = 0;
                                    loop (_j < len(_expr.values), 100000) {
                                        _context = compile_expr(_context, _expr.values[_j]); 
                                        _context = {
                                            "tokens": _context.tokens,
                                            "pos": _context.pos,
                                            "z_idx": _context.z_idx,
                                            "z_sym": _context.z_sym,
                                            "z_bc": _context.z_bc,
                                            "node": _context.node,
                                            "result_reg": _context.result_reg,
                                            "stack": append(_context.stack, _context.result_reg)
                                        };
                                        _j = _j + 1;
                                    }
                                    
                                    _context = {
                                        "tokens": _context.tokens,
                                        "pos": _context.pos,
                                        "z_idx": _context.z_idx + 1,
                                        "z_sym": _context.z_sym,
                                        "z_bc": append(_context.z_bc, {"op": "ObjectCreate", "out": _context.z_idx, "keys": _expr.keys, "values": slice(_context.stack, len(_context.stack) - len(_expr.values), len(_expr.values))}),
                                        "node": _context.node,
                                        "result_reg": _context.z_idx,
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
}
"""

with open(file_path, "r") as f:
    content = f.read()

start_marker = "fn compile_expr(_context_in, _expr) -> object {"
end_marker = "fn compile_stmt(_global_state, _stmt) -> object {"

start_idx = content.find(start_marker)
end_idx = content.find(end_marker)

if start_idx == -1 or end_idx == -1:
    print("Error: Could not find markers")
    exit(1)

# Remove the incorrect content and replace
new_content = content[:start_idx] + new_compile_expr.strip() + "\n\n" + content[end_idx:]

with open(file_path, "w") as f:
    f.write(new_content)

print("Successfully replaced compile_expr")
