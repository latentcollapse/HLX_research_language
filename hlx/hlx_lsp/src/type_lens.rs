//! Type Lens
//!
//! Shows inferred types everywhere as inline hints.
//! Helps understand data flow and catch type errors early.

use tower_lsp::lsp_types::*;
use hlx_core::Value;

/// Type inference engine
pub struct TypeLens {
    /// Type cache for performance
    cache: std::sync::Mutex<std::collections::HashMap<String, String>>,
}

impl TypeLens {
    pub fn new() -> Self {
        Self {
            cache: std::sync::Mutex::new(std::collections::HashMap::new()),
        }
    }

    /// Infer types for a document
    pub fn infer_types(&self, text: &str) -> Vec<TypeHint> {
        let mut hints = Vec::new();

        for (line_idx, line) in text.lines().enumerate() {
            // Variable declarations
            if let Some(hint) = self.infer_variable_type(line, line_idx) {
                hints.push(hint);
            }

            // Function parameters
            hints.extend(self.infer_parameter_types(line, line_idx));

            // Return types
            if let Some(hint) = self.infer_return_type(line, line_idx) {
                hints.push(hint);
            }

            // Contract invocations
            hints.extend(self.infer_contract_types(line, line_idx));
        }

        hints
    }

    /// Infer type of variable declaration
    fn infer_variable_type(&self, line: &str, line_idx: usize) -> Option<TypeHint> {
        let trimmed = line.trim();

        if !trimmed.starts_with("let ") {
            return None;
        }

        // Extract: let VAR = VALUE;
        let after_let = trimmed[4..].trim();
        let eq_pos = after_let.find('=')?;

        let var_name = after_let[..eq_pos].trim();
        let value_str = after_let[eq_pos + 1..].trim()
            .trim_end_matches(';')
            .trim();

        // Infer type from value
        let inferred_type = self.infer_from_value(value_str);

        // Position after variable name
        let position = Position {
            line: line_idx as u32,
            character: (line.find(var_name).unwrap_or(0) + var_name.len()) as u32,
        };

        Some(TypeHint {
            position,
            type_str: inferred_type,
            kind: TypeHintKind::Variable,
        })
    }

    /// Infer types of function parameters
    fn infer_parameter_types(&self, line: &str, line_idx: usize) -> Vec<TypeHint> {
        let mut hints = Vec::new();
        let trimmed = line.trim();

        if !trimmed.starts_with("fn ") {
            return hints;
        }

        // Extract parameters
        let paren_start = match trimmed.find('(') {
            Some(pos) => pos,
            None => return hints,
        };
        let paren_end = match trimmed.find(')') {
            Some(pos) => pos,
            None => return hints,
        };

        let params = &trimmed[paren_start + 1..paren_end];

        for param in params.split(',') {
            let param = param.trim();
            if param.is_empty() {
                continue;
            }

            // Check if already typed
            if param.contains(':') {
                continue;
            }

            // Infer type as 'any' by default
            if let Some(pos) = line.find(param) {
                hints.push(TypeHint {
                    position: Position {
                        line: line_idx as u32,
                        character: (pos + param.len()) as u32,
                    },
                    type_str: "any".to_string(),
                    kind: TypeHintKind::Parameter,
                });
            }
        }

        hints
    }

    /// Infer return type of function
    fn infer_return_type(&self, line: &str, line_idx: usize) -> Option<TypeHint> {
        let trimmed = line.trim();

        if !trimmed.starts_with("fn ") {
            return None;
        }

        // Check if already has return type annotation
        if trimmed.contains("->") {
            return None;
        }

        // Position after closing paren
        let paren_end = trimmed.find(')')?;

        Some(TypeHint {
            position: Position {
                line: line_idx as u32,
                character: (paren_end + 1) as u32,
            },
            type_str: "void".to_string(), // Default, would need flow analysis for accuracy
            kind: TypeHintKind::Return,
        })
    }

    /// Infer types of contract invocations
    fn infer_contract_types(&self, line: &str, line_idx: usize) -> Vec<TypeHint> {
        let mut hints = Vec::new();

        // Find all @ symbols
        for (i, ch) in line.chars().enumerate() {
            if ch == '@' {
                // Extract contract ID
                let rest = &line[i + 1..];
                let id_len = rest.chars().take_while(|c| c.is_numeric()).count();

                if id_len > 0 {
                    let contract_id = &rest[..id_len];
                    let inferred_type = self.infer_contract_return_type(contract_id);

                    // Position after contract closing brace
                    if let Some(brace_end) = line[i..].find('}') {
                        hints.push(TypeHint {
                            position: Position {
                                line: line_idx as u32,
                                character: (i + brace_end + 1) as u32,
                            },
                            type_str: inferred_type,
                            kind: TypeHintKind::Expression,
                        });
                    }
                }
            }
        }

        hints
    }

