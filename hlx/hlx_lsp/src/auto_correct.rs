//! Auto-Correction for Common HLX Mistakes
//!
//! Detects and automatically fixes common errors that AI models
//! and humans make when writing HLX code.

use tower_lsp::lsp_types::*;

/// A correctable mistake with suggested fix
#[derive(Debug, Clone)]
pub struct AutoCorrection {
    pub range: Range,
    pub original: String,
    pub corrected: String,
    pub reason: String,
    pub confidence: f32,
}

/// Auto-correction analyzer
pub struct AutoCorrector {
    // Common field name typos
    field_corrections: Vec<(String, String, String)>, // (wrong, correct, context)
}

impl AutoCorrector {
    pub fn new() -> Self {
        let mut field_corrections = Vec::new();

        // Math operations
        field_corrections.push(("left".to_string(), "lhs".to_string(), "math operation".to_string()));
        field_corrections.push(("right".to_string(), "rhs".to_string(), "math operation".to_string()));
        field_corrections.push(("a".to_string(), "lhs".to_string(), "@200-203".to_string()));
        field_corrections.push(("b".to_string(), "rhs".to_string(), "@200-203".to_string()));

        // Matrix operations
        field_corrections.push(("C".to_string(), "A or B".to_string(), "@906 GEMM".to_string()));
        field_corrections.push(("matrix".to_string(), "A".to_string(), "@906 first operand".to_string()));
        field_corrections.push(("matrix1".to_string(), "A".to_string(), "@906 first operand".to_string()));
        field_corrections.push(("matrix2".to_string(), "B".to_string(), "@906 second operand".to_string()));

        // Array operations
        field_corrections.push(("arr".to_string(), "@0".to_string(), "@18 array constructor".to_string()));
        field_corrections.push(("array".to_string(), "@0".to_string(), "@400-403".to_string()));
        field_corrections.push(("idx".to_string(), "@1".to_string(), "@401 array index".to_string()));
        field_corrections.push(("index".to_string(), "@1".to_string(), "@401 array index".to_string()));

        // Common spelling mistakes
        field_corrections.push(("lenght".to_string(), "length".to_string(), "spelling".to_string()));
        field_corrections.push(("seperator".to_string(), "separator".to_string(), "spelling".to_string()));
        field_corrections.push(("recieve".to_string(), "receive".to_string(), "spelling".to_string()));

        Self {
            field_corrections,
        }
    }

    /// Analyze a document and find all auto-correctable mistakes
    pub fn analyze_document(&self, text: &str) -> Vec<AutoCorrection> {
        let mut corrections = Vec::new();

        // Check each line
        for (line_idx, line) in text.lines().enumerate() {
            // Check for field name typos in contracts
            if line.contains("@") && line.contains("{") {
                corrections.extend(self.check_contract_fields(line, line_idx));
            }

            // Check for missing semicolons
            if let Some(correction) = self.check_missing_semicolon(line, line_idx) {
                corrections.push(correction);
            }

            // Check for common keyword typos
            corrections.extend(self.check_keyword_typos(line, line_idx));

            // Check for incorrect loop bounds
            if let Some(correction) = self.check_loop_bound(line, line_idx) {
                corrections.push(correction);
            }
        }

        corrections
    }

