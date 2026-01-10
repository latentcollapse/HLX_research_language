//! Refactoring Tools
//!
//! Provides intelligent code refactoring capabilities:
//! - Rename Symbol (F2)
//! - Extract Function
//! - Inline Variable
//! - Convert to Contract

use tower_lsp::lsp_types::*;
use std::collections::HashMap;
use crate::symbol_index::SymbolIndex;

/// Refactoring engine
pub struct RefactoringEngine {
    symbol_index: std::sync::Arc<SymbolIndex>,
}

impl RefactoringEngine {
    pub fn new(symbol_index: std::sync::Arc<SymbolIndex>) -> Self {
        Self { symbol_index }
    }

    /// Generate rename edits for a symbol
    pub fn rename_symbol(
        &self,
        uri: &Url,
        position: &Position,
        new_name: &str,
        text: &str,
    ) -> Option<WorkspaceEdit> {
        // Find all references to the symbol at this position
        let references = self.symbol_index.find_references(position, uri, text);

        if references.is_empty() {
            return None;
        }

        // Group edits by document
        let mut changes: HashMap<Url, Vec<TextEdit>> = HashMap::new();

        for reference in references {
            let edit = TextEdit {
                range: reference.range,
                new_text: new_name.to_string(),
            };

            changes.entry(reference.uri.clone())
                .or_insert_with(Vec::new)
                .push(edit);
        }

        Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        })
    }

    /// Extract selected code into a function
    pub fn extract_function(
        &self,
        _uri: &Url,
        range: &Range,
        function_name: &str,
        text: &str,
    ) -> Option<Vec<TextEdit>> {
        // Get the selected text
        let selected_text = self.get_text_in_range(text, range)?;

        // Analyze variables used in selection
        let (inputs, outputs) = self.analyze_data_flow(&selected_text, text, range);

        // Generate function signature
        let params = inputs.join(", ");
        let return_type = if outputs.is_empty() {
            String::new()
        } else if outputs.len() == 1 {
            outputs[0].clone()
        } else {
            format!("[{}]", outputs.join(", "))
        };

        // Build function
        let function_def = if return_type.is_empty() {
            format!("fn {}({}) {{\n{}\n}}\n\n", function_name, params, selected_text.trim())
        } else {
            format!("fn {}({}) {{\n{}\n    return {};\n}}\n\n",
                function_name, params, selected_text.trim(), return_type)
        };

        // Build function call
        let call_args = inputs.join(", ");
        let function_call = if outputs.is_empty() {
            format!("{}({});", function_name, call_args)
        } else if outputs.len() == 1 {
            format!("let {} = {}({});", outputs[0], function_name, call_args)
        } else {
            format!("let [{}] = {}({});", outputs.join(", "), function_name, call_args)
        };

        // Create edits
        let mut edits = Vec::new();

        // Insert function definition before current function
        let insertion_point = self.find_function_insertion_point(text, range)?;
        edits.push(TextEdit {
            range: Range {
                start: insertion_point,
                end: insertion_point,
            },
            new_text: function_def,
        });

        // Replace selection with function call
        edits.push(TextEdit {
            range: *range,
            new_text: function_call,
        });

        Some(edits)
    }

    /// Inline a variable (replace all uses with its value)
    pub fn inline_variable(
        &self,
        uri: &Url,
        position: &Position,
        text: &str,
    ) -> Option<Vec<TextEdit>> {
        // Find the variable definition
        let definition = self.symbol_index.find_definition(position, uri, text)?;

        // Get variable name and value
        let (_var_name, var_value) = self.parse_variable_definition(text, &definition.range)?;

        // Find all references (excluding definition)
        let mut references = self.symbol_index.find_references(position, uri, text);
        references.retain(|r| r.range != definition.range);

        // Create edits
        let mut edits = Vec::new();

        // Replace all references with the value
        for reference in references {
            edits.push(TextEdit {
                range: reference.range,
                new_text: var_value.clone(),
            });
        }

        // Remove the variable definition
        edits.push(TextEdit {
            range: definition.range,
            new_text: String::new(),
        });

        Some(edits)
    }

    /// Convert manual operator to contract
    pub fn convert_to_contract(
        &self,
        range: &Range,
        text: &str,
    ) -> Option<TextEdit> {
        let expression = self.get_text_in_range(text, range)?;
        let trimmed = expression.trim();

        // Detect operator and operands
        if let Some(converted) = self.convert_binary_op(trimmed) {
            return Some(TextEdit {
                range: *range,
                new_text: converted,
            });
        }

        None
    }

    /// Convert binary operation to contract
    fn convert_binary_op(&self, expr: &str) -> Option<String> {
        // Pattern: a + b
        if let Some(plus_pos) = expr.find(" + ") {
            let lhs = expr[..plus_pos].trim();
            let rhs = expr[plus_pos + 3..].trim();
            return Some(format!("@200 {{ lhs: {}, rhs: {} }}", lhs, rhs));
        }

        // Pattern: a - b
        if let Some(minus_pos) = expr.find(" - ") {
            if !expr.contains("->") {  // Not arrow operator
                let lhs = expr[..minus_pos].trim();
                let rhs = expr[minus_pos + 3..].trim();
                return Some(format!("@201 {{ lhs: {}, rhs: {} }}", lhs, rhs));
            }
        }

        // Pattern: a * b
        if let Some(mult_pos) = expr.find(" * ") {
            let lhs = expr[..mult_pos].trim();
            let rhs = expr[mult_pos + 3..].trim();
            return Some(format!("@202 {{ lhs: {}, rhs: {} }}", lhs, rhs));
        }

        // Pattern: a / b
        if let Some(div_pos) = expr.find(" / ") {
            let lhs = expr[..div_pos].trim();
            let rhs = expr[div_pos + 3..].trim();
            return Some(format!("@203 {{ lhs: {}, rhs: {} }}", lhs, rhs));
        }

        None
    }

    /// Get text within a range
    fn get_text_in_range(&self, text: &str, range: &Range) -> Option<String> {
        let lines: Vec<&str> = text.lines().collect();

        if range.start.line == range.end.line {
            // Single line
            let line = lines.get(range.start.line as usize)?;
            let start = range.start.character as usize;
            let end = range.end.character as usize;
            Some(line.get(start..end)?.to_string())
        } else {
            // Multi-line
            let mut result = String::new();

            for line_idx in range.start.line..=range.end.line {
                let line = lines.get(line_idx as usize)?;

                if line_idx == range.start.line {
                    result.push_str(&line[range.start.character as usize..]);
                } else if line_idx == range.end.line {
                    result.push_str(&line[..range.end.character as usize]);
                } else {
                    result.push_str(line);
                }

                if line_idx < range.end.line {
                    result.push('\n');
                }
            }

            Some(result)
        }
    }

    /// Analyze data flow to determine function inputs/outputs
    fn analyze_data_flow(
        &self,
        selected: &str,
        _full_text: &str,
        _range: &Range,
    ) -> (Vec<String>, Vec<String>) {
        let mut inputs = Vec::new();
        let mut outputs = Vec::new();

        // Simple heuristic: find identifiers in selection
        let mut words = std::collections::HashSet::new();
        for word in selected.split(|c: char| !c.is_alphanumeric() && c != '_') {
            if !word.is_empty() && word.chars().next().unwrap().is_alphabetic() {
                words.insert(word.to_string());
            }
        }

        // Variables used are inputs
        inputs.extend(words);

        // If selection has return statement, extract output
        if selected.contains("return ") {
            if let Some(return_pos) = selected.find("return ") {
                let after_return = &selected[return_pos + 7..];
                if let Some(semi_pos) = after_return.find(';') {
                    let return_expr = after_return[..semi_pos].trim();
                    outputs.push(return_expr.to_string());
                }
            }
        }

        (inputs, outputs)
    }

    /// Find where to insert the new function
    fn find_function_insertion_point(&self, text: &str, range: &Range) -> Option<Position> {
        // Find the start of the current function
        for (line_idx, line) in text.lines().enumerate() {
            if line_idx > range.start.line as usize {
                break;
            }

            if line.trim().starts_with("fn ") {
                return Some(Position {
                    line: line_idx as u32,
                    character: 0,
                });
            }
        }

        // Default: insert at start of file
        Some(Position { line: 0, character: 0 })
    }

    /// Parse variable definition to get name and value
    fn parse_variable_definition(&self, text: &str, range: &Range) -> Option<(String, String)> {
        let line = text.lines().nth(range.start.line as usize)?;

        // Pattern: let VAR = VALUE;
        if !line.trim().starts_with("let ") {
            return None;
        }

        let after_let = line.trim()[4..].trim();
        let eq_pos = after_let.find('=')?;

        let var_name = after_let[..eq_pos].trim().to_string();
        let value_part = after_let[eq_pos + 1..].trim();

        // Remove trailing semicolon
        let var_value = if value_part.ends_with(';') {
            value_part[..value_part.len() - 1].trim().to_string()
        } else {
            value_part.to_string()
        };

        Some((var_name, var_value))
    }

    /// Create code action for refactoring
    pub fn create_refactor_action(
        &self,
        title: String,
        uri: Url,
        edits: Vec<TextEdit>,
    ) -> CodeAction {
        let mut changes = HashMap::new();
        changes.insert(uri, edits);

        CodeAction {
            title,
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_binary_op() {
        let engine = RefactoringEngine::new(std::sync::Arc::new(crate::symbol_index::SymbolIndex::new()));

        let result = engine.convert_binary_op("a + b");
        assert_eq!(result, Some("@200 { lhs: a, rhs: b }".to_string()));

        let result = engine.convert_binary_op("x * y");
        assert_eq!(result, Some("@202 { lhs: x, rhs: y }".to_string()));
    }

    #[test]
    fn test_parse_variable_definition() {
        let engine = RefactoringEngine::new(std::sync::Arc::new(crate::symbol_index::SymbolIndex::new()));

        let text = "let x = 42;";
        let range = Range {
            start: Position { line: 0, character: 0 },
            end: Position { line: 0, character: 11 },
        };

        let result = engine.parse_variable_definition(text, &range);
        assert_eq!(result, Some(("x".to_string(), "42".to_string())));
    }
}
