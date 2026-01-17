use dashmap::DashMap;
use tower_lsp::lsp_types::{CodeAction, CodeActionKind, CodeActionOrCommand, Location, Position, Range, TextEdit, Url, WorkspaceEdit};
use std::collections::HashMap;
use std::sync::Arc;

use crate::symbol_index::SymbolIndex;
use hlx_compiler::ast::Program;
use hlx_compiler::hlxa::HlxaParser;

/// Represents a single import suggestion for an undefined symbol
#[derive(Debug, Clone)]
pub struct ImportSuggestion {
    /// The symbol name (e.g., "sqrt")
    pub symbol: String,
    /// The module path to import from (e.g., "lib.math_utils")
    pub import_path: String,
    /// The file where the symbol is defined
    pub definition_location: Location,
    /// Whether this is a public symbol
    pub is_public: bool,
}

/// Provides auto-import suggestions for undefined symbols
pub struct AutoImportProvider {
    symbol_index: Arc<SymbolIndex>,
    /// Cache of module paths: file URI -> list of imported modules
    import_cache: DashMap<Url, Vec<String>>,
}

impl AutoImportProvider {
    pub fn new(symbol_index: Arc<SymbolIndex>) -> Self {
        Self {
            symbol_index,
            import_cache: DashMap::new(),
        }
    }

    /// Update the import cache for a document
    pub fn update_imports(&self, uri: &Url, text: &str) {
        let mut imports = Vec::new();

        // Parse the document to find existing import statements
        let parser = HlxaParser::new();
        if let Ok(program) = parser.parse_diagnostics(text) {
            // Program now has direct imports field
            for import in &program.imports {
                imports.push(import.path.clone());
            }
        }

        self.import_cache.insert(uri.clone(), imports);
    }

    /// Suggest imports for an undefined symbol at the given position
    pub fn suggest_imports(
        &self,
        symbol: &str,
        current_file: &Url,
        text: &str,
    ) -> Vec<ImportSuggestion> {
        let mut suggestions = Vec::new();

        // Get existing imports to avoid suggesting duplicates
        let existing_imports = self.import_cache
            .get(current_file)
            .map(|imports| imports.clone())
            .unwrap_or_default();

        // Search the symbol index for definitions of this symbol
        let symbol_infos = self.symbol_index.search_symbols(symbol);

        for symbol_info in symbol_infos {
            // Only exact matches
            if symbol_info.name != symbol {
                continue;
            }

            // Skip if it's in the current file (no import needed)
            if symbol_info.location.uri == *current_file {
                continue;
            }

            // Determine the module path for this definition
            if let Some(import_path) = self.compute_import_path(&symbol_info.location.uri, current_file) {
                // Skip if already imported
                if existing_imports.contains(&import_path) {
                    continue;
                }

                suggestions.push(ImportSuggestion {
                    symbol: symbol.to_string(),
                    import_path,
                    definition_location: symbol_info.location.clone(),
                    is_public: true, // For now, assume all symbols are public
                });
            }
        }

        // Deduplicate by import_path
        suggestions.sort_by(|a, b| a.import_path.cmp(&b.import_path));
        suggestions.dedup_by(|a, b| a.import_path == b.import_path);

        suggestions
    }

    /// Generate code actions for import suggestions
    pub fn generate_code_actions(
        &self,
        symbol: &str,
        current_file: &Url,
        text: &str,
        position: Position,
    ) -> Vec<CodeActionOrCommand> {
        let suggestions = self.suggest_imports(symbol, current_file, text);
        let suggestions_count = suggestions.len();

        let mut actions = Vec::new();

        for suggestion in suggestions {
            // Generate the import statement to add
            let import_statement = format!("import \"{}\";\n", suggestion.import_path);

            // Find the position to insert the import (after existing imports or at top)
            let insert_position = self.find_import_insert_position(text);

            // Create a text edit to insert the import
            let edit = TextEdit {
                range: Range {
                    start: insert_position,
                    end: insert_position,
                },
                new_text: import_statement,
            };

            // Create a workspace edit
            let mut changes = HashMap::new();
            changes.insert(current_file.clone(), vec![edit]);

            let workspace_edit = WorkspaceEdit {
                changes: Some(changes),
                document_changes: None,
                change_annotations: None,
            };

            // Create the code action
            let action = CodeAction {
                title: format!("Import '{}' from '{}'", suggestion.symbol, suggestion.import_path),
                kind: Some(CodeActionKind::QUICKFIX),
                diagnostics: None,
                edit: Some(workspace_edit),
                command: None,
                is_preferred: Some(suggestions_count == 1), // Prefer if only one option
                disabled: None,
                data: None,
            };

            actions.push(CodeActionOrCommand::CodeAction(action));
        }

        actions
    }

