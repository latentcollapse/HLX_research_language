//! Call Hierarchy Implementation
//!
//! Provides call hierarchy navigation for HLX functions, allowing users to:
//! - Find all callers of a function (incoming calls)
//! - Find all functions called by a function (outgoing calls)

use hlx_compiler::ast::*;
use hlx_compiler::HlxaParser;
use tower_lsp::lsp_types::*;
use dashmap::DashMap;

/// Represents a single call site in the code
#[derive(Debug, Clone)]
pub struct CallSite {
    /// The function making the call
    pub caller: String,
    /// The function being called
    pub callee: String,
    /// Location of the call in the source
    pub location: Location,
    /// Range of the call expression
    pub call_expr_range: Range,
}

/// Maintains an index of all function calls in the workspace
pub struct CallHierarchyIndex {
    /// Map from function name to locations where it's called (incoming calls)
    incoming_calls: DashMap<String, Vec<CallSite>>,
    /// Map from function name to functions it calls (outgoing calls)
    outgoing_calls: DashMap<String, Vec<CallSite>>,
    /// Map from function name to its definition location
    function_locations: DashMap<String, Location>,
}

impl CallHierarchyIndex {
    pub fn new() -> Self {
        Self {
            incoming_calls: DashMap::new(),
            outgoing_calls: DashMap::new(),
            function_locations: DashMap::new(),
        }
    }

    /// Index a document, extracting all function definitions and calls
    pub fn index_document(&self, uri: &Url, text: &str) {
        // Clear existing data for this document
        self.clear_document(uri);

        let parser = HlxaParser::new();
        let program = match parser.parse_diagnostics(text) {
            Ok(p) => p,
            Err(_) => return, // Skip documents with parse errors
        };

        // Build call graph
        let mut builder = CallGraphBuilder::new(uri.clone(), text);
        builder.visit_program(&program);

        // Store the collected data
        for (func_name, location) in builder.function_locations {
            self.function_locations.insert(func_name, location);
        }

        for call_site in builder.call_sites {
            // Add to incoming calls (who calls this function)
            self.incoming_calls
                .entry(call_site.callee.clone())
                .or_insert_with(Vec::new)
                .push(call_site.clone());

            // Add to outgoing calls (what this function calls)
            self.outgoing_calls
                .entry(call_site.caller.clone())
                .or_insert_with(Vec::new)
                .push(call_site);
        }
    }

    /// Clear all call data for a specific document
    fn clear_document(&self, uri: &Url) {
        // Remove all entries related to this URI
        // Note: This is a simplified implementation. A production version would
        // track which functions belong to which documents for precise cleanup.
        let uri_str = uri.to_string();

        // Clear function locations for this document
        self.function_locations.retain(|_, loc| loc.uri != *uri);

        // Clear incoming calls from this document
        for mut entry in self.incoming_calls.iter_mut() {
            entry.value_mut().retain(|cs| cs.location.uri != *uri);
        }

        // Clear outgoing calls from this document
        for mut entry in self.outgoing_calls.iter_mut() {
            entry.value_mut().retain(|cs| cs.location.uri != *uri);
        }
    }

    /// Prepare call hierarchy for a given position
    /// Returns the function at that position if found
    pub fn prepare(
        &self,
        uri: &Url,
        position: Position,
        text: &str,
    ) -> Option<CallHierarchyItem> {
        // Find which function the position is in
        let parser = HlxaParser::new();
        let program = parser.parse_diagnostics(text).ok()?;

        // Find the function at this position
        let function_name = self.find_function_at_position(&program, position)?;

        // Get the function's location
        let location = self.function_locations.get(&function_name)?;

        Some(CallHierarchyItem {
            name: function_name.clone(),
            kind: SymbolKind::FUNCTION,
            tags: None,
            detail: None,
            uri: location.uri.clone(),
            range: location.range,
            selection_range: location.range,
            data: None,
        })
    }

