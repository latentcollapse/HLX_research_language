//! Signature Help
//!
//! Shows parameter hints as users type function calls and contract invocations.
//! Displays required fields, types, and documentation.

use tower_lsp::lsp_types::*;
use crate::contracts::ContractCatalogue;

/// Signature help provider
pub struct SignatureHelpProvider {
    /// Trigger characters
    pub triggers: Vec<String>,
}

impl SignatureHelpProvider {
    pub fn new() -> Self {
        Self {
            triggers: vec![
                "{".to_string(),  // Contract field list start
                ",".to_string(),  // Next parameter
                "(".to_string(),  // Function call start
            ],
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
        // For now, handle built-in functions
        let builtin_sigs = self.get_builtin_signatures();

        if let Some(sig_info) = builtin_sigs.get(function_name) {
            // Determine active parameter
            let line = text.lines().nth(position.line as usize)?;
            let before_cursor = &line[..position.character.min(line.len() as u32) as usize];

            // Count commas to determine parameter position
            let active_param = before_cursor.matches(',').count() as u32;

            let mut signature = sig_info.clone();
            signature.active_parameter = Some(active_param);

            return Some(SignatureHelp {
                signatures: vec![signature],
                active_signature: Some(0),
                active_parameter: Some(active_param),
            });
        }

        None
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

    /// Get built-in function signatures
    fn get_builtin_signatures(&self) -> std::collections::HashMap<String, SignatureInformation> {
        let mut sigs = std::collections::HashMap::new();

        // print
        sigs.insert("print".to_string(), SignatureInformation {
            label: "print(value)".to_string(),
            documentation: Some(Documentation::String(
                "Print a value to stdout".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("value: any".to_string()),
                    documentation: Some(Documentation::String("Value to print".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        // len
        sigs.insert("len".to_string(), SignatureInformation {
            label: "len(container)".to_string(),
            documentation: Some(Documentation::String(
                "Get the length of a string, array, or object".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("container: string | array | object".to_string()),
                    documentation: Some(Documentation::String("Container to measure".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        // to_int
        sigs.insert("to_int".to_string(), SignatureInformation {
            label: "to_int(value)".to_string(),
            documentation: Some(Documentation::String(
                "Convert value to integer (truncates floats)".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("value: any".to_string()),
                    documentation: Some(Documentation::String("Value to convert".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        // slice
        sigs.insert("slice".to_string(), SignatureInformation {
            label: "slice(array, start, length)".to_string(),
            documentation: Some(Documentation::String(
                "Extract a slice from an array".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("array: array".to_string()),
                    documentation: Some(Documentation::String("Source array".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("start: int".to_string()),
                    documentation: Some(Documentation::String("Starting index".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("length: int".to_string()),
                    documentation: Some(Documentation::String("Number of elements".to_string())),
                },
            ]),
            active_parameter: Some(0),
        });

        // append
        sigs.insert("append".to_string(), SignatureInformation {
            label: "append(array, item)".to_string(),
            documentation: Some(Documentation::String(
                "Return new array with item appended".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("array: array".to_string()),
                    documentation: Some(Documentation::String("Source array".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("item: any".to_string()),
                    documentation: Some(Documentation::String("Item to append".to_string())),
                },
            ]),
            active_parameter: Some(0),
        });

        // type
        sigs.insert("type".to_string(), SignatureInformation {
            label: "type(value)".to_string(),
            documentation: Some(Documentation::String(
                "Get the type name of a value".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("value: any".to_string()),
                    documentation: Some(Documentation::String("Value to check".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        // read_file
        sigs.insert("read_file".to_string(), SignatureInformation {
            label: "read_file(path)".to_string(),
            documentation: Some(Documentation::String(
                "Read file contents as string".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("path: string".to_string()),
                    documentation: Some(Documentation::String("File path".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        // http_request
        sigs.insert("http_request".to_string(), SignatureInformation {
            label: "http_request(method, url, body, headers)".to_string(),
            documentation: Some(Documentation::String(
                "Make an HTTP request".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("method: string".to_string()),
                    documentation: Some(Documentation::String("HTTP method (GET, POST, etc)".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("url: string".to_string()),
                    documentation: Some(Documentation::String("Target URL".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("body: string".to_string()),
                    documentation: Some(Documentation::String("Request body".to_string())),
                },
                ParameterInformation {
                    label: ParameterLabel::Simple("headers: object".to_string()),
                    documentation: Some(Documentation::String("HTTP headers".to_string())),
                },
            ]),
            active_parameter: Some(0),
        });

        // json_parse
        sigs.insert("json_parse".to_string(), SignatureInformation {
            label: "json_parse(json_string)".to_string(),
            documentation: Some(Documentation::String(
                "Parse JSON string into HLX value".to_string()
            )),
            parameters: Some(vec![
                ParameterInformation {
                    label: ParameterLabel::Simple("json_string: string".to_string()),
                    documentation: Some(Documentation::String("JSON to parse".to_string())),
                }
            ]),
            active_parameter: Some(0),
        });

        sigs
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
