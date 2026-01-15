//! Module System Support for LSP
//!
//! Provides import completions, cross-module navigation, and diagnostics.

use tower_lsp::lsp_types::*;
use hlx_compiler::{ModuleResolver, HlxaParser};
use hlx_compiler::ast::Program;
use std::collections::{HashMap, HashSet};
use url::Url;

pub struct ModuleSupport {
    resolver: ModuleResolver,
    // Cache of resolved modules for quick lookups
    module_cache: HashMap<String, Program>,
}

impl ModuleSupport {
    pub fn new() -> Self {
        Self {
            resolver: ModuleResolver::new(),
            module_cache: HashMap::new(),
        }
    }

    /// Get completions for import paths
    /// Triggers on: import "std.|"
    pub fn get_import_path_completions(&self, partial: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Built-in stdlib modules
        let stdlib_modules = vec![
            ("std.math", "Mathematical operations (pi, abs, max, min, etc.)"),
            ("std.array", "Array utilities (empty, is_empty, first, last)"),
            ("std.string", "String utilities (empty, is_empty)"),
        ];

        for (path, description) in stdlib_modules {
            if path.starts_with(partial) || partial.is_empty() {
                items.push(CompletionItem {
                    label: path.to_string(),
                    kind: Some(CompletionItemKind::MODULE),
                    detail: Some("Standard Library Module".to_string()),
                    documentation: Some(Documentation::String(description.to_string())),
                    insert_text: Some(path.to_string()),
                    ..Default::default()
                });
            }
        }

        items
    }

    /// Get completions for imported symbols
    /// Triggers on: import "std.math" { p| }
    pub fn get_import_symbol_completions(&self, module_path: &str) -> Vec<CompletionItem> {
        let mut items = Vec::new();

        // Try to resolve the module
        let mut resolver = ModuleResolver::new();
        if let Ok(program) = resolver.resolve(module_path) {
            // Extract exported symbols from the module
            for module in &program.modules {
                // Add all functions as potential imports (blocks are functions)
                for block in &module.blocks {
                    items.push(CompletionItem {
                        label: block.name.clone(),
                        kind: Some(CompletionItemKind::FUNCTION),
                        detail: Some(format!("Function from {}", module_path)),
                        documentation: Some(Documentation::String(
                            format!("Imported from module '{}'", module_path)
                        )),
                        insert_text: Some(block.name.clone()),
                        ..Default::default()
                    });
                }

                // Add constants
                for constant in &module.constants {
                    items.push(CompletionItem {
                        label: constant.name.clone(),
                        kind: Some(CompletionItemKind::CONSTANT),
                        detail: Some(format!("Constant from {}", module_path)),
                        insert_text: Some(constant.name.clone()),
                        ..Default::default()
                    });
                }
            }
        }

        items
    }

