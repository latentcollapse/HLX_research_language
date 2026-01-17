//! Import Organization
//!
//! Provides import management capabilities:
//! - Sort imports
//! - Remove unused imports
//! - Group imports by category
//! - Organize imports command

use tower_lsp::lsp_types::{CodeAction, CodeActionKind, Range, TextEdit, Url, WorkspaceEdit};
use std::collections::{HashMap, HashSet};
use hlx_compiler::ast::{Import, Program};

/// Import category for grouping
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ImportCategory {
    Stdlib,    // Standard library imports
    External,  // External dependencies
    Local,     // Local project imports
}

/// Analyzed import
#[derive(Debug, Clone)]
pub struct AnalyzedImport {
    pub path: String,
    pub category: ImportCategory,
    pub is_used: bool,
    pub range: Range,
}

/// Import organization provider
pub struct ImportOrganizer {
    /// Standard library prefixes
    stdlib_prefixes: Vec<String>,
}

impl ImportOrganizer {
    pub fn new() -> Self {
        Self {
            stdlib_prefixes: vec![
                "std.".to_string(),
                "core.".to_string(),
                "hlx.".to_string(),
            ],
        }
    }

    /// Organize imports in a document
    pub fn organize_imports(
        &self,
        uri: &Url,
        program: &Program,
        source: &str,
        used_symbols: &HashSet<String>,
    ) -> Option<CodeAction> {
        // Analyze imports
        let mut imports = self.analyze_imports(&program.imports, used_symbols);

        // Remove unused
        imports.retain(|imp| imp.is_used);

        // Sort and group
        imports.sort_by(|a, b| {
            a.category.cmp(&b.category)
                .then_with(|| a.path.cmp(&b.path))
        });

        // Generate organized import text
        let organized = self.format_imports(&imports);

        // Find import region
        let import_range = self.find_import_range(&program.imports, source)?;

        // Create text edit
        let mut changes = HashMap::new();
        changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: import_range,
                new_text: organized,
            }],
        );

        Some(CodeAction {
            title: "Organize Imports".to_string(),
            kind: Some(CodeActionKind::SOURCE_ORGANIZE_IMPORTS),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    /// Analyze imports to categorize and check usage
    fn analyze_imports(
        &self,
        imports: &[Import],
        used_symbols: &HashSet<String>,
    ) -> Vec<AnalyzedImport> {
        imports
            .iter()
            .map(|imp| {
                let category = self.categorize_import(&imp.path);

                // Check if import is used
                let is_used = self.is_import_used(&imp.path, used_symbols);

                AnalyzedImport {
                    path: imp.path.clone(),
                    category,
                    is_used,
                    range: Range {
                        start: tower_lsp::lsp_types::Position {
                            line: imp.span.line,
                            character: imp.span.col,
                        },
                        end: tower_lsp::lsp_types::Position {
                            line: imp.span.line,
                            character: imp.span.col + imp.path.len() as u32,
                        },
                    },
                }
            })
            .collect()
    }

    /// Categorize an import
    fn categorize_import(&self, path: &str) -> ImportCategory {
        for prefix in &self.stdlib_prefixes {
            if path.starts_with(prefix) {
                return ImportCategory::Stdlib;
            }
        }

        // Check if it's an external dependency (contains organization/author)
        if path.contains('.') && !path.starts_with("./") && !path.starts_with("../") {
            return ImportCategory::External;
        }

        ImportCategory::Local
    }

    /// Check if an import is used
    fn is_import_used(&self, import_path: &str, used_symbols: &HashSet<String>) -> bool {
        // Extract module name from path
        let module_name = import_path
            .split('.')
            .last()
            .unwrap_or(import_path);

        // Check if any symbol from this module is used
        used_symbols.iter().any(|symbol| {
            symbol.starts_with(module_name)
                || symbol.contains(&format!("{}.", module_name))
        })
    }

    /// Format imports with proper grouping
    fn format_imports(&self, imports: &[AnalyzedImport]) -> String {
        let mut result = String::new();
        let mut last_category = None;

        for import in imports {
            // Add blank line between categories
            if let Some(last) = last_category {
                if last != import.category {
                    result.push('\n');
                }
            }

            result.push_str(&format!("import \"{}\";\n", import.path));
            last_category = Some(import.category.clone());
        }

        result
    }

    /// Find the range of all imports
    fn find_import_range(&self, imports: &[Import], source: &str) -> Option<Range> {
        if imports.is_empty() {
            return None;
        }

        let first = imports.first()?;
        let last = imports.last()?;

        // Find line numbers
        let start_line = first.span.line;
        let end_line = last.span.line + 1; // Include the line after last import

        Some(Range {
            start: tower_lsp::lsp_types::Position {
                line: start_line,
                character: 0,
            },
            end: tower_lsp::lsp_types::Position {
                line: end_line,
                character: 0,
            },
        })
    }

    /// Remove unused imports
    pub fn remove_unused_imports(
        &self,
        uri: &Url,
        program: &Program,
        used_symbols: &HashSet<String>,
    ) -> Option<CodeAction> {
        let imports = self.analyze_imports(&program.imports, used_symbols);

        // Find unused imports
        let unused: Vec<_> = imports.iter().filter(|i| !i.is_used).collect();

        if unused.is_empty() {
            return None;
        }

        // Create edits to remove unused imports
        let mut changes = HashMap::new();
        let edits: Vec<TextEdit> = unused
            .iter()
            .map(|imp| TextEdit {
                range: imp.range,
                new_text: String::new(), // Delete
            })
            .collect();

        changes.insert(uri.clone(), edits);

        Some(CodeAction {
            title: format!("Remove {} unused imports", unused.len()),
            kind: Some(CodeActionKind::SOURCE),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        })
    }

    /// Sort imports alphabetically
    pub fn sort_imports(
        &self,
        uri: &Url,
        program: &Program,
        used_symbols: &HashSet<String>,
    ) -> Option<CodeAction> {
        let mut imports = self.analyze_imports(&program.imports, used_symbols);

        // Sort alphabetically within categories
        imports.sort_by(|a, b| {
            a.category.cmp(&b.category)
                .then_with(|| a.path.cmp(&b.path))
        });

        let organized = self.format_imports(&imports);
        let import_range = self.find_import_range(&program.imports, "")?;

        let mut changes = HashMap::new();
        changes.insert(
            uri.clone(),
            vec![TextEdit {
                range: import_range,
                new_text: organized,
            }],
        );

        Some(CodeAction {
            title: "Sort Imports".to_string(),
            kind: Some(CodeActionKind::SOURCE),
            edit: Some(WorkspaceEdit {
                changes: Some(changes),
                ..Default::default()
            }),
            ..Default::default()
        })
    }
}

impl Default for ImportOrganizer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorization() {
        let organizer = ImportOrganizer::new();

        assert_eq!(
            organizer.categorize_import("std.math"),
            ImportCategory::Stdlib
        );
        assert_eq!(
            organizer.categorize_import("external.lib"),
            ImportCategory::External
        );
        assert_eq!(
            organizer.categorize_import("my_module"),
            ImportCategory::Local
        );
    }

    #[test]
    fn test_usage_check() {
        let organizer = ImportOrganizer::new();
        let mut used = HashSet::new();
        used.insert("math.sqrt".to_string());

        assert!(organizer.is_import_used("std.math", &used));
        assert!(!organizer.is_import_used("std.io", &used));
    }

    #[test]
    fn test_import_formatting() {
        let organizer = ImportOrganizer::new();
        let imports = vec![
            AnalyzedImport {
                path: "std.math".to_string(),
                category: ImportCategory::Stdlib,
                is_used: true,
                range: Range::default(),
            },
            AnalyzedImport {
                path: "my_module".to_string(),
                category: ImportCategory::Local,
                is_used: true,
                range: Range::default(),
            },
        ];

        let formatted = organizer.format_imports(&imports);

        assert!(formatted.contains("std.math"));
        assert!(formatted.contains("my_module"));
        // Should have blank line between categories
        assert!(formatted.contains("\n\n"));
    }
}
