//! Signature Help
//!
//! Shows parameter hints as users type function calls and contract invocations.
//! Displays required fields, types, and documentation.

use tower_lsp::lsp_types::*;
use crate::contracts::ContractCatalogue;
use hlx_core::BuiltinRegistry;

/// Signature help provider
pub struct SignatureHelpProvider {
    /// Trigger characters
    pub triggers: Vec<String>,
    /// Unified builtin registry
    registry: BuiltinRegistry,
}

impl SignatureHelpProvider {
    pub fn new() -> Self {
        Self {
            triggers: vec![
                "{".to_string(),  // Contract field list start
                ",".to_string(),  // Next parameter
                "(".to_string(),  // Function call start
            ],
            registry: BuiltinRegistry::new(),
        }
    }

    /// Generate signature help for contract invocations
    pub fn get_contract_signature(
        &self,
        text: &str,
        position: &Position,
        catalogue: &ContractCatalogue,
    ) -> Option<SignatureHelp> {
        // Find the contract being typed
        let line = text.lines().nth(position.line as usize)?;

        // Look backwards from cursor for @ symbol
        let before_cursor = &line[..position.character.min(line.len() as u32) as usize];

        // Find most recent @
        let at_pos = before_cursor.rfind('@')?;

        // Extract contract ID
        let after_at = &before_cursor[at_pos + 1..];
        let id_end = after_at.find(|c: char| !c.is_numeric())
            .unwrap_or(after_at.len());

        if id_end == 0 {
            return None;
        }

        let contract_id = &after_at[..id_end];

        // Get contract spec
        let spec = catalogue.get_contract(contract_id)?;

        // Determine which parameter we're on
        let active_parameter = self.get_active_parameter(before_cursor, at_pos);

        // Build signature information
        let mut parameters = Vec::new();

        for (field_name, field_spec) in &spec.fields {
            let label = if field_spec.required {
                format!("{}: {}", field_name, field_spec.field_type)
            } else {
                format!("{}?: {}", field_name, field_spec.field_type)
            };

            parameters.push(ParameterInformation {
                label: ParameterLabel::Simple(label),
                documentation: Some(Documentation::String(field_spec.description.clone())),
            });
        }

        // Build signature
        let signature = SignatureInformation {
            label: format!("@{} {{ {} }}", contract_id,
                parameters.iter()
                    .map(|p| {
                        if let ParameterLabel::Simple(ref s) = p.label {
                            s.clone()
                        } else {
                            String::new()
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!(
                    "**{}** ({})\n\n{}\n\n**Usage**: `{}`",
                    spec.name,
                    spec.tier,
                    spec.description,
                    spec.example
                ),
            })),
            parameters: Some(parameters),
            active_parameter: Some(active_parameter),
        };

        Some(SignatureHelp {
            signatures: vec![signature],
            active_signature: Some(0),
            active_parameter: Some(active_parameter),
        })
    }

    /// Generate signature help for function calls
    pub fn get_function_signature(
        &self,
        text: &str,
        position: &Position,
        function_name: &str,
    ) -> Option<SignatureHelp> {
        // Check if it's a builtin function
        let sig = self.registry.get(function_name)?;

        // Determine active parameter
        let line = text.lines().nth(position.line as usize)?;
        let before_cursor = &line[..position.character.min(line.len() as u32) as usize];

        // Count commas to determine parameter position
        let active_param = before_cursor.matches(',').count() as u32;

        // Build parameter information
        let parameters: Vec<ParameterInformation> = sig.params.iter().enumerate().map(|(i, param)| {
            ParameterInformation {
                label: ParameterLabel::Simple(format!("arg{}: {}", i, param.to_string())),
                documentation: None,
            }
        }).collect();

        // Build signature label
        let param_labels: Vec<String> = sig.params.iter().enumerate()
            .map(|(i, p)| format!("arg{}: {}", i, p.to_string()))
            .collect();

        let label = if sig.max_args.is_none() && sig.min_args == 0 {
            format!("{}(...)", function_name)
        } else if sig.max_args.is_none() {
            format!("{}({}, ...)", function_name, param_labels.join(", "))
        } else {
            format!("{}({})", function_name, param_labels.join(", "))
        };

        let signature = SignatureInformation {
            label,
            documentation: Some(Documentation::MarkupContent(MarkupContent {
                kind: MarkupKind::Markdown,
                value: format!("**{}**\n\n{}\n\n**Returns**: {}",
                    function_name,
                    sig.description,
                    sig.return_type.to_string()),
            })),
            parameters: if parameters.is_empty() { None } else { Some(parameters) },
            active_parameter: Some(active_param),
        };

        Some(SignatureHelp {
            signatures: vec![signature],
            active_signature: Some(0),
            active_parameter: Some(active_param),
        })
    }

    /// Get active parameter index based on cursor position
    fn get_active_parameter(&self, text_before_cursor: &str, contract_start: usize) -> u32 {
        // Count commas after the opening brace
        let after_contract = &text_before_cursor[contract_start..];

        if let Some(brace_pos) = after_contract.find('{') {
            let inside_braces = &after_contract[brace_pos + 1..];
            return inside_braces.matches(',').count() as u32;
        }

        0
    }

    /// Detect if we're inside a contract or function call
    pub fn detect_context(&self, text: &str, position: &Position) -> SignatureContext {
        let line = text.lines().nth(position.line as usize)
            .unwrap_or("");

        let before_cursor = &line[..position.character.min(line.len() as u32) as usize];

        // Check for contract (@ID {)
        if let Some(at_pos) = before_cursor.rfind('@') {
            let after_at = &before_cursor[at_pos..];
            if after_at.contains('{') && !after_at.contains('}') {
                return SignatureContext::Contract;
            }
        }

        // Check for function call (name()
        if let Some(paren_pos) = before_cursor.rfind('(') {
            // Find function name before paren
            let before_paren = &before_cursor[..paren_pos];
            if let Some(name_start) = before_paren.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                let func_name = &before_paren[name_start + 1..];
                if !func_name.is_empty() {
                    return SignatureContext::Function(func_name.to_string());
                }
            }
        }

        SignatureContext::None
    }
}

/// Context for signature help
#[derive(Debug, Clone)]
pub enum SignatureContext {
    Contract,
    Function(String),
    None,
}

impl Default for SignatureHelpProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_contract_context() {
        let provider = SignatureHelpProvider::new();

        let text = "let x = @200 { ";
        let pos = Position { line: 0, character: 15 };

        let context = provider.detect_context(text, &pos);
        matches!(context, SignatureContext::Contract);
    }

    #[test]
    fn test_detect_function_context() {
        let provider = SignatureHelpProvider::new();

        let text = "let x = slice(arr, ";
        let pos = Position { line: 0, character: 18 };

        let context = provider.detect_context(text, &pos);
        if let SignatureContext::Function(name) = context {
            assert_eq!(name, "slice");
        } else {
            panic!("Expected Function context");
        }
    }

    #[test]
    fn test_active_parameter_counting() {
        let provider = SignatureHelpProvider::new();

        let text = "@200 { lhs: 5, ";
        let active = provider.get_active_parameter(text, 0);

        assert_eq!(active, 1); // Second parameter (after one comma)
    }
}
