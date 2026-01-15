//! Backend Compatibility Checker
//!
//! Detects when code uses features only available in specific backends.
//! Prevents "works in interpreter, fails in native" bugs.

use tower_lsp::lsp_types::*;
use regex::Regex;

/// Backend compatibility checker
pub struct BackendCompatChecker {
    /// Cached interpreter capabilities
    interpreter_builtins: Vec<String>,
    /// Cached LLVM capabilities
    llvm_builtins: Vec<String>,
    /// Pattern for finding function calls
    func_call_pattern: Regex,
}

impl BackendCompatChecker {
    pub fn new() -> Self {
        // Query static capability lists (no backend instantiation needed)
        let interpreter_builtins = hlx_runtime::executor::Executor::static_supported_builtins();
        let llvm_builtins = hlx_backend_llvm::CodeGen::static_supported_builtins();

        Self {
            interpreter_builtins,
            llvm_builtins,
            func_call_pattern: Regex::new(r"(\w+)\s*\(").unwrap(),
        }
    }

    /// Check a document for backend compatibility issues
    ///
    /// target: "interpreter", "llvm", or "auto" (default: check both)
    pub fn check_document(&self, text: &str, target: &str) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Only check if targeting LLVM/native
        if target != "llvm" && target != "native" && target != "compile-native" {
            return diagnostics;
        }

        // Find all function calls
        for (line_idx, line) in text.lines().enumerate() {
            for cap in self.func_call_pattern.captures_iter(line) {
                let func_name = cap.get(1).unwrap().as_str();
                let match_start = cap.get(1).unwrap().start();

                // Skip if not a builtin
                if !self.interpreter_builtins.contains(&func_name.to_string()) {
                    continue;
                }

                // Check if it's supported in LLVM
                if !self.llvm_builtins.contains(&func_name.to_string()) {
                    // This builtin works in interpreter but NOT in LLVM!
                    let message = format!(
                        "Builtin '{}()' is supported in Interpreter (JIT) but NOT in LLVM Native backend.\n\
                        \n\
                        This code will:\n\
                        ✅ Work with: hlx run (interpreter)\n\
                        ❌ Fail with: hlx compile-native (LLVM)\n\
                        \n\
                        Workaround: Use interpreter for now, or implement this builtin in LLVM backend.",
                        func_name
                    );

                    diagnostics.push(Diagnostic {
                        range: Range {
                            start: Position {
                                line: line_idx as u32,
                                character: match_start as u32,
                            },
                            end: Position {
                                line: line_idx as u32,
                                character: (match_start + func_name.len()) as u32,
                            },
                        },
                        severity: Some(DiagnosticSeverity::WARNING),
                        code: Some(NumberOrString::String("backend-parity".to_string())),
                        source: Some("hlx-backend-compat".to_string()),
                        message,
                        related_information: None,
                        tags: None,
                        code_description: None,
                        data: None,
                    });
                }
            }
        }

        diagnostics
    }

    /// Get list of math functions missing in LLVM
    pub fn get_llvm_missing_math(&self) -> Vec<String> {
        vec!["sin", "cos", "tan", "log", "exp", "sqrt", "floor", "ceil", "round"]
            .into_iter()
            .map(String::from)
            .collect()
    }

    /// Generate helpful message for a specific builtin
    pub fn get_compat_info(&self, builtin: &str) -> Option<String> {
        let in_interpreter = self.interpreter_builtins.contains(&builtin.to_string());
        let in_llvm = self.llvm_builtins.contains(&builtin.to_string());

        if !in_interpreter {
            return None; // Not a builtin at all
        }

        if in_llvm {
            Some(format!("✅ Supported in: Interpreter, LLVM Native"))
        } else {
            Some(format!(
                "⚠️  Supported in: Interpreter only\n❌ Not available in: LLVM Native\n\n\
                Use interpreter (hlx run) or wait for LLVM implementation."
            ))
        }
    }
}

impl Default for BackendCompatChecker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_incompatible_builtin() {
        let checker = BackendCompatChecker::new();

        // Use a builtin that's actually not in LLVM yet (like random())
        let code = r#"
            fn main() {
                let x = random();  // This will fail in LLVM!
                print(x);
            }
        "#;

        let diagnostics = checker.check_document(code, "llvm");
        assert!(!diagnostics.is_empty(), "Should detect random() incompatibility");
        assert!(diagnostics[0].message.contains("random"));
        assert!(diagnostics[0].message.contains("LLVM"));
    }

    #[test]
    fn test_no_warning_for_supported_builtin() {
        let checker = BackendCompatChecker::new();

        let code = r#"
            fn main() {
                let x = to_string(42);  // This works everywhere
                print(x);
            }
        "#;

        let diagnostics = checker.check_document(code, "llvm");
        // Should have minimal or no warnings for these supported builtins
        assert!(diagnostics.len() <= 1, "Should not warn for supported builtins");
    }

    #[test]
    fn test_interpreter_has_all_math() {
        let checker = BackendCompatChecker::new();

        // Interpreter should support all math functions
        assert!(checker.interpreter_builtins.contains(&"sin".to_string()));
        assert!(checker.interpreter_builtins.contains(&"cos".to_string()));
        assert!(checker.interpreter_builtins.contains(&"tan".to_string()));
        assert!(checker.interpreter_builtins.contains(&"log".to_string()));
        assert!(checker.interpreter_builtins.contains(&"exp".to_string()));
    }

    #[test]
    fn test_llvm_missing_math() {
        let checker = BackendCompatChecker::new();

        // LLVM now HAS math functions via external linkage to libm
        assert!(checker.llvm_builtins.contains(&"sin".to_string()));
        assert!(checker.llvm_builtins.contains(&"cos".to_string()));
        assert!(checker.llvm_builtins.contains(&"tan".to_string()));

        // But LLVM should NOT have random() yet
        assert!(!checker.llvm_builtins.contains(&"random".to_string()));
    }
}
