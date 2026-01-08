//! Inline Execution Preview
//!
//! Shows evaluation results as you type for immediate feedback.
//! Safely evaluates contract invocations and displays results as inlay hints.

use tower_lsp::lsp_types::*;
use hlx_core::Value;
use crate::contracts::ContractCatalogue;

/// A preview result with position
#[derive(Debug, Clone)]
pub struct InlinePreview {
    pub position: Position,
    pub result: PreviewResult,
}

/// The result of evaluating a contract
#[derive(Debug, Clone)]
pub enum PreviewResult {
    Success(Value),
    Error(String),
    Skipped(String), // Reason for skipping (e.g., "Side effects")
}

/// Inline preview engine
pub struct InlinePreviewEngine {
    /// Maximum depth for nested evaluations
    max_depth: usize,
}

impl InlinePreviewEngine {
    pub fn new() -> Self {
        Self {
            max_depth: 5,
        }
    }

    /// Analyze a document and generate previews for safe contract calls
    pub fn generate_previews(&self, text: &str, catalogue: &ContractCatalogue) -> Vec<InlinePreview> {
        let mut previews = Vec::new();

        // Parse each line looking for contract invocations
        for (line_idx, line) in text.lines().enumerate() {
            if let Some(preview) = self.try_preview_line(line, line_idx, catalogue) {
                previews.push(preview);
            }
        }

        previews
    }

    /// Try to preview a single line
    fn try_preview_line(&self, line: &str, line_idx: usize, catalogue: &ContractCatalogue) -> Option<InlinePreview> {
        // Look for contract invocations: @ID { ... }
        if !line.contains('@') || !line.contains('{') {
            return None;
        }

        // Parse contract ID
        let at_pos = line.find('@')?;
        let id_start = at_pos + 1;

        let id_end = line[id_start..]
            .find(|c: char| !c.is_numeric())
            .map(|i| id_start + i)?;

        let contract_id = &line[id_start..id_end];

        // Skip if contract doesn't exist
        let spec = catalogue.get_contract(contract_id)?;

        // Skip contracts with side effects (I/O, HTTP, etc.)
        if self.has_side_effects(contract_id) {
            return Some(InlinePreview {
                position: Position {
                    line: line_idx as u32,
                    character: line.len() as u32,
                },
                result: PreviewResult::Skipped("Has side effects".to_string()),
            });
        }

        // Extract and parse fields
        let brace_start = line[at_pos..].find('{')?;
        let brace_end = line[at_pos + brace_start..].find('}')?;

        let fields_section = &line[at_pos + brace_start + 1..at_pos + brace_start + brace_end];

        // Parse field values
        let fields = self.parse_fields(fields_section)?;

        // Evaluate the contract
        let result = self.evaluate_contract(contract_id, &fields, catalogue);

        Some(InlinePreview {
            position: Position {
                line: line_idx as u32,
                character: line.len() as u32,
            },
            result,
        })
    }

    /// Check if a contract has side effects
    fn has_side_effects(&self, contract_id: &str) -> bool {
        // I/O contracts (600-699)
        if let Ok(id) = contract_id.parse::<u32>() {
            if (600..700).contains(&id) {
                return true;
            }
        }

        false
    }

    /// Parse field values from a contract invocation
    fn parse_fields(&self, fields_str: &str) -> Option<Vec<(String, Value)>> {
        let mut fields = Vec::new();

        for field_pair in fields_str.split(',') {
            let field_pair = field_pair.trim();
            if field_pair.is_empty() {
                continue;
            }

            // Split by colon
            let parts: Vec<&str> = field_pair.splitn(2, ':').collect();
            if parts.len() != 2 {
                continue;
            }

            let field_name = parts[0].trim().to_string();
            let field_value = parts[1].trim();

            // Try to parse the value
            if let Some(value) = self.parse_value(field_value) {
                fields.push((field_name, value));
            } else {
                // If we can't parse a value, skip this entire invocation
                return None;
            }
        }

        Some(fields)
    }

    /// Parse a simple value (numbers, strings, booleans)
    fn parse_value(&self, value_str: &str) -> Option<Value> {
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
        if value_str.starts_with('"') && value_str.ends_with('"') {
            let content = &value_str[1..value_str.len()-1];
            return Some(Value::String(content.to_string()));
        }

        // Number (integer or float)
        if let Ok(int_val) = value_str.parse::<i64>() {
            return Some(Value::Integer(int_val));
        }
        if let Ok(float_val) = value_str.parse::<f64>() {
            return Some(Value::Float(float_val));
        }

        // Array (simple case: [1, 2, 3])
        if value_str.starts_with('[') && value_str.ends_with(']') {
            let inner = &value_str[1..value_str.len()-1];
            let elements: Vec<Value> = inner
                .split(',')
                .filter_map(|s| self.parse_value(s.trim()))
                .collect();

            if !elements.is_empty() {
                return Some(Value::Array(elements.into()));
            }
        }

        None
    }