    /// Find definition of an imported symbol
    /// Returns location in the imported module
    pub fn find_import_definition(&mut self, symbol: &str, module_path: &str) -> Option<Location> {
        // Resolve the module if not cached
        if !self.module_cache.contains_key(module_path) {
            if let Ok(program) = self.resolver.resolve(module_path) {
                self.module_cache.insert(module_path.to_string(), program);
            }
        }

        // Search for the symbol in the cached module
        if let Some(program) = self.module_cache.get(module_path) {
            for module in &program.modules {
                for block in &module.blocks {
                    if block.name == symbol {
                        // Create a synthetic location (we'd need actual file tracking for real impl)
                        return Some(Location {
                            uri: Url::parse(&format!("hlx://stdlib/{}.hlxa", module_path)).unwrap(),
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 0 },
                            },
                        });
                    }
                }
            }
        }

        None
    }

    /// Get hover information for an import or imported symbol
    pub fn get_import_hover(&mut self, text: &str, position: Position, doc: &str) -> Option<Hover> {
        // Extract import statement at position
        let line_start = doc.lines().take(position.line as usize).map(|l| l.len() + 1).sum::<usize>();
        let line = doc.lines().nth(position.line as usize)?;

        // Check if we're hovering over an import path
        if let Some(module_path) = self.extract_import_path(line) {
            // Resolve module and show summary
            if let Ok(program) = self.resolver.resolve(&module_path) {
                let mut content = format!("**Module:** `{}`\n\n", module_path);

                for module in &program.modules {
                    let func_count = module.blocks.len();
                    let const_count = module.constants.len();

                    content.push_str(&format!(
                        "**Exports:**\n- {} functions\n- {} constants\n",
                        func_count, const_count
                    ));

                    // List first few functions
                    if func_count > 0 {
                        content.push_str("\n**Functions:**\n");
                        let mut shown = 0;
                        for block in &module.blocks {
                            if shown < 5 {
                                content.push_str(&format!("- `{}`\n", block.name));
                                shown += 1;
                            }
                        }
                        if func_count > 5 {
                            content.push_str(&format!("- ... and {} more\n", func_count - 5));
                        }
                    }
                }

                return Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: content,
                    }),
                    range: None,
                });
            }
        }

        None
    }

    /// Validate imports and return diagnostics
    pub fn validate_imports(&mut self, doc: &str, uri: &Url) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();

        // Parse document to find imports
        let parser = HlxaParser::new();
        if let Ok(program) = parser.parse_diagnostics(doc) {
            for import in &program.imports {
                // Try to resolve the import
                match self.resolver.resolve(&import.path) {
                    Ok(_) => {
                        // Import is valid
                    }
                    Err(_) => {
                        // Import failed - create diagnostic
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position {
                                    line: import.span.line as u32,
                                    character: import.span.col as u32,
                                },
                                end: Position {
                                    line: import.span.line as u32,
                                    character: (import.span.col + import.path.len() as u32),
                                },
                            },
                            severity: Some(DiagnosticSeverity::ERROR),
                            code: Some(NumberOrString::String("import-not-found".to_string())),
                            source: Some("hlx-lsp".to_string()),
                            message: format!("Module '{}' not found", import.path),
                            related_information: None,
                            tags: None,
                            code_description: None,
                            data: None,
                        });
                    }
                }
            }

            // Check for unused imports
            let imported_symbols = self.extract_imported_symbols(&program);
            let used_symbols = self.extract_used_symbols(&program);

            for (import_path, symbols) in &imported_symbols {
                for symbol in symbols {
                    if !used_symbols.contains(symbol) {
                        diagnostics.push(Diagnostic {
                            range: Range {
                                start: Position { line: 0, character: 0 },
                                end: Position { line: 0, character: 0 },
                            },
                            severity: Some(DiagnosticSeverity::HINT),
                            code: Some(NumberOrString::String("unused-import".to_string())),
                            source: Some("hlx-lsp".to_string()),
                            message: format!("Imported symbol '{}' is never used", symbol),
                            tags: Some(vec![DiagnosticTag::UNNECESSARY]),
                            related_information: None,
                            code_description: None,
                            data: None,
                        });
                    }
                }
            }
        }

        diagnostics
    }

    /// Suggest auto-imports for undefined symbols
    pub fn suggest_auto_imports(&mut self, symbol: &str) -> Vec<CodeAction> {
        let mut actions = Vec::new();

        // Check if symbol exists in any stdlib module
        let stdlib_modules = vec!["std.math", "std.array", "std.string"];

        for module_path in stdlib_modules {
            if let Ok(program) = self.resolver.resolve(module_path) {
                for module in &program.modules {
                    for block in &module.blocks {
                        if block.name == symbol {
                            // Create code action to add import
                            actions.push(CodeAction {
                                title: format!("Import '{}' from '{}'", symbol, module_path),
                                kind: Some(CodeActionKind::QUICKFIX),
                                diagnostics: None,
                                edit: Some(WorkspaceEdit {
                                    changes: None,
                                    document_changes: None,
                                    change_annotations: None,
                                }),
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

        actions
    }

    // Helper: Extract module path from import line
    fn extract_import_path(&self, line: &str) -> Option<String> {
        if !line.contains("import") {
            return None;
        }

        // Match: import "path"
        if let Some(start) = line.find('"') {
            if let Some(end) = line[start + 1..].find('"') {
                return Some(line[start + 1..start + 1 + end].to_string());
            }
        }

        None
    }

    // Helper: Extract imported symbols from program
    fn extract_imported_symbols(&self, program: &Program) -> HashMap<String, HashSet<String>> {
        let mut result = HashMap::new();

        for import in &program.imports {
            if let Some(items) = &import.items {
                let mut symbols = HashSet::new();
                for item in items {
                    symbols.insert(item.name.clone());
                }
                result.insert(import.path.clone(), symbols);
            }
        }

        result
    }

    // Helper: Extract used symbols from program
    fn extract_used_symbols(&self, program: &Program) -> HashSet<String> {
        let mut used = HashSet::new();

        // Walk through all expressions and collect identifiers
        // (This is a simplified version - real impl would walk the full AST)
        for module in &program.modules {
            for block in &module.blocks {
                // Extract identifiers from function items
                // This would need proper AST walking
                used.insert(block.name.clone());
            }
        }

        used
    }
}

impl Default for ModuleSupport {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_import_path_completions() {
        let support = ModuleSupport::new();
        let completions = support.get_import_path_completions("std.");
        assert!(!completions.is_empty());
        assert!(completions.iter().any(|c| c.label == "std.math"));
    }

    #[test]
    fn test_import_symbol_completions() {
        let support = ModuleSupport::new();
        let completions = support.get_import_symbol_completions("std.math");
        assert!(!completions.is_empty());
    }
}