    /// Get all incoming calls for a function
    pub fn get_incoming_calls(&self, function: &str) -> Vec<CallHierarchyIncomingCall> {
        let incoming = match self.incoming_calls.get(function) {
            Some(calls) => calls,
            None => return Vec::new(),
        };

        let mut result = Vec::new();
        let mut seen_callers = std::collections::HashSet::new();

        for call_site in incoming.value() {
            // Deduplicate by caller
            if seen_callers.contains(&call_site.caller) {
                continue;
            }
            seen_callers.insert(call_site.caller.clone());

            // Get caller's location
            if let Some(caller_loc) = self.function_locations.get(&call_site.caller) {
                result.push(CallHierarchyIncomingCall {
                    from: CallHierarchyItem {
                        name: call_site.caller.clone(),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        detail: None,
                        uri: caller_loc.uri.clone(),
                        range: caller_loc.range,
                        selection_range: caller_loc.range,
                        data: None,
                    },
                    from_ranges: vec![call_site.call_expr_range],
                });
            }
        }

        result
    }

    /// Get all outgoing calls for a function
    pub fn get_outgoing_calls(&self, function: &str) -> Vec<CallHierarchyOutgoingCall> {
        let outgoing = match self.outgoing_calls.get(function) {
            Some(calls) => calls,
            None => return Vec::new(),
        };

        let mut result = Vec::new();
        let mut seen_callees = std::collections::HashSet::new();

        for call_site in outgoing.value() {
            // Deduplicate by callee
            if seen_callees.contains(&call_site.callee) {
                continue;
            }
            seen_callees.insert(call_site.callee.clone());

            // Get callee's location
            if let Some(callee_loc) = self.function_locations.get(&call_site.callee) {
                result.push(CallHierarchyOutgoingCall {
                    to: CallHierarchyItem {
                        name: call_site.callee.clone(),
                        kind: SymbolKind::FUNCTION,
                        tags: None,
                        detail: None,
                        uri: callee_loc.uri.clone(),
                        range: callee_loc.range,
                        selection_range: callee_loc.range,
                        data: None,
                    },
                    from_ranges: vec![call_site.call_expr_range],
                });
            }
        }

        result
    }

    /// Find the function name at a given position
    fn find_function_at_position(&self, program: &Program, position: Position) -> Option<String> {
        // Search through all blocks to find one containing the position
        for block in &program.blocks {
            if let Some(name_span) = &block.name_span {
                if self.position_in_span(position, *name_span) {
                    return Some(block.name.clone());
                }
            }
        }

        None
    }

    /// Check if a position is within a span
    fn position_in_span(&self, position: Position, span: Span) -> bool {
        let line = position.line;
        let char = position.character;

        (line > span.line || (line == span.line && char >= span.col))
            && (line < span.line || (line == span.line && char <= span.col))
    }
}

/// Builds a call graph by walking the AST
struct CallGraphBuilder {
    /// URI of the document being indexed
    uri: Url,
    /// Source text
    source: String,
    /// Current function being analyzed
    current_function: Option<String>,
    /// All discovered call sites
    call_sites: Vec<CallSite>,
    /// Function definition locations
    function_locations: Vec<(String, Location)>,
}

impl CallGraphBuilder {
    fn new(uri: Url, source: &str) -> Self {
        Self {
            uri,
            source: source.to_string(),
            current_function: None,
            call_sites: Vec::new(),
            function_locations: Vec::new(),
        }
    }

    fn visit_program(&mut self, program: &Program) {
        for block in &program.blocks {
            self.visit_block(block);
        }

        for module in &program.modules {
            self.visit_module(module);
        }
    }

    fn visit_module(&mut self, module: &Module) {
        for block in &module.blocks {
            self.visit_block(block);
        }
    }

    fn visit_block(&mut self, block: &Block) {
        // Record function definition
        if let Some(name_span) = &block.name_span {
            let range = self.span_to_range(*name_span);
            let location = Location {
                uri: self.uri.clone(),
                range,
            };
            self.function_locations.push((block.name.clone(), location));
        }

        // Set current function context
        let prev_function = self.current_function.clone();
        self.current_function = Some(block.name.clone());

        // Visit all items in the block
        for item in &block.items {
            match &item.node {
                Item::Statement(stmt) => self.visit_statement(stmt, item.span),
                Item::Node(node) => self.visit_node(node),
            }
        }

        // Restore previous function context
        self.current_function = prev_function;
    }

    fn visit_node(&mut self, node: &Node) {
        // Check if the operation is a function call
        for input in &node.inputs {
            self.visit_expr(&input.node, input.span);
        }
    }

