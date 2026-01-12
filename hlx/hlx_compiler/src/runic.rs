//! HLX (Runic) Parser and Emitter
//!
//! Parses and emits the glyph-based HLX syntax.
//! Bijective with HLXL (ASCII) form.

use crate::ast::*;
use crate::parser::Parser;
use crate::emitter::Emitter;
use hlx_core::Result;

/// HLX Runic Parser
pub struct RunicParser;

impl RunicParser {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RunicParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Parser for RunicParser {
    fn parse(&self, source: &str) -> Result<Program> {
        // For now, convert runic to HLXA and parse that
        // In a full implementation, this would be a native runic parser
        let hlxa_source = transliterate_to_hlxl(source)?;
        crate::HlxaParser::new().parse(&hlxa_source)
    }
    
    fn name(&self) -> &'static str {
        "HLX-Runic"
    }
}

/// HLX Runic Emitter
pub struct RunicEmitter {
    indent: usize,
}

impl RunicEmitter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }
    
    fn indent_str(&self) -> String {
        "  ".repeat(self.indent)
    }
}

impl Default for RunicEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for RunicEmitter {
    fn emit(&self, program: &Program) -> Result<String> {
        let mut output = String::new();
        let mut emitter = RunicEmitter::new();
        
        // Program header
        output.push_str(&format!("{} {} {{\n", glyphs::PROGRAM, program.name));
        emitter.indent += 1;
        
        // Blocks
        for block in &program.blocks {
            emitter.emit_block(block, &mut output)?;
            output.push('\n');
        }
        
        output.push_str("}\n");
        Ok(output)
    }
    
    fn name(&self) -> &'static str {
        "HLX-Runic"
    }
}