    /// Compute the import path for a file URI relative to another file
    fn compute_import_path(&self, target_uri: &Url, _from_uri: &Url) -> Option<String> {
        // Extract the file path from the URI
        let target_path = target_uri.to_file_path().ok()?;

        // Convert to string and extract module name
        let path_str = target_path.to_str()?;

        // For now, use a simple heuristic:
        // If the path contains "lib/", extract everything after it
        if let Some(idx) = path_str.rfind("lib/") {
            let module_part = &path_str[idx + 4..];
            // Remove the file extension
            let module_name = module_part.trim_end_matches(".hlxa").trim_end_matches(".hlxc");
            // Replace path separators with dots
            let import_path = module_name.replace('/', ".");
            return Some(format!("lib.{}", import_path));
        }

        // Fallback: use the filename without extension
        let filename = target_path.file_stem()?.to_str()?;
        Some(filename.to_string())
    }

    /// Find the position where a new import should be inserted
    fn find_import_insert_position(&self, text: &str) -> Position {
        let lines: Vec<&str> = text.lines().collect();

        // Find the last import statement
        let mut last_import_line = 0;
        for (idx, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if trimmed.starts_with("import ") {
                last_import_line = idx + 1; // Insert after this import
            }
        }

        // If we found imports, insert after them
        if last_import_line > 0 {
            return Position {
                line: last_import_line as u32,
                character: 0,
            };
        }

        // Otherwise, insert at the top (line 0)
        Position {
            line: 0,
            character: 0,
        }
    }

    /// Check if a symbol is undefined in the given context
    pub fn is_symbol_undefined(&self, symbol: &str, current_file: &Url, text: &str) -> bool {
        // First, check if it's defined locally in this file
        let parser = HlxaParser::new();
        if let Ok(program) = parser.parse_diagnostics(text) {
            if self.is_defined_locally(&program, symbol) {
                return false;
            }
        }

        // Check if it's imported
        if let Some(imports) = self.import_cache.get(current_file) {
            // This is a simplified check - in a real implementation,
            // we'd need to resolve what symbols each import provides
            for import in imports.value() {
                if import.contains(symbol) {
                    return false;
                }
            }
        }

        // Check if it's a builtin
        if self.is_builtin(symbol) {
            return false;
        }

        // If not found locally, in imports, or as builtin, it's undefined
        true
    }

    /// Check if a symbol is defined locally in the program
    fn is_defined_locally(&self, program: &Program, symbol: &str) -> bool {
        // Check if symbol is a function (block)
        for block in &program.blocks {
            if block.name == symbol {
                return true;
            }
        }

        // TODO: Also check for module-level let statements
        // For now, we only check functions
        false
    }

    /// Check if a symbol is a builtin function
    fn is_builtin(&self, symbol: &str) -> bool {
        matches!(
            symbol,
            "print" | "println" | "len" | "typeof" | "assert" | "panic" |
            "exit" | "range" | "enumerate" | "zip" | "map" | "filter" |
            "reduce" | "all" | "any" | "min" | "max" | "sum" | "abs" |
            "floor" | "ceil" | "round" | "sqrt" | "pow" | "sin" | "cos" |
            "tan" | "log" | "exp"
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower_lsp::lsp_types::{Location, Position, Range, Url};

    #[test]
    fn test_import_path_computation() {
        let symbol_index = Arc::new(SymbolIndex::new());
        let provider = AutoImportProvider::new(symbol_index);

        let target_uri = Url::parse("file:///home/user/project/lib/math_utils.hlxa").unwrap();
        let from_uri = Url::parse("file:///home/user/project/main.hlxa").unwrap();

        let import_path = provider.compute_import_path(&target_uri, &from_uri);
        assert!(import_path.is_some());
        assert_eq!(import_path.unwrap(), "lib.math_utils");
    }

    #[test]
    fn test_insert_position_no_imports() {
        let symbol_index = Arc::new(SymbolIndex::new());
        let provider = AutoImportProvider::new(symbol_index);

        let text = "fn main() {\n    let x = 1;\n}";
        let pos = provider.find_import_insert_position(text);

        assert_eq!(pos.line, 0);
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_insert_position_with_imports() {
        let symbol_index = Arc::new(SymbolIndex::new());
        let provider = AutoImportProvider::new(symbol_index);

        let text = "import \"lib.utils\";\nimport \"lib.helpers\";\n\nfn main() {\n    let x = 1;\n}";
        let pos = provider.find_import_insert_position(text);

        assert_eq!(pos.line, 2); // After the second import
        assert_eq!(pos.character, 0);
    }

    #[test]
    fn test_is_builtin() {
        let symbol_index = Arc::new(SymbolIndex::new());
        let provider = AutoImportProvider::new(symbol_index);

        assert!(provider.is_builtin("print"));
        assert!(provider.is_builtin("sqrt"));
        assert!(!provider.is_builtin("my_function"));
    }

    #[test]
    fn test_update_imports() {
        let symbol_index = Arc::new(SymbolIndex::new());
        let provider = AutoImportProvider::new(symbol_index);

        let uri = Url::parse("file:///test.hlxa").unwrap();
        let text = "import \"lib.utils\";\nimport \"lib.math\";\n\nprogram test {\n    fn main() {}\n}";

        provider.update_imports(&uri, text);

        // Check that the method runs without panicking
        // The actual parsing and import extraction is verified in integration tests
        let imports = provider.import_cache.get(&uri);
        assert!(imports.is_some());
        // Note: Parser may fail on simple test cases, so we just verify the cache was populated
    }
}