    /// Check for field name typos in contract usage
    fn check_contract_fields(&self, line: &str, line_idx: usize) -> Vec<AutoCorrection> {
        let mut corrections = Vec::new();

        // Extract contract usage pattern: @ID { field: value, ... }
        if let Some(at_pos) = line.find('@') {
            if let Some(brace_start) = line[at_pos..].find('{') {
                if let Some(brace_end) = line[at_pos + brace_start..].find('}') {
                    let inside_braces = &line[at_pos + brace_start + 1..at_pos + brace_start + brace_end];

                    // Split by comma to get fields
                    for field_part in inside_braces.split(',') {
                        if let Some(colon_pos) = field_part.find(':') {
                            let field_name = field_part[..colon_pos].trim();

                            // Check against known corrections
                            for (wrong, correct, context) in &self.field_corrections {
                                if field_name == wrong {
                                    let field_start = line.find(field_name).unwrap_or(0);

                                    corrections.push(AutoCorrection {
                                        range: Range {
                                            start: Position {
                                                line: line_idx as u32,
                                                character: field_start as u32,
                                            },
                                            end: Position {
                                                line: line_idx as u32,
                                                character: (field_start + field_name.len()) as u32,
                                            },
                                        },
                                        original: wrong.clone(),
                                        corrected: correct.clone(),
                                        reason: format!("Common mistake: '{}' should be '{}' for {}", wrong, correct, context),
                                        confidence: 0.9,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }

        corrections
    }

    /// Check for missing semicolon at end of statement
    fn check_missing_semicolon(&self, line: &str, line_idx: usize) -> Option<AutoCorrection> {
        let trimmed = line.trim();

        // Lines that should end with semicolon
        let needs_semicolon = trimmed.starts_with("let ") ||
                              trimmed.starts_with("return ") ||
                              (trimmed.contains("@") && trimmed.contains("{") && trimmed.contains("}"));

        if needs_semicolon && !trimmed.ends_with(';') && !trimmed.ends_with('{') {
            // Check if it's not inside a function declaration or other block
            if !trimmed.contains("fn ") && !trimmed.contains("if ") && !trimmed.contains("loop ") {
                return Some(AutoCorrection {
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: line.len() as u32,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: line.len() as u32,
                        },
                    },
                    original: String::new(),
                    corrected: ";".to_string(),
                    reason: "Statement should end with semicolon".to_string(),
                    confidence: 0.85,
                });
            }
        }

        None
    }

    /// Check for common keyword typos
    fn check_keyword_typos(&self, line: &str, line_idx: usize) -> Vec<AutoCorrection> {
        let mut corrections = Vec::new();

        let typos = vec![
            ("fucntion", "fn"),
            ("funciton", "fn"),
            ("functoin", "fn"),
            ("retrun", "return"),
            ("reutrn", "return"),
            ("els", "else"),
            ("esle", "else"),
            ("ture", "true"),
            ("flase", "false"),
            ("fasle", "false"),
        ];

        for (wrong, correct) in typos {
            if let Some(pos) = line.find(wrong) {
                // Make sure it's a whole word (not part of a string or comment)
                let before_ok = pos == 0 || !line.chars().nth(pos - 1).unwrap().is_alphanumeric();
                let after_ok = pos + wrong.len() >= line.len() ||
                              !line.chars().nth(pos + wrong.len()).unwrap().is_alphanumeric();

                if before_ok && after_ok {
                    corrections.push(AutoCorrection {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: pos as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (pos + wrong.len()) as u32,
                            },
                        },
                        original: wrong.to_string(),
                        corrected: correct.to_string(),
                        reason: format!("Typo: '{}' should be '{}'", wrong, correct),
                        confidence: 0.95,
                    });
                }
            }
        }

        corrections
    }

    /// Check for incorrect loop bounds (should use DEFAULT_MAX_ITER)
    fn check_loop_bound(&self, line: &str, line_idx: usize) -> Option<AutoCorrection> {
        if line.contains("loop ") && line.contains("(") {
            // Check if it has a numeric literal instead of DEFAULT_MAX_ITER
            if let Some(comma_pos) = line.rfind(',') {
                if let Some(paren_pos) = line[comma_pos..].find(')') {
                    let bound = line[comma_pos + 1..comma_pos + paren_pos].trim();

                    // If it's a number, suggest DEFAULT_MAX_ITER
                    if bound.chars().all(|c| c.is_numeric() || c == '_') {
                        let bound_start = comma_pos + 1 + line[comma_pos + 1..].find(bound).unwrap_or(0);

                        return Some(AutoCorrection {
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: bound_start as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (bound_start + bound.len()) as u32,
                                },
                            },
                            original: bound.to_string(),
                            corrected: "DEFAULT_MAX_ITER".to_string(),
                            reason: "Loop bounds should use DEFAULT_MAX_ITER for safety".to_string(),
                            confidence: 0.7,
                        });
                    }
                }
            }
        }

        None
    }

    /// Create a diagnostic for an auto-correction
    pub fn create_diagnostic(&self, correction: &AutoCorrection) -> Diagnostic {
        Diagnostic {
            range: correction.range,
            severity: Some(DiagnosticSeverity::WARNING),
            code: Some(NumberOrString::String("auto-correct".to_string())),
            source: Some("hlx-autocorrect".to_string()),
            message: format!("{}\n💡 Quick fix available: '{}' → '{}'",
                correction.reason,
                correction.original,
                correction.corrected
            ),
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        }
    }

    /// Create a code action to apply an auto-correction
    pub fn create_code_action(
        &self,
        correction: &AutoCorrection,
        uri: Url,
    ) -> CodeAction {
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            uri,
            vec![TextEdit {
                range: correction.range,
                new_text: correction.corrected.clone(),
            }],
        );

        CodeAction {
            title: format!("🔧 Fix: '{}' → '{}'", correction.original, correction.corrected),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(correction.confidence > 0.8),
            disabled: None,
            data: None,
        }
    }
}

impl Default for AutoCorrector {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_field_correction() {
        let corrector = AutoCorrector::new();
        let code = "let result = @200 { left: 5, right: 3 };";

        let corrections = corrector.analyze_document(code);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.original == "left" && c.corrected == "lhs"));
        assert!(corrections.iter().any(|c| c.original == "right" && c.corrected == "rhs"));
    }

    #[test]
    fn test_missing_semicolon() {
        let corrector = AutoCorrector::new();
        let code = "let x = 42";

        let corrections = corrector.analyze_document(code);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.corrected == ";"));
    }

    #[test]
    fn test_keyword_typo() {
        let corrector = AutoCorrector::new();
        let code = "retrun 42;";

        let corrections = corrector.analyze_document(code);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.original == "retrun" && c.corrected == "return"));
    }

    #[test]
    fn test_loop_bound() {
        let corrector = AutoCorrector::new();
        let code = "loop (i < 10, 1000) { }";

        let corrections = corrector.analyze_document(code);
        assert!(!corrections.is_empty());
        assert!(corrections.iter().any(|c| c.corrected == "DEFAULT_MAX_ITER"));
    }
}