impl RunicEmitter {
    fn emit_block(&mut self, block: &Block, out: &mut String) -> Result<()> {
        out.push_str(&self.indent_str());
        out.push_str(&format!("{} {}(", glyphs::BLOCK, block.name));
        
        let params_str = block.params.iter()
            .map(|(name, _name_span, typ_opt)| {
                if let Some((t, _type_span)) = typ_opt {
                    format!("{}: {}", name, t.to_string())
                } else {
                    name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
            
        out.push_str(&params_str);
        out.push_str(") {\n");
        
        self.indent += 1;
        for item in &block.items {
            match &item.node {
                Item::Statement(s) => self.emit_stmt(s, out)?,
                Item::Node(n) => {
                    // TODO: implement native runic node emission
                    out.push_str(&format!("node {}: {} -> {:?}", n.id.as_deref().unwrap_or(""), n.op, n.outputs));
                }
            }
            out.push('\n');
        }
        self.indent -= 1;
        
        out.push_str(&self.indent_str());
        out.push('}');
        Ok(())
    }
    
    fn emit_stmt(&mut self, stmt: &Statement, out: &mut String) -> Result<()> {
        out.push_str(&self.indent_str());
        
        match stmt {
            Statement::Let { name, type_annotation, value, .. } => {
                out.push_str(glyphs::LET);
                out.push(' ');
                out.push_str(name);
                // TODO: Emit type annotation in runic form if present
                if let Some(_typ) = type_annotation {
                    // For now, skip type annotations in runic output
                }
                out.push_str(" = ");
                self.emit_expr(&value.node, out)?;
                out.push(';');
            }
            Statement::Local { name, value, .. } => {
                out.push_str(glyphs::LOCAL);
                out.push(' ');
                out.push_str(name);
                out.push_str(" = ");
                self.emit_expr(&value.node, out)?;
                out.push(';');
            }
            
            Statement::Assign { lhs, value } => {
                self.emit_expr(&lhs.node, out)?;
                out.push(' ');
                out.push_str(glyphs::ASSIGN);
                out.push(' ');
                self.emit_expr(&value.node, out)?;
                out.push(';');
            }
            
            Statement::Return { value, .. } => {
                out.push_str(glyphs::RETURN);
                out.push(' ');
                self.emit_expr(&value.node, out)?;
                out.push(';');
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                out.push_str(glyphs::IF);
                out.push_str(" (");
                self.emit_expr(&condition.node, out)?;
                out.push_str(") {\n");
                
                self.indent += 1;
                for s in then_branch {
                    self.emit_stmt(&s.node, out)?;
                    out.push('\n');
                }
                self.indent -= 1;
                
                out.push_str(&self.indent_str());
                out.push('}');
                
                if let Some(else_stmts) = else_branch {
                    out.push(' ');
                    out.push_str(glyphs::ELSE);
                    out.push_str(" {\n");
                    
                    self.indent += 1;
                    for s in else_stmts {
                        self.emit_stmt(&s.node, out)?;
                        out.push('\n');
                    }
                    self.indent -= 1;
                    
                    out.push_str(&self.indent_str());
                    out.push('}');
                }
            }
            
            Statement::While { condition, body, .. } => {
                out.push_str(glyphs::WHILE);
                self.emit_expr(&condition.node, out)?;
                out.push_str(" { ");
                for stmt in body {
                    self.emit_stmt(&stmt.node, out)?;
                }
                out.push_str(" }");
            }
            
            Statement::Break => {
                out.push_str("break;");
            }

            Statement::Continue => {
                out.push_str("continue;");
            }
            
            Statement::Expr(e) => {
                self.emit_expr(&e.node, out)?;
            }
            
            Statement::Asm { template, .. } => {
                out.push_str("asm(\"");
                out.push_str(template);
                out.push_str("\");");
            }
        }
        
        Ok(())
    }
    
    fn emit_expr(&self, expr: &Expr, out: &mut String) -> Result<()> {
        match expr {
            Expr::Literal(lit) => self.emit_literal(lit, out),
            
            Expr::Ident(name) => {
                out.push_str(name);
                Ok(())
            }
            
            Expr::Array(elems) => {
                out.push('[');
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    self.emit_expr(&e.node, out)?;
                }
                out.push(']');
                Ok(())
            }
            
            Expr::Object(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\": ");
                    self.emit_expr(&v.node, out)?;
                }
                out.push('}');
                Ok(())
            }
            
            Expr::BinOp { op, lhs, rhs } => {
                self.emit_expr(&lhs.node, out)?;
                out.push(' ');
                out.push_str(op.hlxl_str());
                out.push(' ');
                self.emit_expr(&rhs.node, out)?;
                Ok(())
            }
            
            Expr::UnaryOp { op, operand } => {
                out.push_str(op.runic_str());
                self.emit_expr(&operand.node, out)?;
                Ok(())
            }
            
            Expr::Call { func, args } => {
                self.emit_expr(&func.node, out)?;
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    self.emit_expr(&arg.node, out)?;
                }
                out.push(')');
                Ok(())
            }
            
            Expr::Index { object, index } => {
                self.emit_expr(&object.node, out)?;
                out.push('[');
                self.emit_expr(&index.node, out)?;
                out.push(']');
                Ok(())
            }
            
            Expr::Field { object, field } => {
                self.emit_expr(&object.node, out)?;
                out.push('.');
                out.push_str(field);
                Ok(())
            }
            
            Expr::Cast { expr, target_type } => {
                out.push('(');
                self.emit_expr(&expr.node, out)?;
                out.push_str(" as ");
                out.push_str(&format!("{:?}", target_type));
                out.push(')');
                Ok(())
            }
            
            Expr::Pipe { value, func } => {
                self.emit_expr(&value.node, out)?;
                out.push(' ');
                out.push_str(glyphs::PIPE);
                out.push(' ');
                self.emit_expr(&func.node, out)?;
                Ok(())
            }
            
            Expr::Collapse { table, namespace, value } => {
                out.push_str(glyphs::COLLAPSE);
                out.push(' ');
                out.push_str(table);
                out.push(' ');
                out.push_str(namespace);
                out.push(' ');
                self.emit_expr(&value.node, out)?;
                Ok(())
            }
            
            Expr::Resolve { target } => {
                out.push_str(glyphs::RESOLVE);
                out.push(' ');
                self.emit_expr(&target.node, out)?;
                Ok(())
            }
            
            Expr::Snapshot => {
                out.push_str(glyphs::SNAPSHOT);
                Ok(())
            }
            
            Expr::Transaction { body: _ } => {
                out.push_str(glyphs::TRANSACTION);
                out.push_str(" {\n");
                // TODO: emit body
                out.push('}');
                Ok(())
            }
            
            Expr::Handle(h) => {
                // Convert &h_xxx to ⟁xxx
                let name = h.strip_prefix("&h_").unwrap_or(h);
                out.push_str(glyphs::HANDLE_PREFIX);
                out.push_str(name);
                Ok(())
            }
            
            Expr::Contract { id, fields } => {
                out.push_str(&format!("@{} {{", id));
                for (i, (idx, val)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(&format!("@{}: ", idx));
                    self.emit_expr(&val.node, out)?;
                }
                out.push('}');
                Ok(())
            }
        }
    }
    
    fn emit_literal(&self, lit: &Literal, out: &mut String) -> Result<()> {
        match lit {
            Literal::Null => {
                out.push_str(glyphs::TYPE_NULL);
            }
            Literal::Bool(true) => {
                out.push_str(glyphs::TYPE_TRUE);
            }
            Literal::Bool(false) => {
                out.push_str(glyphs::TYPE_FALSE);
            }
            Literal::Int(i) => {
                out.push_str(glyphs::TYPE_INT);
                out.push_str(&i.to_string());
            }
            Literal::Float(f) => {
                out.push_str(glyphs::TYPE_FLOAT);
                out.push_str(&f.to_string());
            }
            Literal::String(s) => {
                out.push_str(glyphs::TYPE_STRING);
                out.push('"');
                out.push_str(&s.escape_default().to_string());
                out.push('"');
            }
            Literal::Array(elems) => {
                out.push_str(glyphs::ARRAY_OPEN);
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 { out.push(' '); }
                    self.emit_expr(&elem.node, out)?;
                }
                out.push_str(glyphs::ARRAY_CLOSE);
            }
            Literal::Object(fields) => {
                out.push_str(glyphs::OBJ_OPEN);
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 { out.push(' '); }
                    out.push_str(key);
                    out.push(':');
                    self.emit_expr(&val.node, out)?;
                }
                out.push_str(glyphs::OBJ_CLOSE);
            }
        }
        Ok(())
    }
}