    /// Infer type from value literal
    fn infer_from_value(&self, value_str: &str) -> String {
        // Boolean
        if value_str == "true" || value_str == "false" {
            return "bool".to_string();
        }

        // Null
        if value_str == "null" {
            return "null".to_string();
        }

        // String
        if value_str.starts_with('"') && value_str.ends_with('"') {
            return "string".to_string();
        }

        // Array
        if value_str.starts_with('[') && value_str.ends_with(']') {
            // Try to infer element type
            let inner = &value_str[1..value_str.len() - 1];
            if inner.is_empty() {
                return "array<any>".to_string();
            }

            let first_elem = inner.split(',').next().unwrap_or("").trim();
            let elem_type = self.infer_from_value(first_elem);

            return format!("array<{}>", elem_type);
        }

        // Object
        if value_str.starts_with('{') && value_str.ends_with('}') {
            return "object".to_string();
        }

        // Number (integer)
        if value_str.parse::<i64>().is_ok() {
            return "int".to_string();
        }

        // Number (float)
        if value_str.parse::<f64>().is_ok() {
            return "float".to_string();
        }

        // Contract invocation
        if value_str.starts_with('@') {
            let id_end = value_str.find(|c: char| !c.is_numeric())
                .unwrap_or(value_str.len());

            if id_end > 1 {
                let contract_id = &value_str[1..id_end];
                return self.infer_contract_return_type(contract_id);
            }
        }

        // Default
        "any".to_string()
    }

    /// Infer return type of contract
    fn infer_contract_return_type(&self, contract_id: &str) -> String {
        match contract_id {
            // Math operations -> number
            "200" | "201" | "202" | "203" => "number".to_string(),

            // String operations -> string
            "300" | "301" | "302" => "string".to_string(),

            // Array operations
            "400" => "int".to_string(),      // length
            "401" => "any".to_string(),      // index
            "402" => "array<any>".to_string(), // slice
            "403" => "array<any>".to_string(), // push

            // I/O operations
            "600" => "void".to_string(),     // print
            "602" => "string".to_string(),   // read_file
            "603" => "void".to_string(),     // write_file
            "604" => "object".to_string(),   // http_request

            // GPU operations
            "906" => "tensor".to_string(),   // GEMM
            "907" => "tensor".to_string(),   // LayerNorm
            "908" => "tensor".to_string(),   // GELU
            "909" => "tensor".to_string(),   // Softmax
            "910" => "float".to_string(),    // CrossEntropy

            _ => "unknown".to_string(),
        }
    }

    /// Create inlay hint from type hint
    pub fn create_inlay_hint(&self, hint: &TypeHint) -> InlayHint {
        let label = match hint.kind {
            TypeHintKind::Variable => format!(": {}", hint.type_str),
            TypeHintKind::Parameter => format!(": {}", hint.type_str),
            TypeHintKind::Return => format!(" -> {}", hint.type_str),
            TypeHintKind::Expression => format!(" : {}", hint.type_str),
        };

        InlayHint {
            position: hint.position,
            label: InlayHintLabel::String(label),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: Some(InlayHintTooltip::String(format!(
                "Inferred type: {}",
                hint.type_str
            ))),
            padding_left: Some(false),
            padding_right: Some(true),
            data: None,
        }
    }
}

/// Type hint information
#[derive(Debug, Clone)]
pub struct TypeHint {
    pub position: Position,
    pub type_str: String,
    pub kind: TypeHintKind,
}

/// Kind of type hint
#[derive(Debug, Clone, PartialEq)]
pub enum TypeHintKind {
    Variable,
    Parameter,
    Return,
    Expression,
}

impl Default for TypeLens {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_from_literals() {
        let lens = TypeLens::new();

        assert_eq!(lens.infer_from_value("42"), "int");
        assert_eq!(lens.infer_from_value("3.14"), "float");
        assert_eq!(lens.infer_from_value("\"hello\""), "string");
        assert_eq!(lens.infer_from_value("true"), "bool");
        assert_eq!(lens.infer_from_value("null"), "null");
        assert_eq!(lens.infer_from_value("[1, 2, 3]"), "array<int>");
        assert_eq!(lens.infer_from_value("{}"), "object");
    }

    #[test]
    fn test_infer_variable_type() {
        let lens = TypeLens::new();

        let hint = lens.infer_variable_type("let x = 42;", 0);
        assert!(hint.is_some());
        assert_eq!(hint.unwrap().type_str, "int");

        let hint = lens.infer_variable_type("let name = \"Alice\";", 0);
        assert!(hint.is_some());
        assert_eq!(hint.unwrap().type_str, "string");
    }

    #[test]
    fn test_infer_contract_return_type() {
        let lens = TypeLens::new();

        assert_eq!(lens.infer_contract_return_type("200"), "number");
        assert_eq!(lens.infer_contract_return_type("300"), "string");
        assert_eq!(lens.infer_contract_return_type("400"), "int");
        assert_eq!(lens.infer_contract_return_type("906"), "tensor");
    }
}
