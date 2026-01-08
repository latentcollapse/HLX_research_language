//! Constrained Grammar Validator
//!
//! Provides parser-level validation to prevent invalid constructs.
//! Enforces structural rules and provides helpful error messages.

use tower_lsp::lsp_types::*;

/// A grammar violation
#[derive(Debug, Clone)]
pub struct GrammarViolation {
    pub range: Range,
    pub rule_id: String,
    pub message: String,
    pub severity: DiagnosticSeverity,
    pub fix_suggestion: Option<String>,
}

/// Constrained grammar validator
pub struct ConstrainedGrammarValidator {
    /// Strict mode (more restrictive)
    strict_mode: bool,
}

impl ConstrainedGrammarValidator {
    pub fn new(strict_mode: bool) -> Self {
        Self { strict_mode }
    }

    /// Validate a document against grammar constraints
    pub fn validate(&self, text: &str) -> Vec<GrammarViolation> {
        let mut violations = Vec::new();

        for (line_idx, line) in text.lines().enumerate() {
            // 1. Check for balanced braces
            if let Some(violation) = self.check_balanced_braces(line, line_idx) {
                violations.push(violation);
            }

            // 2. Check for semicolon placement
            if let Some(violation) = self.check_semicolon_rules(line, line_idx) {
                violations.push(violation);
            }

            // 3. Check for valid function signatures
            if let Some(violation) = self.check_function_signature(line, line_idx) {
                violations.push(violation);
            }

            // 4. Check for valid contract syntax
            if let Some(violation) = self.check_contract_syntax(line, line_idx) {
                violations.push(violation);
            }

            // 5. Check for valid variable declarations
            if let Some(violation) = self.check_variable_declaration(line, line_idx) {
                violations.push(violation);
            }

            // 6. Check for valid loop syntax
            if let Some(violation) = self.check_loop_syntax(line, line_idx) {
                violations.push(violation);
            }

            // 7. Check for forbidden constructs (in strict mode)
            if self.strict_mode {
                violations.extend(self.check_forbidden_constructs(line, line_idx));
            }
        }

        violations
    }

