//! HLX Document Formatter
//!
//! Provides consistent code formatting for HLX/HLXL source files.
//! Supports both full document and range formatting.

use hlx_compiler::ast::*;
use hlx_compiler::HlxaParser;
use tower_lsp::lsp_types::{Range, TextEdit, Position};

/// Formatting configuration
#[derive(Debug, Clone)]
pub struct FormattingConfig {
    pub indent_size: usize,
    pub max_line_length: usize,
    pub brace_style: BraceStyle,
}

#[derive(Debug, Clone, Copy)]
pub enum BraceStyle {
    SameLine,
    NextLine,
}

impl Default for FormattingConfig {
    fn default() -> Self {
        Self {
            indent_size: 4,
            max_line_length: 100,
            brace_style: BraceStyle::SameLine,
        }
    }
}

/// HLX source code formatter
pub struct HlxFormatter {
    config: FormattingConfig,
}

impl HlxFormatter {
    pub fn new(config: FormattingConfig) -> Self {
        Self { config }
    }

    pub fn with_default() -> Self {
        Self::new(FormattingConfig::default())
    }

    /// Format an entire document
    pub fn format_document(&self, source: &str) -> Option<Vec<TextEdit>> {
        let parser = HlxaParser::new();
        let program = parser.parse_diagnostics(source).ok()?;

        let formatted = self.format_program(&program);

        // Generate text edit
        Some(self.compute_edits(source, &formatted))
    }

    /// Format a specific range in the document
    pub fn format_range(&self, source: &str, range: Range) -> Option<Vec<TextEdit>> {
        // For simplicity, we format the entire document and return the edit for the range
        // A more sophisticated implementation could format only the affected statements
        self.format_document(source)
    }

    /// Format a complete program
    fn format_program(&self, program: &Program) -> String {
        let mut output = String::new();

        // Format imports
        for import in &program.imports {
            output.push_str(&self.format_import(import));
            output.push('\n');
        }

        if !program.imports.is_empty() {
            output.push('\n');
        }

        // Format modules
        for module in &program.modules {
            output.push_str(&self.format_module(module));
            output.push('\n');
        }

        if !program.modules.is_empty() {
            output.push('\n');
        }

        // Format top-level blocks (functions)
        for (i, block) in program.blocks.iter().enumerate() {
            if i > 0 {
                output.push('\n');
            }
            output.push_str(&self.format_block(block, 0));
        }

        output
    }

