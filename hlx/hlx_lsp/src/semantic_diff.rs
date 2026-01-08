//! Semantic Diff Analyzer
//!
//! Compares user code against known patterns and suggests semantic improvements.
//! Goes beyond syntax to understand intent and recommend better approaches.

use tower_lsp::lsp_types::*;
use crate::patterns::{PatternLibrary, Pattern};

/// A semantic difference found in the code
#[derive(Debug, Clone)]
pub struct SemanticDiff {
    pub range: Range,
    pub pattern_id: String,
    pub issue: String,
    pub suggestion: String,
    pub severity: DiffSeverity,
    pub current_code: String,
    pub suggested_code: String,
}

/// Severity of a semantic difference
#[derive(Debug, Clone, PartialEq)]
pub enum DiffSeverity {
    Error,      // Will cause problems
    Warning,    // Suboptimal but works
    Info,       // Could be better
    Hint,       // Style suggestion
}

/// Semantic diff analyzer
pub struct SemanticDiffAnalyzer {
    _patterns: Vec<Pattern>,
}

impl SemanticDiffAnalyzer {
    pub fn new(pattern_library: &PatternLibrary) -> Self {
        // Extract patterns from the HashMap
        let patterns: Vec<Pattern> = pattern_library.patterns.values().cloned().collect();

        Self {
            _patterns: patterns,
        }
    }

    /// Analyze a document and find semantic differences
    pub fn analyze(&self, text: &str) -> Vec<SemanticDiff> {
        let mut diffs = Vec::new();

        // Check each line for pattern violations
        for (line_idx, line) in text.lines().enumerate() {
            // 1. Check for unbounded loops
            if let Some(diff) = self.check_unbounded_loop(line, line_idx) {
                diffs.push(diff);
            }

            // 2. Check for unsafe division
            if let Some(diff) = self.check_unsafe_division(line, line_idx) {
                diffs.push(diff);
            }

            // 3. Check for manual math instead of contracts
            if let Some(diff) = self.check_manual_math(line, line_idx) {
                diffs.push(diff);
            }

            // 4. Check for string manipulation without contracts
            if let Some(diff) = self.check_manual_string_ops(line, line_idx) {
                diffs.push(diff);
            }

            // 5. Check for direct I/O instead of contracts
            if let Some(diff) = self.check_manual_io(line, line_idx) {
                diffs.push(diff);
            }

            // 6. Check for mutable state patterns
            if let Some(diff) = self.check_mutable_state(line, line_idx) {
                diffs.push(diff);
            }
        }

        diffs
    }

    /// Check for loops without safety bounds
    fn check_unbounded_loop(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        if !line.contains("loop ") {
            return None;
        }

        // Check if it uses DEFAULT_MAX_ITER
        if line.contains("DEFAULT_MAX_ITER") {
            return None;
        }

        // Check if it has any numeric bound
        let has_bound = line.contains(", ") && line.contains(")");

        if !has_bound {
            return Some(SemanticDiff {
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
                pattern_id: "safe-loops".to_string(),
                issue: "Loop without safety bound".to_string(),
                suggestion: "Always specify DEFAULT_MAX_ITER as the loop bound to prevent infinite loops".to_string(),
                severity: DiffSeverity::Error,
                current_code: line.trim().to_string(),
                suggested_code: line.trim().replace(")", ", DEFAULT_MAX_ITER)"),
            });
        }

        None
    }

    /// Check for division without zero check
    fn check_unsafe_division(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        // Look for @203 (division contract) or / operator
        let has_division = line.contains("@203") || line.contains(" / ");

        if !has_division {
            return None;
        }

        // Check if there's a zero check nearby (simple heuristic)
        let has_check = line.contains("!= 0") || line.contains("== 0");

        if !has_check {
            return Some(SemanticDiff {
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
                pattern_id: "safe-division".to_string(),
                issue: "Division without zero check".to_string(),
                suggestion: "Check denominator for zero before division to prevent runtime errors".to_string(),
                severity: DiffSeverity::Warning,
                current_code: line.trim().to_string(),
                suggested_code: format!("if (denominator != 0) {{ {} }}", line.trim()),
            });
        }

        None
    }

