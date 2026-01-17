//! Folding Ranges Implementation
//!
//! Provides code folding capabilities for HLX source files, allowing users to
//! collapse and expand regions of code in their editor.

use hlx_compiler::ast::*;
use hlx_compiler::HlxaParser;
use tower_lsp::lsp_types::{FoldingRange, FoldingRangeKind};

/// Provider for folding range analysis
pub struct FoldingRangesProvider;

impl FoldingRangesProvider {
    pub fn new() -> Self {
        Self
    }

    /// Provide folding ranges for a document
    pub fn provide_folding_ranges(&self, source: &str) -> Option<Vec<FoldingRange>> {
        let parser = HlxaParser::new();
        let program = match parser.parse_diagnostics(source) {
            Ok(p) => p,
            Err(_) => {
                // If parsing fails, try to provide basic folding based on braces
                return self.fallback_folding(source);
            }
        };

        let mut collector = FoldingRangeCollector::new(source);
        collector.visit_program(&program);

        Some(collector.ranges)
    }

    /// Fallback folding when AST is unavailable (uses brace matching)
    fn fallback_folding(&self, source: &str) -> Option<Vec<FoldingRange>> {
        let mut ranges = Vec::new();
        let mut brace_stack: Vec<(usize, u32)> = Vec::new(); // (char_pos, line)

        for (line_num, line) in source.lines().enumerate() {
            for (char_pos, ch) in line.chars().enumerate() {
                match ch {
                    '{' => {
                        brace_stack.push((char_pos, line_num as u32));
                    }
                    '}' => {
                        if let Some((_, start_line)) = brace_stack.pop() {
                            let end_line = line_num as u32;
                            if end_line > start_line {
                                ranges.push(FoldingRange {
                                    start_line,
                                    start_character: None,
                                    end_line,
                                    end_character: None,
                                    kind: Some(FoldingRangeKind::Region),
                                    collapsed_text: None,
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        Some(ranges)
    }
}

/// Collects folding ranges from an AST
struct FoldingRangeCollector {
    ranges: Vec<FoldingRange>,
    source: String,
}

impl FoldingRangeCollector {
    fn new(source: &str) -> Self {
        Self {
            ranges: Vec::new(),
            source: source.to_string(),
        }
    }

    fn visit_program(&mut self, program: &Program) {
        // Fold imports section if multiple imports
        if program.imports.len() > 1 {
            if let (Some(first), Some(last)) = (program.imports.first(), program.imports.last()) {
                self.add_range(
                    first.span,
                    last.span,
                    FoldingRangeKind::Imports,
                    Some("imports...".to_string()),
                );
            }
        }

        // Fold each module
        for module in &program.modules {
            self.visit_module(module);
        }

        // Fold each top-level block
        for block in &program.blocks {
            self.visit_block(block);
        }
    }

    fn visit_module(&mut self, module: &Module) {
        // Fold the entire module body
        // Note: We'd need span information on the module itself for this
        // For now, fold individual blocks within the module

        for block in &module.blocks {
            self.visit_block(block);
        }
    }

    fn visit_block(&mut self, block: &Block) {
        // Fold the entire function body
        if !block.items.is_empty() {
            if let (Some(first), Some(last)) = (block.items.first(), block.items.last()) {
                self.add_range(
                    first.span,
                    last.span,
                    FoldingRangeKind::Region,
                    Some(format!("fn {}...", block.name)),
                );
            }
        }

        // Visit statements inside for nested folding
        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => self.visit_statement(stmt, item.span),
                Item::Node(_) => {}
            }
        }
    }

    fn visit_statement(&mut self, stmt: &Statement, span: Span) {
        match stmt {
            Statement::If {
                then_branch,
                else_branch,
                ..
            } => {
                // Fold then branch
                if !then_branch.is_empty() {
                    if let (Some(first), Some(last)) = (then_branch.first(), then_branch.last()) {
                        self.add_range(
                            first.span,
                            last.span,
                            FoldingRangeKind::Region,
                            Some("if...".to_string()),
                        );

                        // Visit nested statements
                        for stmt in then_branch {
                            self.visit_statement(&stmt.node, stmt.span);
                        }
                    }
                }

                // Fold else branch
                if let Some(else_stmts) = else_branch {
                    if !else_stmts.is_empty() {
                        if let (Some(first), Some(last)) = (else_stmts.first(), else_stmts.last())
                        {
                            self.add_range(
                                first.span,
                                last.span,
                                FoldingRangeKind::Region,
                                Some("else...".to_string()),
                            );

                            // Visit nested statements
                            for stmt in else_stmts {
                                self.visit_statement(&stmt.node, stmt.span);
                            }
                        }
                    }
                }
            }
            Statement::While { body, .. } => {
                // Fold loop body
                if !body.is_empty() {
                    if let (Some(first), Some(last)) = (body.first(), body.last()) {
                        self.add_range(
                            first.span,
                            last.span,
                            FoldingRangeKind::Region,
                            Some("loop...".to_string()),
                        );

                        // Visit nested statements
                        for stmt in body {
                            self.visit_statement(&stmt.node, stmt.span);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    fn add_range(
        &mut self,
        start_span: Span,
        end_span: Span,
        kind: FoldingRangeKind,
        collapsed_text: Option<String>,
    ) {
        // Only add if the range spans multiple lines
        if end_span.line > start_span.line {
            self.ranges.push(FoldingRange {
                start_line: start_span.line,
                start_character: Some(start_span.col),
                end_line: end_span.line,
                end_character: Some(end_span.col + (end_span.end - end_span.start) as u32),
                kind: Some(kind),
                collapsed_text,
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folding_ranges_simple() {
        let provider = FoldingRangesProvider::new();

        let source = r#"
            program test {
                fn main() {
                    let x = 1;
                    if (x > 0) {
                        print(x);
                    }
                    return x;
                }
            }
        "#;

        let ranges = provider.provide_folding_ranges(source);
        assert!(ranges.is_some());

        let ranges = ranges.unwrap();
        // Should have folding for function body and if statement
        assert!(ranges.len() >= 1);
    }

    #[test]
    fn test_fallback_folding() {
        let provider = FoldingRangesProvider::new();

        // Malformed code that won't parse
        let source = r#"
            fn foo {
                let x = 1;
            }
        "#;

        let ranges = provider.provide_folding_ranges(source);
        assert!(ranges.is_some());

        // Fallback folding should still find the braces
        let ranges = ranges.unwrap();
        assert!(!ranges.is_empty());
    }

    #[test]
    fn test_nested_folding() {
        let provider = FoldingRangesProvider::new();

        let source = r#"
            program test {
                fn main() {
                    let x = 10;
                    loop (x > 0, 100) {
                        if (x % 2 == 0) {
                            print(x);
                        }
                        x = x - 1;
                    }
                    return 0;
                }
            }
        "#;

        let ranges = provider.provide_folding_ranges(source);
        assert!(ranges.is_some());

        let ranges = ranges.unwrap();
        // Should have folding for function, loop, and if statement
        assert!(ranges.len() >= 2);
    }

    #[test]
    fn test_multiple_functions() {
        let provider = FoldingRangesProvider::new();

        let source = r#"
            program test {
                fn foo() {
                    let x = 1;
                    return x;
                }

                fn bar() {
                    let y = 2;
                    return y;
                }

                fn main() {
                    let a = foo();
                    let b = bar();
                    return a + b;
                }
            }
        "#;

        let ranges = provider.provide_folding_ranges(source);
        assert!(ranges.is_some());

        let ranges = ranges.unwrap();
        // Should have folding for function bodies (at least 1)
        assert!(ranges.len() >= 1);
    }
}