    fn visit_statement(&mut self, stmt: &Statement, span: Span) {
        match stmt {
            Statement::Let { value, .. } => {
                self.visit_expr(&value.node, value.span);
            }
            Statement::Local { value, .. } => {
                self.visit_expr(&value.node, value.span);
            }
            Statement::Assign { lhs, value } => {
                self.visit_expr(&lhs.node, lhs.span);
                self.visit_expr(&value.node, value.span);
            }
            Statement::Return { value, .. } => {
                self.visit_expr(&value.node, value.span);
            }
            Statement::If { condition, then_branch, else_branch, .. } => {
                self.visit_expr(&condition.node, condition.span);
                for stmt in then_branch {
                    self.visit_statement(&stmt.node, stmt.span);
                }
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.visit_statement(&stmt.node, stmt.span);
                    }
                }
            }
            Statement::While { condition, body, .. } => {
                self.visit_expr(&condition.node, condition.span);
                for stmt in body {
                    self.visit_statement(&stmt.node, stmt.span);
                }
            }
            Statement::Expr(expr) => {
                self.visit_expr(&expr.node, expr.span);
            }
            Statement::Asm { inputs, .. } => {
                for (_, expr) in inputs {
                    self.visit_expr(&expr.node, expr.span);
                }
            }
            _ => {}
        }
    }

    fn visit_expr(&mut self, expr: &Expr, span: Span) {
        match expr {
            Expr::Call { func, args } => {
                // Check if this is a direct function call (e.g., foo(...))
                if let Expr::Ident(func_name) = &func.node {
                    if let Some(caller) = &self.current_function {
                        let range = self.span_to_range(span);
                        self.call_sites.push(CallSite {
                            caller: caller.clone(),
                            callee: func_name.clone(),
                            location: Location {
                                uri: self.uri.clone(),
                                range,
                            },
                            call_expr_range: range,
                        });
                    }
                }

                // Visit arguments
                for arg in args {
                    self.visit_expr(&arg.node, arg.span);
                }
            }
            Expr::BinOp { lhs, rhs, .. } => {
                self.visit_expr(&lhs.node, lhs.span);
                self.visit_expr(&rhs.node, rhs.span);
            }
            Expr::UnaryOp { operand, .. } => {
                self.visit_expr(&operand.node, operand.span);
            }
            Expr::Index { object, index } => {
                self.visit_expr(&object.node, object.span);
                self.visit_expr(&index.node, index.span);
            }
            Expr::Field { object, .. } => {
                self.visit_expr(&object.node, object.span);
            }
            Expr::Cast { expr, .. } => {
                self.visit_expr(&expr.node, expr.span);
            }
            Expr::Pipe { value, func } => {
                self.visit_expr(&value.node, value.span);
                self.visit_expr(&func.node, func.span);
            }
            Expr::Array(elements) => {
                for elem in elements {
                    self.visit_expr(&elem.node, elem.span);
                }
            }
            Expr::Object(fields) => {
                for (_, value) in fields {
                    self.visit_expr(&value.node, value.span);
                }
            }
            Expr::Contract { fields, .. } => {
                for (_, value) in fields {
                    self.visit_expr(&value.node, value.span);
                }
            }
            Expr::Transaction { body } => {
                for stmt in body {
                    self.visit_statement(&stmt.node, stmt.span);
                }
            }
            _ => {}
        }
    }

    fn span_to_range(&self, span: Span) -> Range {
        Range {
            start: Position {
                line: span.line,
                character: span.col,
            },
            end: Position {
                line: span.line,
                character: span.col + (span.end - span.start) as u32,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_call_hierarchy_index() {
        let index = CallHierarchyIndex::new();
        let uri = Url::parse("file:///test.hlxa").unwrap();

        let source = r#"
            program test {
                fn foo() {
                    return bar();
                }

                fn bar() {
                    return 42;
                }

                fn main() {
                    foo();
                    bar();
                    return 0;
                }
            }
        "#;

        index.index_document(&uri, source);

        // Check that functions are indexed
        assert!(index.function_locations.contains_key("foo"));
        assert!(index.function_locations.contains_key("bar"));
        assert!(index.function_locations.contains_key("main"));

        // Check incoming calls to bar (called by foo and main)
        let bar_incoming = index.get_incoming_calls("bar");
        assert_eq!(bar_incoming.len(), 2);

        // Check outgoing calls from main (calls foo and bar)
        let main_outgoing = index.get_outgoing_calls("main");
        assert_eq!(main_outgoing.len(), 2);
    }
}
