//! Code Lens
//!
//! Shows actionable information above functions and definitions:
//! - Reference counts ("X references")
//! - Run/Debug buttons
//! - Test status

use tower_lsp::lsp_types::*;
use crate::symbol_index::SymbolIndex;

/// Code lens provider
pub struct CodeLensProvider {
    symbol_index: std::sync::Arc<SymbolIndex>,
}

impl CodeLensProvider {
    pub fn new(symbol_index: std::sync::Arc<SymbolIndex>) -> Self {
        Self { symbol_index }
    }

    /// Generate code lenses for a document
    pub fn get_code_lenses(&self, uri: &Url, text: &str) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();

            // Function definitions
            if trimmed.starts_with("fn ") {
                lenses.extend(self.create_function_lenses(uri, text, line_idx, line));
            }

            // Program entry point
            if trimmed.starts_with("program ") {
                lenses.push(self.create_run_lens(line_idx));
            }

            // Variable declarations
            if trimmed.starts_with("let ") {
                if let Some(lens) = self.create_variable_lens(uri, text, line_idx, line) {
                    lenses.push(lens);
                }
            }
        }

        lenses
    }

    /// Create lenses for function definitions
    fn create_function_lenses(&self, uri: &Url, text: &str, line_idx: usize, line: &str) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        // Extract function name
        let after_fn = line.trim()[3..].trim();
        if let Some(paren_pos) = after_fn.find('(') {
            let func_name = &after_fn[..paren_pos];

            // Count references
            let position = Position {
                line: line_idx as u32,
                character: (line.find(func_name).unwrap_or(0)) as u32,
            };

            let references = self.symbol_index.find_references(&position, uri, text);
            let ref_count = references.len().saturating_sub(1); // Exclude definition

            // Reference count lens
            let ref_lens = CodeLens {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                command: Some(Command {
                    title: if ref_count == 1 {
                        "1 reference".to_string()
                    } else {
                        format!("{} references", ref_count)
                    },
                    command: "hlx.showReferences".to_string(),
                    arguments: Some(vec![
                        serde_json::to_value(uri).unwrap(),
                        serde_json::to_value(position).unwrap(),
                    ]),
                }),
                data: None,
            };

            lenses.push(ref_lens);

            // Run button for main-like functions
            if func_name == "main" || func_name.starts_with("test_") {
                lenses.push(self.create_run_lens(line_idx));
            }

            // Test button for test functions
            if func_name.starts_with("test_") {
                lenses.push(self.create_test_lens(line_idx, func_name));
            }
        }

        lenses
    }

    /// Create run button lens
    fn create_run_lens(&self, line_idx: usize) -> CodeLens {
        CodeLens {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: 0,
                },
                end: Position {
                    line: line_idx as u32,
                    character: 0,
                },
            },
            command: Some(Command {
                title: "▶ Run".to_string(),
                command: "hlx.run".to_string(),
                arguments: None,
            }),
            data: None,
        }
    }

    /// Create test button lens
    fn create_test_lens(&self, line_idx: usize, test_name: &str) -> CodeLens {
        CodeLens {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: 0,
                },
                end: Position {
                    line: line_idx as u32,
                    character: 0,
                },
            },
            command: Some(Command {
                title: "🧪 Run Test".to_string(),
                command: "hlx.runTest".to_string(),
                arguments: Some(vec![
                    serde_json::to_value(test_name).unwrap(),
                ]),
            }),
            data: None,
        }
    }

    /// Create lens for variable declarations
    fn create_variable_lens(&self, uri: &Url, text: &str, line_idx: usize, line: &str) -> Option<CodeLens> {
        // Extract variable name
        let after_let = line.trim()[4..].trim();
        let eq_pos = after_let.find('=')?;
        let var_name = after_let[..eq_pos].trim();

        // Count references
        let position = Position {
            line: line_idx as u32,
            character: (line.find(var_name).unwrap_or(0)) as u32,
        };

        let references = self.symbol_index.find_references(&position, uri, text);
        let ref_count = references.len().saturating_sub(1); // Exclude definition

        // Only show if variable is unused
        if ref_count == 0 {
            return Some(CodeLens {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
                command: Some(Command {
                    title: "⚠️ Unused variable".to_string(),
                    command: "hlx.removeUnused".to_string(),
                    arguments: Some(vec![
                        serde_json::to_value(uri).unwrap(),
                        serde_json::to_value(position).unwrap(),
                    ]),
                }),
                data: None,
            });
        }

        None
    }

    /// Create debug button lens
    #[allow(dead_code)]
    fn create_debug_lens(&self, line_idx: usize) -> CodeLens {
        CodeLens {
            range: Range {
                start: Position {
                    line: line_idx as u32,
                    character: 0,
                },
                end: Position {
                    line: line_idx as u32,
                    character: 0,
                },
            },
            command: Some(Command {
                title: "🐛 Debug".to_string(),
                command: "hlx.debug".to_string(),
                arguments: None,
            }),
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::symbol_index::SymbolIndex;
    use std::sync::Arc;

    #[test]
    fn test_function_lens_creation() {
        let index = Arc::new(SymbolIndex::new());
        let provider = CodeLensProvider::new(index.clone());

        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "fn test() {\n    return 42;\n}";

        let lenses = provider.get_code_lenses(&uri, code);

        assert!(!lenses.is_empty());
        assert!(lenses.iter().any(|l| {
            l.command.as_ref().map(|c| c.title.contains("reference")).unwrap_or(false)
        }));
    }

    #[test]
    fn test_run_lens_for_main() {
        let index = Arc::new(SymbolIndex::new());
        let provider = CodeLensProvider::new(index);

        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "fn main() {\n    print(\"Hello\");\n}";

        let lenses = provider.get_code_lenses(&uri, code);

        assert!(lenses.iter().any(|l| {
            l.command.as_ref().map(|c| c.title.contains("Run")).unwrap_or(false)
        }));
    }

    #[test]
    fn test_test_lens_creation() {
        let index = Arc::new(SymbolIndex::new());
        let provider = CodeLensProvider::new(index);

        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "fn test_addition() {\n    let x = 1 + 1;\n}";

        let lenses = provider.get_code_lenses(&uri, code);

        assert!(lenses.iter().any(|l| {
            l.command.as_ref().map(|c| c.title.contains("Test")).unwrap_or(false)
        }));
    }
}