    /// Evaluate a contract with given fields
    fn evaluate_contract(
        &self,
        contract_id: &str,
        fields: &[(String, Value)],
        _catalogue: &ContractCatalogue,
    ) -> PreviewResult {
        // Get field values by name
        let get_field = |name: &str| -> Option<&Value> {
            fields.iter().find(|(n, _)| n == name).map(|(_, v)| v)
        };

        // Helper to get required field or return error
        let get_required = |name: &str| -> Result<&Value, PreviewResult> {
            get_field(name).ok_or_else(|| {
                PreviewResult::Error(format!("Missing required field: {}", name))
            })
        };

        // Basic math operations (200-203)
        match contract_id {
            "200" => {
                // Add
                let lhs = match get_required("lhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let rhs = match get_required("rhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.add_values(lhs, rhs)
            }
            "201" => {
                // Subtract
                let lhs = match get_required("lhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let rhs = match get_required("rhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.subtract_values(lhs, rhs)
            }
            "202" => {
                // Multiply
                let lhs = match get_required("lhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let rhs = match get_required("rhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.multiply_values(lhs, rhs)
            }
            "203" => {
                // Divide
                let lhs = match get_required("lhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let rhs = match get_required("rhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.divide_values(lhs, rhs)
            }
            "300" => {
                // String concat
                let lhs = match get_required("lhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let rhs = match get_required("rhs") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.concat_values(lhs, rhs)
            }
            "400" => {
                // Array length
                let arr = match get_required("@0") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.array_length(arr)
            }
            "401" => {
                // Array index
                let arr = match get_required("@0") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                let idx = match get_required("@1") {
                    Ok(v) => v,
                    Err(e) => return e,
                };
                self.array_index(arr, idx)
            }
            _ => {
                // Unknown or unsupported contract
                PreviewResult::Skipped("Not supported for preview".to_string())
            }
        }
    }

    // Math operations
    fn add_values(&self, lhs: &Value, rhs: &Value) -> PreviewResult {
        match (lhs, rhs) {
            (Value::Integer(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Integer(a + b))
            }
            (Value::Float(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(a + b))
            }
            (Value::Integer(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(*a as f64 + b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Float(a + *b as f64))
            }
            _ => PreviewResult::Error("Type mismatch for addition".to_string()),
        }
    }

    fn subtract_values(&self, lhs: &Value, rhs: &Value) -> PreviewResult {
        match (lhs, rhs) {
            (Value::Integer(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Integer(a - b))
            }
            (Value::Float(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(a - b))
            }
            (Value::Integer(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(*a as f64 - b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Float(a - *b as f64))
            }
            _ => PreviewResult::Error("Type mismatch for subtraction".to_string()),
        }
    }

    fn multiply_values(&self, lhs: &Value, rhs: &Value) -> PreviewResult {
        match (lhs, rhs) {
            (Value::Integer(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Integer(a * b))
            }
            (Value::Float(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(a * b))
            }
            (Value::Integer(a), Value::Float(b)) => {
                PreviewResult::Success(Value::Float(*a as f64 * b))
            }
            (Value::Float(a), Value::Integer(b)) => {
                PreviewResult::Success(Value::Float(a * *b as f64))
            }
            _ => PreviewResult::Error("Type mismatch for multiplication".to_string()),
        }
    }

    fn divide_values(&self, lhs: &Value, rhs: &Value) -> PreviewResult {
        match (lhs, rhs) {
            (Value::Integer(a), Value::Integer(b)) => {
                if *b == 0 {
                    PreviewResult::Error("Division by zero".to_string())
                } else {
                    PreviewResult::Success(Value::Integer(a / b))
                }
            }
            (Value::Float(a), Value::Float(b)) => {
                if *b == 0.0 {
                    PreviewResult::Error("Division by zero".to_string())
                } else {
                    PreviewResult::Success(Value::Float(a / b))
                }
            }
            (Value::Integer(a), Value::Float(b)) => {
                if *b == 0.0 {
                    PreviewResult::Error("Division by zero".to_string())
                } else {
                    PreviewResult::Success(Value::Float(*a as f64 / b))
                }
            }
            (Value::Float(a), Value::Integer(b)) => {
                if *b == 0 {
                    PreviewResult::Error("Division by zero".to_string())
                } else {
                    PreviewResult::Success(Value::Float(a / *b as f64))
                }
            }
            _ => PreviewResult::Error("Type mismatch for division".to_string()),
        }
    }

    // String operations
    fn concat_values(&self, lhs: &Value, rhs: &Value) -> PreviewResult {
        match (lhs, rhs) {
            (Value::String(a), Value::String(b)) => {
                PreviewResult::Success(Value::String(format!("{}{}", a, b)))
            }
            _ => PreviewResult::Error("Type mismatch for string concatenation".to_string()),
        }
    }

    // Array operations
    fn array_length(&self, arr: &Value) -> PreviewResult {
        match arr {
            Value::Array(elements) => {
                PreviewResult::Success(Value::Integer(elements.len() as i64))
            }
            Value::String(s) => {
                PreviewResult::Success(Value::Integer(s.len() as i64))
            }
            _ => PreviewResult::Error("Type mismatch: expected array or string".to_string()),
        }
    }

    fn array_index(&self, arr: &Value, idx: &Value) -> PreviewResult {
        let index = match idx {
            Value::Integer(i) => *i as usize,
            _ => return PreviewResult::Error("Index must be an integer".to_string()),
        };

        match arr {
            Value::Array(elements) => {
                if index < elements.len() {
                    PreviewResult::Success(elements[index].clone())
                } else {
                    PreviewResult::Error(format!("Index {} out of bounds (len={})", index, elements.len()))
                }
            }
            _ => PreviewResult::Error("Type mismatch: expected array".to_string()),
        }
    }

    /// Create an inlay hint for a preview result
    pub fn create_inlay_hint(&self, preview: &InlinePreview) -> InlayHint {
        let label = match &preview.result {
            PreviewResult::Success(value) => {
                format!(" → {}", self.format_value(value))
            }
            PreviewResult::Error(err) => {
                format!(" ⚠ {}", err)
            }
            PreviewResult::Skipped(reason) => {
                format!(" ⊘ {}", reason)
            }
        };

        InlayHint {
            position: preview.position,
            label: InlayHintLabel::String(label),
            kind: Some(InlayHintKind::TYPE),
            text_edits: None,
            tooltip: None,
            padding_left: Some(true),
            padding_right: None,
            data: None,
        }
    }

    /// Format a value for display
    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Integer(i) => i.to_string(),
            Value::Float(f) => format!("{:.2}", f),
            Value::String(s) => format!("\"{}\"", s),
            Value::Boolean(b) => b.to_string(),
            Value::Null => "null".to_string(),
            Value::Array(arr) => {
                if arr.len() <= 3 {
                    let elements: Vec<String> = arr.iter().map(|v| self.format_value(v)).collect();
                    format!("[{}]", elements.join(", "))
                } else {
                    format!("[... {} items]", arr.len())
                }
            }
            Value::Object(_) => "{...}".to_string(),
            Value::Contract(_) => "<contract>".to_string(),
            Value::Handle(_) => "<handle>".to_string(),
        }
    }
}

impl Default for InlinePreviewEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_value() {
        let engine = InlinePreviewEngine::new();

        assert_eq!(engine.parse_value("42"), Some(Value::Integer(42)));
        assert_eq!(engine.parse_value("3.14"), Some(Value::Float(3.14)));
        assert_eq!(engine.parse_value("true"), Some(Value::Boolean(true)));
        assert_eq!(engine.parse_value("false"), Some(Value::Boolean(false)));
        assert_eq!(engine.parse_value("\"hello\""), Some(Value::String("hello".to_string())));
        assert_eq!(engine.parse_value("null"), Some(Value::Null));
    }

    #[test]
    fn test_add_values() {
        let engine = InlinePreviewEngine::new();

        let result = engine.add_values(&Value::Integer(5), &Value::Integer(3));
        match result {
            PreviewResult::Success(Value::Integer(8)) => (),
            _ => panic!("Expected 8"),
        }

        let result = engine.add_values(&Value::Float(2.5), &Value::Float(1.5));
        match result {
            PreviewResult::Success(Value::Float(f)) if (f - 4.0).abs() < 0.001 => (),
            _ => panic!("Expected 4.0"),
        }
    }

    #[test]
    fn test_divide_by_zero() {
        let engine = InlinePreviewEngine::new();

        let result = engine.divide_values(&Value::Integer(10), &Value::Integer(0));
        match result {
            PreviewResult::Error(msg) if msg.contains("Division by zero") => (),
            _ => panic!("Expected division by zero error"),
        }
    }
}