    /// Check for balanced braces
    fn check_balanced_braces(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        let mut balance = 0;
        let mut first_unmatched = None;

        for (i, ch) in line.chars().enumerate() {
            match ch {
                '{' => balance += 1,
                '}' => {
                    balance -= 1;
                    if balance < 0 && first_unmatched.is_none() {
                        first_unmatched = Some(i);
                    }
                }
                _ => {}
            }
        }

        if let Some(pos) = first_unmatched {
            return Some(GrammarViolation {
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: pos as u32,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: (pos + 1) as u32,
                    },
                },
                rule_id: "balanced-braces".to_string(),
                message: "Unmatched closing brace '}'".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Remove this brace or add matching opening brace".to_string()),
            });
        }

        None
    }

    /// Check semicolon placement rules
    fn check_semicolon_rules(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        let trimmed = line.trim();

        // Statements that should end with semicolon
        let needs_semicolon = trimmed.starts_with("let ") ||
                              trimmed.starts_with("return ") ||
                              (trimmed.contains("@") && trimmed.contains("}") && !trimmed.ends_with("{"));

        // Lines that should NOT end with semicolon
        let no_semicolon = trimmed.ends_with("{") ||
                           trimmed.starts_with("fn ") ||
                           trimmed.starts_with("if ") ||
                           trimmed.starts_with("loop ") ||
                           trimmed.starts_with("//") ||
                           trimmed.is_empty();

        if needs_semicolon && !trimmed.ends_with(';') && !no_semicolon {
            return Some(GrammarViolation {
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
                rule_id: "missing-semicolon".to_string(),
                message: "Statement must end with semicolon".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Add ';' at the end of this line".to_string()),
            });
        }

        None
    }

    /// Check function signature format
    fn check_function_signature(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        let trimmed = line.trim();

        if !trimmed.starts_with("fn ") {
            return None;
        }

        // Must have format: fn name(args) {
        let has_parens = trimmed.contains('(') && trimmed.contains(')');
        let has_brace = trimmed.contains('{');

        if !has_parens {
            return Some(GrammarViolation {
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
                rule_id: "invalid-function-signature".to_string(),
                message: "Function declaration must include parentheses for parameters".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Add '()' after function name".to_string()),
            });
        }

        if has_parens && !has_brace {
            return Some(GrammarViolation {
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
                rule_id: "invalid-function-signature".to_string(),
                message: "Function declaration must include opening brace '{'".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Add '{' at the end of this line".to_string()),
            });
        }

        None
    }

    /// Check contract syntax
    fn check_contract_syntax(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        if !line.contains('@') {
            return None;
        }

        // Find all @ symbols
        for (i, ch) in line.chars().enumerate() {
            if ch == '@' {
                // Check if followed by digits
                let rest = &line[i + 1..];
                let has_digits = rest.chars().next().map(|c| c.is_numeric()).unwrap_or(false);

                if !has_digits {
                    return Some(GrammarViolation {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: i as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (i + 1) as u32,
                            },
                        },
                        rule_id: "invalid-contract-syntax".to_string(),
                        message: "'@' must be followed by a contract ID (digits)".to_string(),
                        severity: DiagnosticSeverity::ERROR,
                        fix_suggestion: Some("Use '@<number>' format for contract invocation".to_string()),
                    });
                }
            }
        }

        None
    }

    /// Check variable declaration format
    fn check_variable_declaration(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        let trimmed = line.trim();

        if !trimmed.starts_with("let ") {
            return None;
        }

        // Must have format: let VAR = VALUE;
        if !trimmed.contains('=') {
            return Some(GrammarViolation {
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
                rule_id: "invalid-let-statement".to_string(),
                message: "Variable declaration must include assignment with '='".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Add '= value' after variable name".to_string()),
            });
        }

        None
    }

    /// Check loop syntax
    fn check_loop_syntax(&self, line: &str, line_idx: usize) -> Option<GrammarViolation> {
        let trimmed = line.trim();

        if !trimmed.starts_with("loop ") {
            return None;
        }

        // Must have format: loop (condition, bound) {
        if !trimmed.contains('(') || !trimmed.contains(')') {
            return Some(GrammarViolation {
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
                rule_id: "invalid-loop-syntax".to_string(),
                message: "Loop must include condition and bound in parentheses".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Use 'loop (condition, DEFAULT_MAX_ITER) {'".to_string()),
            });
        }

        // Check for comma (separating condition and bound)
        let has_comma = trimmed.contains(',');
        if !has_comma {
            return Some(GrammarViolation {
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
                rule_id: "invalid-loop-syntax".to_string(),
                message: "Loop must specify both condition and bound separated by comma".to_string(),
                severity: DiagnosticSeverity::ERROR,
                fix_suggestion: Some("Add ', DEFAULT_MAX_ITER' before closing parenthesis".to_string()),
            });
        }

        None
    }

    /// Check for forbidden constructs in strict mode
    fn check_forbidden_constructs(&self, line: &str, line_idx: usize) -> Vec<GrammarViolation> {
        let mut violations = Vec::new();

        // In strict mode, forbid certain patterns
        let forbidden = vec![
            ("while ", "Use 'loop' instead of 'while'"),
            ("for ", "Use 'loop' instead of 'for'"),
            ("var ", "Use 'let' instead of 'var'"),
            ("goto ", "'goto' is not allowed in HLX"),
        ];

        for (keyword, message) in forbidden {
            if line.contains(keyword) {
                if let Some(pos) = line.find(keyword) {
                    violations.push(GrammarViolation {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: pos as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (pos + keyword.len()) as u32,
                            },
                        },
                        rule_id: "forbidden-construct".to_string(),
                        message: message.to_string(),
                        severity: DiagnosticSeverity::ERROR,
                        fix_suggestion: Some("Rewrite using HLX-approved constructs".to_string()),
                    });
                }
            }
        }

        violations
    }

    /// Create a diagnostic from a grammar violation
    pub fn create_diagnostic(&self, violation: &GrammarViolation) -> Diagnostic {
        let mut message = violation.message.clone();
        if let Some(ref suggestion) = violation.fix_suggestion {
            message.push_str(&format!("\n💡 {}", suggestion));
        }

        Diagnostic {
            range: violation.range,
            severity: Some(violation.severity),
            code: Some(NumberOrString::String(violation.rule_id.clone())),
            source: Some("hlx-grammar".to_string()),
            message,
            related_information: None,
            tags: None,
            data: None,
            code_description: None,
        }
    }
}

impl Default for ConstrainedGrammarValidator {
    fn default() -> Self {
        Self::new(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_balanced_braces() {
        let validator = ConstrainedGrammarValidator::new(false);

        let violations = validator.validate("{ code }");
        assert!(violations.is_empty());

        let violations = validator.validate("} unmatched");
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].rule_id, "balanced-braces");
    }

    #[test]
    fn test_semicolon_rules() {
        let validator = ConstrainedGrammarValidator::new(false);

        let violations = validator.validate("let x = 42");
        assert!(!violations.is_empty());
        assert!(violations.iter().any(|v| v.rule_id == "missing-semicolon"));

        let violations = validator.validate("let x = 42;");
        assert!(!violations.iter().any(|v| v.rule_id == "missing-semicolon"));
    }

    #[test]
    fn test_function_signature() {
        let validator = ConstrainedGrammarValidator::new(false);

        let violations = validator.validate("fn test");
        assert!(!violations.is_empty());
        assert_eq!(violations[0].rule_id, "invalid-function-signature");

        let violations = validator.validate("fn test() {");
        assert!(!violations.iter().any(|v| v.rule_id == "invalid-function-signature"));
    }

    #[test]
    fn test_strict_mode() {
        let validator = ConstrainedGrammarValidator::new(true);

        let violations = validator.validate("while (true) {");
        assert!(!violations.is_empty());
        assert_eq!(violations[0].rule_id, "forbidden-construct");
    }

    #[test]
    fn test_loop_syntax() {
        let validator = ConstrainedGrammarValidator::new(false);

        let violations = validator.validate("loop (i < 10) {");
        assert!(!violations.is_empty());
        assert_eq!(violations[0].rule_id, "invalid-loop-syntax");

        let violations = validator.validate("loop (i < 10, DEFAULT_MAX_ITER) {");
        assert!(!violations.iter().any(|v| v.rule_id == "invalid-loop-syntax"));
    }
}
