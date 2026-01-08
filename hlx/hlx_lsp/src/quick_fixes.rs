//! Quick Fix System - Automatic Error Repairs
//!
//! Generates code actions to automatically fix common errors.

use tower_lsp::lsp_types::*;
use std::collections::HashMap;

/// Quick fix generator
pub struct QuickFixGenerator {
    /// Maps error codes to fix generators
    fix_generators: HashMap<String, fn(&QuickFixContext) -> Vec<CodeAction>>,
}

/// Context for generating quick fixes
pub struct QuickFixContext {
    pub uri: Url,
    pub diagnostic: Diagnostic,
    pub source_text: String,
}

impl QuickFixGenerator {
    pub fn new() -> Self {
        let mut generator = Self {
            fix_generators: HashMap::new(),
        };

        // Register fix generators
        generator.register("uninitialized-variable", Self::fix_uninitialized_variable);
        generator.register("type-error", Self::fix_type_error);
        generator.register("missing-semicolon", Self::fix_missing_semicolon);

        generator
    }

    /// Register a fix generator for an error code
    fn register(&mut self, code: &str, generator: fn(&QuickFixContext) -> Vec<CodeAction>) {
        self.fix_generators.insert(code.to_string(), generator);
    }

    /// Generate quick fixes for a diagnostic
    pub fn generate_fixes(&self, ctx: &QuickFixContext) -> Vec<CodeAction> {
        let code = match &ctx.diagnostic.code {
            Some(NumberOrString::String(s)) => s.as_str(),
            _ => return vec![],
        };

        if let Some(generator) = self.fix_generators.get(code) {
            generator(ctx)
        } else {
            vec![]
        }
    }

    /// Fix uninitialized variable errors
    fn fix_uninitialized_variable(ctx: &QuickFixContext) -> Vec<CodeAction> {
        let mut fixes = Vec::new();

        // Extract variable name from message
        let message = &ctx.diagnostic.message;
        if let Some(start) = message.find("Variable '") {
            if let Some(end) = message[start + 10..].find('\'') {
                let var_name = &message[start + 10..start + 10 + end];

                // Fix 1: Initialize to default value (0)
                let line = ctx.diagnostic.range.start.line as usize;
                let lines: Vec<&str> = ctx.source_text.lines().collect();

                if line < lines.len() {
                    let current_line = lines[line];

                    // Find the variable declaration
                    if current_line.contains(&format!("let {}", var_name)) {
                        // Change `let x;` to `let x = 0;`
                        let new_line = if current_line.contains(&format!("let {};", var_name)) {
                            current_line.replace(&format!("let {};", var_name), &format!("let {} = 0;", var_name))
                        } else if current_line.contains(&format!("let {}", var_name)) && !current_line.contains('=') {
                            current_line.replace(&format!("let {}", var_name), &format!("let {} = 0", var_name))
                        } else {
                            current_line.to_string()
                        };

                        let edit = WorkspaceEdit {
                            changes: Some({
                                let mut changes = HashMap::new();
                                changes.insert(
                                    ctx.uri.clone(),
                                    vec![TextEdit {
                                        range: Range {
                                            start: Position { line: line as u32, character: 0 },
                                            end: Position { line: line as u32, character: current_line.len() as u32 },
                                        },
                                        new_text: new_line,
                                    }],
                                );
                                changes
                            }),
                            document_changes: None,
                            change_annotations: None,
                        };

                        fixes.push(CodeAction {
                            title: format!("Initialize '{}' to 0", var_name),
                            kind: Some(CodeActionKind::QUICKFIX),
                            diagnostics: Some(vec![ctx.diagnostic.clone()]),
                            edit: Some(edit),
                            command: None,
                            is_preferred: Some(true),
                            disabled: None,
                            data: None,
                        });
                    }
                }
            }
        }

        fixes
    }

    /// Fix type errors
    fn fix_type_error(ctx: &QuickFixContext) -> Vec<CodeAction> {
        let mut fixes = Vec::new();
        let message = &ctx.diagnostic.message;

        // Check if it's an Int->Float conversion issue
        if message.contains("Expected type Float") && message.contains("got Int") {
            // Extract the line with the error
            let line = ctx.diagnostic.range.start.line as usize;
            let lines: Vec<&str> = ctx.source_text.lines().collect();

            if line < lines.len() {
                let current_line = lines[line];

                // Try to find the problematic argument
                // This is simplified - just look for function calls
                if let Some(func_start) = current_line.find('(') {
                    if let Some(func_end) = current_line.find(')') {
                        let args_str = &current_line[func_start + 1..func_end];

                        // If there's a single integer argument, wrap it with to_float
                        if !args_str.contains("to_float") && args_str.trim().parse::<i64>().is_ok() {
                            let new_line = current_line.replace(
                                &format!("({})", args_str),
                                &format!("(to_float({}))", args_str.trim())
                            );

                            let edit = WorkspaceEdit {
                                changes: Some({
                                    let mut changes = HashMap::new();
                                    changes.insert(
                                        ctx.uri.clone(),
                                        vec![TextEdit {
                                            range: Range {
                                                start: Position { line: line as u32, character: 0 },
                                                end: Position { line: line as u32, character: current_line.len() as u32 },
                                            },
                                            new_text: new_line,
                                        }],
                                    );
                                    changes
                                }),
                                document_changes: None,
                                change_annotations: None,
                            };

                            fixes.push(CodeAction {
                                title: "Convert argument to Float with to_float()".to_string(),
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: Some(vec![ctx.diagnostic.clone()]),
                                edit: Some(edit),
                                command: None,
                                is_preferred: Some(true),
                                disabled: None,
                                data: None,
                            });
                        }
                    }
                }
            }
        }

        fixes
    }