/// HLXL Emitter (for completeness)
pub struct HlxlEmitter {
    indent: usize,
}

impl HlxlEmitter {
    pub fn new() -> Self {
        Self { indent: 0 }
    }
    
    fn indent_str(&self) -> String {
        "    ".repeat(self.indent)
    }
}

impl Default for HlxlEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Emitter for HlxlEmitter {
    fn emit(&self, program: &Program) -> Result<String> {
        let mut output = String::new();
        let mut emitter = HlxlEmitter::new();
        
        output.push_str(&format!("program {} {{\n", program.name));
        emitter.indent += 1;
        
        for block in &program.blocks {
            emitter.emit_block(block, &mut output)?;
            output.push('\n');
        }
        
        output.push_str("}\n");
        Ok(output)
    }
    
    fn name(&self) -> &'static str {
        "HLXL"
    }
}

impl HlxlEmitter {
    fn emit_block(&mut self, block: &Block, out: &mut String) -> Result<()> {
        out.push_str(&self.indent_str());
        out.push_str(&format!("block {}(", block.name));
        
        let params_str = block.params.iter()
            .map(|(name, _name_span, typ_opt)| {
                if let Some((t, _type_span)) = typ_opt {
                    format!("{}: {}", name, t.to_string())
                } else {
                    name.clone()
                }
            })
            .collect::<Vec<_>>()
            .join(", ");
            
        out.push_str(&params_str);
        out.push_str(") {\n");
        
        self.indent += 1;
        for item in &block.items {
            if let Item::Statement(s) = &item.node {
                self.emit_stmt(s, out)?;
                out.push('\n');
            }
        }
        self.indent -= 1;
        
        out.push_str(&self.indent_str());
        out.push('}');
        Ok(())
    }
    
    fn emit_stmt(&mut self, stmt: &Statement, out: &mut String) -> Result<()> {
        out.push_str(&self.indent_str());
        
        match stmt {
            Statement::Let { name, type_annotation: _, value, .. } => {
                out.push_str("let ");
                out.push_str(name);
                // Type annotations not emitted in this debug output
                out.push_str(" = ");
                self.emit_expr(&value.node, out)?;
            }

            Statement::Local { name, value, .. } => {
                out.push_str("local ");
                out.push_str(name);
                out.push_str(" = ");
                self.emit_expr(&value.node, out)?;
            }
            
            Statement::Assign { lhs, value } => {
                self.emit_expr(&lhs.node, out)?;
                out.push_str(" = ");
                self.emit_expr(&value.node, out)?;
            }
            
            Statement::Return { value, .. } => {
                out.push_str("return ");
                self.emit_expr(&value.node, out)?;
            }

            Statement::If { condition, then_branch, else_branch, .. } => {
                out.push_str("if (");
                self.emit_expr(&condition.node, out)?;
                out.push_str(") {\n");
                
                self.indent += 1;
                for s in then_branch {
                    self.emit_stmt(&s.node, out)?;
                    out.push('\n');
                }
                self.indent -= 1;
                
                out.push_str(&self.indent_str());
                out.push('}');
                
                if let Some(else_stmts) = else_branch {
                    out.push_str(" else {\n");
                    
                    self.indent += 1;
                    for s in else_stmts {
                        self.emit_stmt(&s.node, out)?;
                        out.push('\n');
                    }
                    self.indent -= 1;
                    
                    out.push_str(&self.indent_str());
                    out.push('}');
                }
            }
            
            Statement::While { condition, body, max_iter, .. } => {
                out.push_str("loop (");
                self.emit_expr(&condition.node, out)?;
                out.push_str(&format!(", {}", max_iter));
                out.push_str(") {\n");
                
                self.indent += 1;
                for s in body {
                    self.emit_stmt(&s.node, out)?;
                    out.push('\n');
                }
                self.indent -= 1;
                
                out.push_str(&self.indent_str());
                out.push('}');
            }
            
            Statement::Break => {
                out.push_str("break;");
            }

            Statement::Continue => {
                out.push_str("continue;");
            }
            
            Statement::Expr(e) => {
                self.emit_expr(&e.node, out)?;
                out.push(';');
            }
            
            Statement::Asm { template, .. } => {
                out.push_str("asm(\"");
                out.push_str(template);
                out.push_str("\");");
            }
        }
        
        Ok(())
    }
    