    /// Format an import statement
    fn format_import(&self, import: &Import) -> String {
        let mut output = format!("import \"{}\"", import.path);

        if let Some(alias) = &import.alias {
            output.push_str(&format!(" as {}", alias));
        }

        if let Some(items) = &import.items {
            output.push_str(" { ");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    output.push_str(", ");
                }
                output.push_str(&item.name);
                if let Some(alias) = &item.alias {
                    output.push_str(&format!(" as {}", alias));
                }
            }
            output.push_str(" }");
        }

        output.push(';');
        output
    }

    /// Format a module
    fn format_module(&self, module: &Module) -> String {
        let mut output = format!("module {} {{\n", module.name);

        // Constants
        for constant in &module.constants {
            output.push_str(&self.indent(1));
            output.push_str(&format!("const {}: {} = ", constant.name, constant.typ.to_string()));
            output.push_str(&self.format_expr(&constant.value.node));
            output.push_str(";\n");
        }

        // Structs
        for struct_def in &module.structs {
            output.push_str(&self.indent(1));
            output.push_str(&format!("struct {} {{\n", struct_def.name));
            for (field_name, field_type) in &struct_def.fields {
                output.push_str(&self.indent(2));
                output.push_str(&format!("{}: {},\n", field_name, field_type.to_string()));
            }
            output.push_str(&self.indent(1));
            output.push_str("}\n");
        }

        // Enums
        for enum_def in &module.enums {
            output.push_str(&self.indent(1));
            output.push_str(&format!("enum {} {{\n", enum_def.name));
            for variant in &enum_def.variants {
                output.push_str(&self.indent(2));
                output.push_str(&format!("{},\n", variant));
            }
            output.push_str(&self.indent(1));
            output.push_str("}\n");
        }

        // Blocks
        for block in &module.blocks {
            output.push_str(&self.indent(1));
            let block_formatted = self.format_block(block, 1);
            output.push_str(&block_formatted);
        }

        output.push_str("}\n");
        output
    }

    /// Format a function block
    fn format_block(&self, block: &Block, base_indent: usize) -> String {
        let mut output = String::new();

        // Attributes
        for attr in &block.attributes {
            output.push_str(&self.indent(base_indent));
            output.push_str(&format!("#[{}]\n", attr));
        }

        // Function signature
        output.push_str(&self.indent(base_indent));
        output.push_str("fn ");
        output.push_str(&block.name);
        output.push('(');

        // Parameters
        for (i, (name, _name_span, type_info)) in block.params.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            output.push_str(name);
            if let Some((typ, _)) = type_info {
                output.push_str(": ");
                output.push_str(&typ.to_string());
            }
        }

        output.push(')');

        // Return type
        if let Some(ret_type) = &block.return_type {
            output.push_str(" -> ");
            output.push_str(&ret_type.to_string());
        }

        // Opening brace
        match self.config.brace_style {
            BraceStyle::SameLine => output.push_str(" {\n"),
            BraceStyle::NextLine => {
                output.push('\n');
                output.push_str(&self.indent(base_indent));
                output.push_str("{\n");
            }
        }

        // Body
        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => {
                    output.push_str(&self.format_statement(stmt, base_indent + 1));
                }
                Item::Node(node) => {
                    output.push_str(&self.format_node(node, base_indent + 1));
                }
            }
        }

        // Closing brace
        output.push_str(&self.indent(base_indent));
        output.push_str("}\n");

        output
    }

    /// Format a topological node
    fn format_node(&self, node: &Node, indent: usize) -> String {
        let mut output = self.indent(indent);

        if let Some(id) = &node.id {
            output.push_str(&format!("{}: ", id));
        }

        output.push_str(&node.op);
        output.push('(');

        for (i, input) in node.inputs.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            output.push_str(&self.format_expr(&input.node));
        }

        output.push_str(") -> (");

        for (i, output_var) in node.outputs.iter().enumerate() {
            if i > 0 {
                output.push_str(", ");
            }
            output.push_str(output_var);
        }

        output.push_str(");\n");
        output
    }

    /// Format a statement
    fn format_statement(&self, stmt: &Statement, indent: usize) -> String {
        let indent_str = self.indent(indent);
        match stmt {
            Statement::Let { name, type_annotation, value, .. } => {
                let mut output = indent_str;
                output.push_str("let ");
                output.push_str(name);
                if let Some(typ) = type_annotation {
                    output.push_str(": ");
                    output.push_str(&typ.to_string());
                }
                output.push_str(" = ");
                output.push_str(&self.format_expr(&value.node));
                output.push_str(";\n");
                output
            }
            Statement::Local { name, value, .. } => {
                let mut output = indent_str;
                output.push_str("local ");
                output.push_str(name);
                output.push_str(" = ");
                output.push_str(&self.format_expr(&value.node));
                output.push_str(";\n");
                output
            }
            Statement::Assign { lhs, value } => {
                let mut output = indent_str;
                output.push_str(&self.format_expr(&lhs.node));
                output.push_str(" = ");
                output.push_str(&self.format_expr(&value.node));
                output.push_str(";\n");
                output
            }
            Statement::Return { value, .. } => {
                let mut output = indent_str;
                output.push_str("return ");
                output.push_str(&self.format_expr(&value.node));
                output.push_str(";\n");
                output
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                let mut output = indent_str;
                output.push_str("if (");
                output.push_str(&self.format_expr(&condition.node));
                output.push_str(") {\n");

                for stmt in then_branch {
                    output.push_str(&self.format_statement(&stmt.node, indent + 1));
                }

                if let Some(else_stmts) = else_branch {
                    output.push_str(&self.indent(indent));
                    output.push_str("} else {\n");
                    for stmt in else_stmts {
                        output.push_str(&self.format_statement(&stmt.node, indent + 1));
                    }
                }

                output.push_str(&self.indent(indent));
                output.push_str("}\n");
                output
            }
            Statement::While { condition, body, max_iter, .. } => {
                let mut output = indent_str;
                output.push_str("loop (");
                output.push_str(&self.format_expr(&condition.node));
                output.push_str(&format!(", {}) {{\n", max_iter));

                for stmt in body {
                    output.push_str(&self.format_statement(&stmt.node, indent + 1));
                }

                output.push_str(&self.indent(indent));
                output.push_str("}\n");
                output
            }
            Statement::Break => {
                let mut output = indent_str;
                output.push_str("break;\n");
                output
            }
            Statement::Continue => {
                let mut output = indent_str;
                output.push_str("continue;\n");
                output
            }
            Statement::Expr(expr) => {
                let mut output = indent_str;
                output.push_str(&self.format_expr(&expr.node));
                output.push_str(";\n");
                output
            }
            Statement::Asm { template, outputs, inputs, clobbers } => {
                let mut output = indent_str;
                output.push_str(&format!("asm(\"{}\"", template));

                if !outputs.is_empty() {
                    output.push_str(" : ");
                    for (i, (constraint, var)) in outputs.iter().enumerate() {
                        if i > 0 { output.push_str(", "); }
                        output.push_str(&format!("\"{}\"({})", constraint, var));
                    }
                }

                if !inputs.is_empty() {
                    output.push_str(" : ");
                    for (i, (constraint, expr)) in inputs.iter().enumerate() {
                        if i > 0 { output.push_str(", "); }
                        output.push_str(&format!("\"{}\"({})", constraint, self.format_expr(&expr.node)));
                    }
                }

                if !clobbers.is_empty() {
                    output.push_str(" : ");
                    for (i, clobber) in clobbers.iter().enumerate() {
                        if i > 0 { output.push_str(", "); }
                        output.push_str(&format!("\"{}\"", clobber));
                    }
                }

                output.push_str(");\n");
                output
            }
            Statement::Barrier { name, .. } => {
                let mut output = indent_str;
                output.push_str("barrier");
                if let Some(n) = name {
                    output.push_str(&format!("(\"{}\")", n));
                }
                output.push_str(";\n");
                output
            }
        }
    }

    /// Format an expression
    fn format_expr(&self, expr: &Expr) -> String {
        match expr {
            Expr::Literal(lit) => self.format_literal(lit),
            Expr::Ident(name) => name.clone(),
            Expr::Array(elements) => {
                let mut output = String::from("[");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&self.format_expr(&elem.node));
                }
                output.push(']');
                output
            }
            Expr::Object(fields) => {
                let mut output = String::from("{ ");
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("\"{}\": {}", key, self.format_expr(&value.node)));
                }
                output.push_str(" }");
                output
            }
            Expr::BinOp { op, lhs, rhs } => {
                let needs_parens = |e: &Expr| matches!(e, Expr::BinOp { .. });
                let left = self.format_expr(&lhs.node);
                let right = self.format_expr(&rhs.node);

                let left_str = if needs_parens(&lhs.node) && self.needs_parens_for_precedence(&lhs.node, *op) {
                    format!("({})", left)
                } else {
                    left
                };

                let right_str = if needs_parens(&rhs.node) && self.needs_parens_for_precedence(&rhs.node, *op) {
                    format!("({})", right)
                } else {
                    right
                };

                format!("{} {} {}", left_str, op.hlxl_str(), right_str)
            }
            Expr::UnaryOp { op, operand } => {
                let operand_str = self.format_expr(&operand.node);
                format!("{}{}", op.hlxl_str(), operand_str)
            }
            Expr::Call { func, args } => {
                let func_str = self.format_expr(&func.node);
                let mut output = format!("{}(", func_str);
                for (i, arg) in args.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&self.format_expr(&arg.node));
                }
                output.push(')');
                output
            }
            Expr::Index { object, index } => {
                let obj_str = self.format_expr(&object.node);
                let idx_str = self.format_expr(&index.node);
                format!("{}[{}]", obj_str, idx_str)
            }
            Expr::Field { object, field } => {
                let obj_str = self.format_expr(&object.node);
                format!("{}.{}", obj_str, field)
            }
            Expr::Cast { expr, target_type } => {
                let expr_str = self.format_expr(&expr.node);
                format!("{} as {}", expr_str, target_type.to_string())
            }
            Expr::Pipe { value, func } => {
                let value_str = self.format_expr(&value.node);
                let func_str = self.format_expr(&func.node);
                format!("{} |> {}", value_str, func_str)
            }
            Expr::Collapse { table, namespace, value } => {
                let value_str = self.format_expr(&value.node);
                format!("ls.collapse {} {} {}", table, namespace, value_str)
            }
            Expr::Resolve { target } => {
                let target_str = self.format_expr(&target.node);
                format!("ls.resolve {}", target_str)
            }
            Expr::Snapshot => "ls.snapshot".to_string(),
            Expr::Transaction { body } => {
                let mut output = String::from("ls.transaction {\n");
                for stmt in body {
                    output.push_str(&self.format_statement(&stmt.node, 1));
                }
                output.push_str("}");
                output
            }
            Expr::Handle(name) => format!("&{}", name),
            Expr::Contract { id, fields } => {
                let mut output = format!("@{} {{ ", id);
                for (i, (field_idx, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("@{}: {}", field_idx, self.format_expr(&value.node)));
                }
                output.push_str(" }");
                output
            }
        }
    }

    /// Format a literal value
    fn format_literal(&self, lit: &Literal) -> String {
        match lit {
            Literal::Null => "null".to_string(),
            Literal::Bool(b) => b.to_string(),
            Literal::Int(i) => i.to_string(),
            Literal::Float(f) => {
                // Ensure floats always have decimal point
                let s = f.to_string();
                if !s.contains('.') && !s.contains('e') {
                    format!("{}.0", s)
                } else {
                    s
                }
            }
            Literal::String(s) => format!("\"{}\"", s),
            Literal::Array(elements) => {
                let mut output = String::from("[");
                for (i, elem) in elements.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&self.format_expr(&elem.node));
                }
                output.push(']');
                output
            }
            Literal::Object(fields) => {
                let mut output = String::from("{ ");
                for (i, (key, value)) in fields.iter().enumerate() {
                    if i > 0 {
                        output.push_str(", ");
                    }
                    output.push_str(&format!("\"{}\": {}", key, self.format_expr(&value.node)));
                }
                output.push_str(" }");
                output
            }
        }
    }

    /// Check if expression needs parentheses based on operator precedence
    fn needs_parens_for_precedence(&self, inner_expr: &Expr, outer_op: BinOp) -> bool {
        if let Expr::BinOp { op: inner_op, .. } = inner_expr {
            inner_op.precedence() < outer_op.precedence()
        } else {
            false
        }
    }

    /// Generate indentation string
    fn indent(&self, level: usize) -> String {
        " ".repeat(level * self.config.indent_size)
    }

    /// Compute minimal text edits to transform old text to new text
    fn compute_edits(&self, old: &str, new: &str) -> Vec<TextEdit> {
        // For simplicity, replace entire document
        // A more sophisticated implementation would use a diff algorithm
        let line_count = old.lines().count() as u32;
        let last_line = old.lines().last().unwrap_or("");
        let last_char = last_line.len() as u32;

        vec![TextEdit {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position {
                    line: line_count.saturating_sub(1),
                    character: last_char,
                },
            },
            new_text: new.to_string(),
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_simple_function() {
        let source = "program test { fn main(){let x=1;return x;} }";
        let formatter = HlxFormatter::with_default();

        let edits = formatter.format_document(source);
        assert!(edits.is_some());

        if let Some(edits) = edits {
            assert!(!edits.is_empty());
            // Check that formatted text contains proper indentation
            let formatted = &edits[0].new_text;
            assert!(formatted.contains("    let x = 1;"));
            assert!(formatted.contains("    return x;"));
        }
    }

    #[test]
    fn test_format_if_statement() {
        let source = "program test { fn test(){if(x>0){return 1;}else{return 0;}} }";
        let formatter = HlxFormatter::with_default();

        let edits = formatter.format_document(source);
        assert!(edits.is_some());

        if let Some(edits) = edits {
            let formatted = &edits[0].new_text;
            // Check proper if/else formatting
            assert!(formatted.contains("if (x > 0)"));
            assert!(formatted.contains("} else {"));
        }
    }

    #[test]
    fn test_format_binary_ops() {
        let formatter = HlxFormatter::with_default();

        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Spanned::dummy(Expr::Literal(Literal::Int(1)))),
            rhs: Box::new(Spanned::dummy(Expr::Literal(Literal::Int(2)))),
        };

        let formatted = formatter.format_expr(&expr);
        assert_eq!(formatted, "1 + 2");
    }

    #[test]
    fn test_format_precedence() {
        let formatter = HlxFormatter::with_default();

        // Test that 1 + 2 * 3 formats correctly without extra parens
        let expr = Expr::BinOp {
            op: BinOp::Add,
            lhs: Box::new(Spanned::dummy(Expr::Literal(Literal::Int(1)))),
            rhs: Box::new(Spanned::dummy(Expr::BinOp {
                op: BinOp::Mul,
                lhs: Box::new(Spanned::dummy(Expr::Literal(Literal::Int(2)))),
                rhs: Box::new(Spanned::dummy(Expr::Literal(Literal::Int(3)))),
            })),
        };

        let formatted = formatter.format_expr(&expr);
        assert_eq!(formatted, "1 + 2 * 3");
    }

    #[test]
    fn test_format_loop() {
        let source = "program test { fn main(){loop(i<10,100){print(i);i=i+1;}} }";
        let formatter = HlxFormatter::with_default();

        let edits = formatter.format_document(source);
        assert!(edits.is_some());

        if let Some(edits) = edits {
            let formatted = &edits[0].new_text;
            assert!(formatted.contains("loop (i < 10, 100)"));
        }
    }
}