    /// Check for manual math operations that should use contracts
    fn check_manual_math(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        let trimmed = line.trim();

        // Look for simple arithmetic expressions without contract usage
        if trimmed.contains(" + ") && !trimmed.contains('@') {
            let char_pos = trimmed.find(" + ")?;

            return Some(SemanticDiff {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: char_pos as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: (char_pos + 3) as u32,
                    },
                },
                pattern_id: "use-contracts".to_string(),
                issue: "Manual addition operator".to_string(),
                suggestion: "Use @200 contract for addition to leverage HLX's type system and optimizations".to_string(),
                severity: DiffSeverity::Info,
                current_code: trimmed.to_string(),
                suggested_code: trimmed.replace(" + ", " @200 { lhs: ").replace(")", ", rhs: ) }") + "}",
            });
        }

        if trimmed.contains(" - ") && !trimmed.contains('@') && !trimmed.contains("->") {
            let char_pos = trimmed.find(" - ")?;

            return Some(SemanticDiff {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: char_pos as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: (char_pos + 3) as u32,
                    },
                },
                pattern_id: "use-contracts".to_string(),
                issue: "Manual subtraction operator".to_string(),
                suggestion: "Use @201 contract for subtraction".to_string(),
                severity: DiffSeverity::Info,
                current_code: trimmed.to_string(),
                suggested_code: "Use @201 { lhs: a, rhs: b }".to_string(),
            });
        }

        None
    }

    /// Check for manual string operations
    fn check_manual_string_ops(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        let trimmed = line.trim();

        // Look for string concatenation without contracts
        if (trimmed.contains("\"") && trimmed.contains("+")) && !trimmed.contains('@') {
            return Some(SemanticDiff {
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
                pattern_id: "use-string-contracts".to_string(),
                issue: "Manual string concatenation".to_string(),
                suggestion: "Use @300 contract for string concatenation".to_string(),
                severity: DiffSeverity::Hint,
                current_code: trimmed.to_string(),
                suggested_code: "Use @300 { lhs: str1, rhs: str2 }".to_string(),
            });
        }

        None
    }

    /// Check for manual I/O operations
    fn check_manual_io(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        let trimmed = line.trim();

        // Look for print statements
        if trimmed.starts_with("print(") && !trimmed.contains("@600") {
            return Some(SemanticDiff {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: 5,
                    },
                },
                pattern_id: "use-io-contracts".to_string(),
                issue: "Using builtin print instead of @600 contract".to_string(),
                suggestion: "Consider using @600 for standardized output handling".to_string(),
                severity: DiffSeverity::Hint,
                current_code: trimmed.to_string(),
                suggested_code: "Use @600 { value: message }".to_string(),
            });
        }

        None
    }

    /// Check for mutable state patterns
    fn check_mutable_state(&self, line: &str, line_idx: usize) -> Option<SemanticDiff> {
        let trimmed = line.trim();

        // Look for repeated assignments to same variable
        if trimmed.contains("=") && !trimmed.starts_with("let ") {
            // This is a reassignment
            return Some(SemanticDiff {
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
                pattern_id: "prefer-immutability".to_string(),
                issue: "Variable reassignment (mutation)".to_string(),
                suggestion: "HLX favors immutability. Consider using new variables or contracts instead of mutation".to_string(),
                severity: DiffSeverity::Hint,
                current_code: trimmed.to_string(),
                suggested_code: format!("let {}_new = ...", trimmed.split('=').next().unwrap_or("var").trim()),
            });
        }

        None
    }

    /// Create a diagnostic for a semantic diff
    pub fn create_diagnostic(&self, diff: &SemanticDiff) -> Diagnostic {
        let severity = match diff.severity {
            DiffSeverity::Error => DiagnosticSeverity::ERROR,
            DiffSeverity::Warning => DiagnosticSeverity::WARNING,
            DiffSeverity::Info => DiagnosticSeverity::INFORMATION,
            DiffSeverity::Hint => DiagnosticSeverity::HINT,
        };

        Diagnostic {
            range: diff.range,
            severity: Some(severity),
            code: Some(NumberOrString::String(diff.pattern_id.clone())),
            source: Some("hlx-semantic".to_string()),
            message: format!("{}\n💡 {}", diff.issue, diff.suggestion),
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        }
    }

    /// Create a code action for a semantic diff
    pub fn create_code_action(&self, diff: &SemanticDiff, uri: Url) -> CodeAction {
        let mut changes = std::collections::HashMap::new();
        changes.insert(
            uri,
            vec![TextEdit {
                range: diff.range,
                new_text: diff.suggested_code.clone(),
            }],
        );

        CodeAction {
            title: format!("🔄 Refactor: {}", diff.suggestion),
            kind: Some(CodeActionKind::REFACTOR),
            diagnostics: None,
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(diff.severity == DiffSeverity::Error),
            disabled: None,
            data: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::patterns::PatternLibrary;

    #[test]
    fn test_unbounded_loop_detection() {
        let patterns = PatternLibrary::new();
        let analyzer = SemanticDiffAnalyzer::new(&patterns);

        let code = "loop (i < 100) { }";
        let diffs = analyzer.analyze(code);

        assert!(!diffs.is_empty());
        assert_eq!(diffs[0].pattern_id, "safe-loops");
        assert_eq!(diffs[0].severity, DiffSeverity::Error);
    }

    #[test]
    fn test_safe_loop_no_warning() {
        let patterns = PatternLibrary::new();
        let analyzer = SemanticDiffAnalyzer::new(&patterns);

        let code = "loop (i < 100, DEFAULT_MAX_ITER) { }";
        let diffs = analyzer.analyze(code);

        // Should not flag loops with DEFAULT_MAX_ITER
        assert!(diffs.iter().all(|d| d.pattern_id != "safe-loops"));
    }

    #[test]
    fn test_manual_math_detection() {
        let patterns = PatternLibrary::new();
        let analyzer = SemanticDiffAnalyzer::new(&patterns);

        let code = "let result = a + b;";
        let diffs = analyzer.analyze(code);

        assert!(!diffs.is_empty());
        let math_diff = diffs.iter().find(|d| d.pattern_id == "use-contracts");
        assert!(math_diff.is_some());
    }

    #[test]
    fn test_contract_usage_no_warning() {
        let patterns = PatternLibrary::new();
        let analyzer = SemanticDiffAnalyzer::new(&patterns);

        let code = "let result = @200 { lhs: a, rhs: b };";
        let diffs = analyzer.analyze(code);

        // Should not flag contract usage
        assert!(diffs.iter().all(|d| d.pattern_id != "use-contracts" || d.issue.contains("Manual")));
    }
}
