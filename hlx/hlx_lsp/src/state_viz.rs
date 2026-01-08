//! State Visualization
//!
//! Tracks variable values and shows them as inline hints.
//! Helps users understand program state as they write code.

use tower_lsp::lsp_types::*;
use hlx_core::Value;
use std::collections::HashMap;

/// A state snapshot at a particular line
#[derive(Debug, Clone)]
pub struct StateSnapshot {
    pub line: usize,
    pub variables: HashMap<String, VariableState>,
}

/// Information about a variable
#[derive(Debug, Clone)]
pub struct VariableState {
    pub name: String,
    pub value: Option<Value>,
    pub type_hint: String,
    pub last_modified: usize, // Line number
}

/// State visualization engine
pub struct StateVizEngine {
    /// Maximum number of variables to track
    max_vars: usize,
}

impl StateVizEngine {
    pub fn new() -> Self {
        Self {
            max_vars: 100,
        }
    }

    /// Analyze a document and generate state snapshots
    pub fn analyze_state(&self, text: &str) -> Vec<StateSnapshot> {
        let mut snapshots = Vec::new();
        let mut current_state: HashMap<String, VariableState> = HashMap::new();

        for (line_idx, line) in text.lines().enumerate() {
            // Check for variable declarations (let statements)
            if let Some(var_state) = self.parse_let_statement(line, line_idx) {
                current_state.insert(var_state.name.clone(), var_state);

                // Create snapshot after this line
                snapshots.push(StateSnapshot {
                    line: line_idx,
                    variables: current_state.clone(),
                });
            }

            // Check for assignments (var = value)
            if let Some((var_name, value)) = self.parse_assignment(line) {
                if let Some(var_state) = current_state.get_mut(&var_name) {
                    var_state.value = Some(value.clone());
                    var_state.last_modified = line_idx;

                    // Create snapshot
                    snapshots.push(StateSnapshot {
                        line: line_idx,
                        variables: current_state.clone(),
                    });
                }
            }
        }

        snapshots
    }

    /// Parse a let statement to extract variable name and initial value
    fn parse_let_statement(&self, line: &str, line_idx: usize) -> Option<VariableState> {
        let trimmed = line.trim();

        // Check if it starts with "let"
        if !trimmed.starts_with("let ") {
            return None;
        }

        // Extract: let VAR = VALUE;
        let after_let = &trimmed[4..].trim();

        // Find the equals sign
        let eq_pos = after_let.find('=')?;

        let var_name = after_let[..eq_pos].trim().to_string();

        // Extract value part
        let value_part = after_let[eq_pos + 1..].trim();

        // Remove trailing semicolon if present
        let value_str = if value_part.ends_with(';') {
            &value_part[..value_part.len() - 1]
        } else {
            value_part
        }.trim();

        // Try to parse the value
        let value = self.try_parse_simple_value(value_str);

        // Infer type
        let type_hint = if let Some(ref v) = value {
            self.value_type_name(v)
        } else {
            "unknown".to_string()
        };

        Some(VariableState {
            name: var_name,
            value,
            type_hint,
            last_modified: line_idx,
        })
    }

    /// Parse an assignment statement (not a declaration)
    fn parse_assignment(&self, line: &str) -> Option<(String, Value)> {
        let trimmed = line.trim();

        // Skip if it's a let statement or function declaration
        if trimmed.starts_with("let ") || trimmed.starts_with("fn ") {
            return None;
        }

        // Look for VAR = VALUE pattern
        let eq_pos = trimmed.find('=')?;

        // Make sure it's not ==, !=, <=, >=
        if eq_pos > 0 {
            let before = trimmed.chars().nth(eq_pos - 1)?;
            if before == '!' || before == '=' || before == '<' || before == '>' {
                return None;
            }
        }

        let after_eq = trimmed.chars().nth(eq_pos + 1)?;
        if after_eq == '=' {
            return None; // This is ==
        }

        let var_name = trimmed[..eq_pos].trim().to_string();
        let value_part = trimmed[eq_pos + 1..].trim();

        // Remove trailing semicolon
        let value_str = if value_part.ends_with(';') {
            &value_part[..value_part.len() - 1]
        } else {
            value_part
        }.trim();

        let value = self.try_parse_simple_value(value_str)?;

        Some((var_name, value))
    }

    /// Try to parse a simple literal value
    fn try_parse_simple_value(&self, value_str: &str) -> Option<Value> {
        let value_str = value_str.trim();

        // Boolean
        if value_str == "true" {
            return Some(Value::Boolean(true));
        }
        if value_str == "false" {
            return Some(Value::Boolean(false));
        }

        // Null
        if value_str == "null" {
            return Some(Value::Null);
        }

        // String (quoted)
        if value_str.starts_with('"') && value_str.ends_with('"') && value_str.len() >= 2 {
            let content = &value_str[1..value_str.len() - 1];
            return Some(Value::String(content.to_string()));
        }

        // Number (integer)
        if let Ok(int_val) = value_str.parse::<i64>() {
            return Some(Value::Integer(int_val));
        }

        // Number (float)
        if let Ok(float_val) = value_str.parse::<f64>() {
            return Some(Value::Float(float_val));
        }

        // Array (simple case: [1, 2, 3])
        if value_str.starts_with('[') && value_str.ends_with(']') {
            let inner = &value_str[1..value_str.len() - 1];
            let elements: Vec<Value> = inner
                .split(',')
                .filter_map(|s| self.try_parse_simple_value(s.trim()))
                .collect();

            if !elements.is_empty() {
                return Some(Value::Array(elements.into()));
            } else if inner.trim().is_empty() {
                // Empty array
                return Some(Value::Array(Vec::new().into()));
            }
        }

        // Contract call or expression - return None (can't evaluate statically)
        None
    }