    fn emit_expr(&self, expr: &Expr, out: &mut String) -> Result<()> {
        match expr {
            Expr::Literal(lit) => self.emit_literal(lit, out),
            
            Expr::Ident(name) => {
                out.push_str(name);
                Ok(())
            }
            
            Expr::Array(elems) => {
                out.push('[');
                for (i, e) in elems.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    self.emit_expr(&e.node, out)?;
                }
                out.push(']');
                Ok(())
            }
            
            Expr::Object(fields) => {
                out.push('{');
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push('"');
                    out.push_str(k);
                    out.push_str("\": ");
                    self.emit_expr(&v.node, out)?;
                }
                out.push('}');
                Ok(())
            }
            
            Expr::BinOp { op, lhs, rhs } => {
                out.push('(');
                self.emit_expr(&lhs.node, out)?;
                out.push(' ');
                out.push_str(op.hlxl_str());
                out.push(' ');
                self.emit_expr(&rhs.node, out)?;
                out.push(')');
                Ok(())
            }
            
            Expr::UnaryOp { op, operand } => {
                out.push_str(op.hlxl_str());
                if matches!(op, UnaryOp::Not) {
                    out.push(' ');
                }
                self.emit_expr(&operand.node, out)?;
                Ok(())
            }
            
            Expr::Call { func, args } => {
                self.emit_expr(&func.node, out)?;
                out.push('(');
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    self.emit_expr(&arg.node, out)?;
                }
                out.push(')');
                Ok(())
            }
            
            Expr::Index { object, index } => {
                self.emit_expr(&object.node, out)?;
                out.push('[');
                self.emit_expr(&index.node, out)?;
                out.push(']');
                Ok(())
            }
            
            Expr::Field { object, field } => {
                self.emit_expr(&object.node, out)?;
                out.push('.');
                out.push_str(field);
                Ok(())
            }
            
            Expr::Cast { expr, target_type } => {
                out.push('(');
                self.emit_expr(&expr.node, out)?;
                out.push_str(" as ");
                out.push_str(&format!("{:?}", target_type));
                out.push(')');
                Ok(())
            }
            
            Expr::Pipe { value, func } => {
                self.emit_expr(&value.node, out)?;
                out.push_str(" |> ");
                self.emit_expr(&func.node, out)?;
                Ok(())
            }
            
            Expr::Collapse { table, namespace, value } => {
                out.push_str("ls.collapse ");
                out.push_str(table);
                out.push(' ');
                out.push_str(namespace);
                out.push(' ');
                self.emit_expr(&value.node, out)?;
                Ok(())
            }
            
            Expr::Resolve { target } => {
                out.push_str("ls.resolve ");
                self.emit_expr(&target.node, out)?;
                Ok(())
            }
            
            Expr::Snapshot => {
                out.push_str("ls.snapshot");
                Ok(())
            }
            
            Expr::Transaction { .. } => {
                out.push_str("ls.transaction { }");
                Ok(())
            }
            
            Expr::Handle(h) => {
                out.push_str(h);
                Ok(())
            }
            
            Expr::Contract { id, fields } => {
                out.push_str(&format!("@{} {{", id));
                for (i, (idx, val)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(&format!("@{}: ", idx));
                    self.emit_expr(&val.node, out)?;
                }
                out.push('}');
                Ok(())
            }
        }
    }
    
    fn emit_literal(&self, lit: &Literal, out: &mut String) -> Result<()> {
        match lit {
            Literal::Null => out.push_str("null"),
            Literal::Bool(true) => out.push_str("true"),
            Literal::Bool(false) => out.push_str("false"),
            Literal::Int(i) => out.push_str(&i.to_string()),
            Literal::Float(f) => {
                let s = f.to_string();
                out.push_str(&s);
                // Ensure there's a decimal point
                if !s.contains('.') {
                    out.push_str(".0");
                }
            }
            Literal::String(s) => {
                out.push('"');
                out.push_str(&s.escape_default().to_string());
                out.push('"');
            }
            Literal::Array(elems) => {
                out.push('[');
                for (i, elem) in elems.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    self.emit_expr(&elem.node, out)?;
                }
                out.push(']');
            }
            Literal::Object(fields) => {
                out.push('{');
                for (i, (key, val)) in fields.iter().enumerate() {
                    if i > 0 { out.push_str(", "); }
                    out.push_str(key);
                    out.push_str(": ");
                    self.emit_expr(&val.node, out)?;
                }
                out.push('}');
            }
        }
        Ok(())
    }
}