    /// Fix missing semicolon errors
    fn fix_missing_semicolon(ctx: &QuickFixContext) -> Vec<CodeAction> {
        let mut fixes = Vec::new();

        // Check if the error message mentions semicolon
        if ctx.diagnostic.message.contains("';'") || ctx.diagnostic.message.contains("semicolon") {
            let line = ctx.diagnostic.range.start.line as usize;
            let lines: Vec<&str> = ctx.source_text.lines().collect();

            if line < lines.len() {
                let current_line = lines[line];

                // Add semicolon at the end if not present
                if !current_line.trim().ends_with(';') && !current_line.trim().ends_with('{') {
                    let new_line = format!("{};", current_line.trim_end());

                    let edit = WorkspaceEdit {
                        changes: Some({
                            let mut changes = HashMap::new();
                            changes.insert(
                                ctx.uri.clone(),
                                vec![TextEdit {
                                    range: Range {
                                        start: Position { line: line as u32, character: 0 },
                                        end: Position { line: line as u32, character: current_line.len() as u32 },
                                    },
                                    new_text: new_line,
                                }],
                            );
                            changes
                        }),
                        document_changes: None,
                        change_annotations: None,
                    };

                    fixes.push(CodeAction {
                        title: "Add missing semicolon".to_string(),
                        kind: Some(CodeActionKind::QUICKFIX),
                        diagnostics: Some(vec![ctx.diagnostic.clone()]),
                        edit: Some(edit),
                        command: None,
                        is_preferred: Some(true),
                        disabled: None,
                        data: None,
                    });
                }
            }
        }

        fixes
    }

    /// Generate "Fix All" action for multiple similar errors
    pub fn generate_fix_all(&self, uri: &Url, diagnostics: &[Diagnostic], source_text: &str) -> Option<CodeAction> {
        // Group diagnostics by code
        let mut by_code: HashMap<String, Vec<&Diagnostic>> = HashMap::new();

        for diag in diagnostics {
            if let Some(code) = &diag.code {
                let code_str = match code {
                    NumberOrString::String(s) => s.clone(),
                    NumberOrString::Number(n) => n.to_string(),
                };
                by_code.entry(code_str).or_insert_with(Vec::new).push(diag);
            }
        }

        // Find the most common error type
        let most_common = by_code.iter().max_by_key(|(_, diags)| diags.len())?;

        if most_common.1.len() < 2 {
            return None; // Not worth a "Fix All" for just one error
        }

        let (code, diags) = most_common;

        // Generate all fixes
        let mut all_edits = Vec::new();
        for diag in diags {
            let ctx = QuickFixContext {
                uri: uri.clone(),
                diagnostic: (*diag).clone(),
                source_text: source_text.to_string(),
            };

            let fixes = self.generate_fixes(&ctx);
            if let Some(fix) = fixes.first() {
                if let Some(edit) = &fix.edit {
                    if let Some(changes) = &edit.changes {
                        if let Some(edits) = changes.get(uri) {
                            all_edits.extend(edits.clone());
                        }
                    }
                }
            }
        }

        if all_edits.is_empty() {
            return None;
        }

        Some(CodeAction {
            title: format!("Fix all {} errors ({})", code, diags.len()),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(diags.iter().map(|d| (*d).clone()).collect()),
            edit: Some(WorkspaceEdit {
                changes: Some({
                    let mut changes = HashMap::new();
                    changes.insert(uri.clone(), all_edits);
                    changes
                }),
                document_changes: None,
                change_annotations: None,
            }),
            command: None,
            is_preferred: Some(false),
            disabled: None,
            data: None,
        })
    }
}

impl Default for QuickFixGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_semicolon_fix_generation() {
        let generator = QuickFixGenerator::new();

        let uri = Url::parse("file:///test.hlxa").unwrap();
        let diagnostic = Diagnostic {
            range: Range {
                start: Position { line: 0, character: 0 },
                end: Position { line: 0, character: 10 },
            },
            severity: Some(DiagnosticSeverity::ERROR),
            code: Some(NumberOrString::String("missing-semicolon".to_string())),
            message: "Missing ';' after statement".to_string(),
            source: Some("hlx".to_string()),
            related_information: None,
            tags: None,
            code_description: None,
            data: None,
        };

        let ctx = QuickFixContext {
            uri: uri.clone(),
            diagnostic: diagnostic.clone(),
            source_text: "let x = 5".to_string(),
        };

        let fixes = generator.generate_fixes(&ctx);
        assert_eq!(fixes.len(), 1);
        assert_eq!(fixes[0].title, "Add missing semicolon");
    }
}