    /// Get type name for a value
    fn value_type_name(&self, value: &Value) -> String {
        match value {
            Value::Integer(_) => "int".to_string(),
            Value::Float(_) => "float".to_string(),
            Value::String(_) => "string".to_string(),
            Value::Boolean(_) => "bool".to_string(),
            Value::Null => "null".to_string(),
            Value::Array(_) => "array".to_string(),
            Value::Object(_) => "object".to_string(),
            Value::Contract(_) => "contract".to_string(),
            Value::Handle(_) => "handle".to_string(),
        }
    }

    /// Create inlay hints for variable states
    pub fn create_inlay_hints(&self, snapshots: &[StateSnapshot]) -> Vec<InlayHint> {
        let mut hints = Vec::new();

        for snapshot in snapshots {
            // For each variable that was just modified/declared on this line
            for (var_name, var_state) in &snapshot.variables {
                if var_state.last_modified == snapshot.line {
                    // Create hint showing variable type and value
                    let label = if let Some(ref value) = var_state.value {
                        format!(" : {} = {}", var_state.type_hint, self.format_value(value))
                    } else {
                        format!(" : {}", var_state.type_hint)
                    };

                    hints.push(InlayHint {
                        position: Position {
                            line: snapshot.line as u32,
                            character: 200, // End of line
                        },
                        label: InlayHintLabel::String(label),
                        kind: Some(InlayHintKind::TYPE),
                        text_edits: None,
                        tooltip: Some(InlayHintTooltip::String(format!(
                            "Variable '{}' = {}",
                            var_name,
                            if let Some(ref v) = var_state.value {
                                self.format_value(v)
                            } else {
                                "?".to_string()
                            }
                        ))),
                        padding_left: Some(true),
                        padding_right: None,
                        data: None,
                    });
                }
            }
        }

        hints
    }

    /// Format a value for display
    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => {
                if f.fract() == 0.0 && f.abs() < 1e10 {
                    format!("{:.0}", f)
                } else {
                    format!("{:.2}", f)
                }
            }
            Value::String(s) => {
                if s.len() > 20 {
                    format!("\"{}...\"", &s[..17])
                } else {
                    format!("\"{}\"", s)
                }
            }
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                if arr.is_empty() {
                    "[]".to_string()
                } else if arr.len() <= 3 {
                    let elements: Vec<String> = arr.iter().map(|v| self.format_value(v)).collect();
                    format!("[{}]", elements.join(", "))
                } else {
                    format!("[{} items]", arr.len())
                }
            }
            Value::Object(obj) => {
                if obj.is_empty() {
                    "{}".to_string()
                } else {
                    format!("{{...}} ({} keys)", obj.len())
                }
            }
            Value::Contract(_) => "<contract>".to_string(),
            Value::Handle(_) => "<handle>".to_string(),
        }
    }

    /// Get all variables at a specific line
    pub fn get_variables_at_line(&self, snapshots: &[StateSnapshot], line: usize) -> HashMap<String, VariableState> {
        // Find the most recent snapshot before or at this line
        snapshots
            .iter()
            .rev()
            .find(|snapshot| snapshot.line <= line)
            .map(|snapshot| snapshot.variables.clone())
            .unwrap_or_default()
    }
}

impl Default for StateVizEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_let_statement() {
        let engine = StateVizEngine::new();

        let var = engine.parse_let_statement("let x = 42;", 0).unwrap();
        assert_eq!(var.name, "x");
        assert_eq!(var.value, Some(Value::Integer(42)));
        assert_eq!(var.type_hint, "int");

        let var = engine.parse_let_statement("let message = \"hello\";", 0).unwrap();
        assert_eq!(var.name, "message");
        assert_eq!(var.value, Some(Value::String("hello".to_string())));
        assert_eq!(var.type_hint, "string");
    }

    #[test]
    fn test_parse_assignment() {
        let engine = StateVizEngine::new();

        let (name, value) = engine.parse_assignment("x = 100;").unwrap();
        assert_eq!(name, "x");
        assert_eq!(value, Value::Integer(100));

        // Should not parse == as assignment
        assert!(engine.parse_assignment("if (x == 100)").is_none());

        // Should not parse let statements
        assert!(engine.parse_assignment("let x = 50;").is_none());
    }

    #[test]
    fn test_state_tracking() {
        let engine = StateVizEngine::new();

        let code = r#"
let x = 10;
let y = 20;
x = 15;
"#;

        let snapshots = engine.analyze_state(code);

        // Should have 3 snapshots (2 let statements, 1 assignment)
        assert_eq!(snapshots.len(), 3);

        // Check final state
        let final_state = &snapshots[2].variables;
        assert_eq!(final_state.get("x").unwrap().value, Some(Value::Integer(15)));
        assert_eq!(final_state.get("y").unwrap().value, Some(Value::Integer(20)));
    }

    #[test]
    fn test_array_parsing() {
        let engine = StateVizEngine::new();

        let var = engine.parse_let_statement("let arr = [1, 2, 3];", 0).unwrap();
        assert_eq!(var.type_hint, "array");

        if let Some(Value::Array(elements)) = var.value {
            assert_eq!(elements.len(), 3);
        } else {
            panic!("Expected array value");
        }
    }
}