/// Convert runic glyphs to HLXL (basic transliteration)
pub fn transliterate_to_hlxl(source: &str) -> Result<String> {
    let mut result = source.to_string();
    
    // Structure
    result = result.replace(glyphs::PROGRAM, "program");
    result = result.replace(glyphs::BLOCK, "block");
    result = result.replace(glyphs::LET, "let");
    result = result.replace(glyphs::LOCAL, "local");
    result = result.replace(glyphs::RETURN, "return");
    result = result.replace(glyphs::IF, "if");
    result = result.replace(glyphs::ELSE, "else");
    result = result.replace(glyphs::WHILE, "while");
    result = result.replace(glyphs::FOR, "for");
    
    // Operators
    result = result.replace("⊕", "+");
    result = result.replace("⊖", "-");
    result = result.replace("⊗", "*");
    result = result.replace("⊘", "/");
    result = result.replace("⩵", "==");
    result = result.replace("≠", "!=");
    result = result.replace("≪", "<");
    result = result.replace("≤", "<=");
    result = result.replace("≫", ">");
    result = result.replace("≥", ">=");
    result = result.replace("∧", " and ");
    result = result.replace("∨", " or ");
    result = result.replace("¬", "not ");
    
    // LS operations
    result = result.replace(glyphs::COLLAPSE, "ls.collapse");
    result = result.replace(glyphs::RESOLVE, "ls.resolve");
    result = result.replace(glyphs::SNAPSHOT, "ls.snapshot");
    result = result.replace(glyphs::TRANSACTION, "ls.transaction");
    
    // Types
    result = result.replace(glyphs::TYPE_NULL, "null");
    result = result.replace(glyphs::TYPE_TRUE, "true");
    result = result.replace(glyphs::TYPE_FALSE, "false");
    result = result.replace(glyphs::TYPE_INT, "");
    result = result.replace(glyphs::TYPE_FLOAT, "");
    result = result.replace(glyphs::TYPE_STRING, "");
    
    // Brackets
    result = result.replace(glyphs::ARRAY_OPEN, "[");
    result = result.replace(glyphs::ARRAY_CLOSE, "]");
    result = result.replace(glyphs::OBJ_OPEN, "{");
    result = result.replace(glyphs::OBJ_CLOSE, "}");
    result = result.replace(glyphs::PAREN_OPEN, "(");
    result = result.replace(glyphs::PAREN_CLOSE, ")");
    
    // Pipe
    result = result.replace(glyphs::PIPE, "|>");
    
    // Handle prefix (⟁xxx → &h_xxx)
    let mut final_result = String::new();
    let mut chars = result.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '⟁' {
            final_result.push_str("&h_");
        } else {
            final_result.push(c);
        }
    }
    
    Ok(final_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::HlxaParser;
    
    #[test]
    fn test_hlxl_to_runic_roundtrip() {
        let hlxl_src = r#"
            program test {
                fn main() {
                    let x = 42;
                    let y = (x + 10);
                    return y;
                }
            }
        "#;
        
        // Parse HLXL
        let ast = HlxaParser::new().parse(hlxl_src).unwrap();
        
        // Emit as runic
        let runic = RunicEmitter::new().emit(&ast).unwrap();
        
        // Parse runic back (via transliteration)
        let ast2 = RunicParser::new().parse(&runic).unwrap();
        
        // Emit as HLXL again
        let hlxl2 = HlxlEmitter::new().emit(&ast2).unwrap();
        
        // Should preserve semantics (not exact text)
        assert!(hlxl2.contains("let x = 42"));
        assert!(hlxl2.contains("return"));
    }
    
    #[test]
    fn test_runic_transliteration() {
        let runic = "⊢ x = ⓘ42 ⊕ ⓘ10";
        let hlxl = transliterate_to_hlxl(runic).unwrap();
        assert_eq!(hlxl, "let x = 42 + 10");
    }
}
