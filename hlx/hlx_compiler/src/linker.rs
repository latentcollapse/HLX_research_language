// ! Module Linker
//!
//! Links imported functions from external modules into the main program.
//! Resolves imports and merges all code into a single AST for lowering.

use crate::ast::{Program, Block, Import, ExportKind, Module};
use crate::module_resolver::ModuleResolver;
use crate::parser::Parser;
use hlx_core::{Result, HlxError};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Link a program with its imports
/// Returns a single Program with all imported functions included
pub fn link_program(main_program: Program, main_file_path: &Path) -> Result<Program> {
    // If no imports, return as-is
    if main_program.imports.is_empty() {
        return Ok(main_program);
    }

    // Resolve imports manually to handle relative paths correctly
    let mut imported_modules: HashMap<String, Program> = HashMap::new();
    let parent_dir = main_file_path.parent().unwrap_or(Path::new("."));

    for import in &main_program.imports {
        // Resolve relative imports directly
        let module_path = if import.path.starts_with("./") || import.path.starts_with("../") {
            // Relative path - resolve from main file's directory
            parent_dir.join(&import.path).canonicalize()
                .map_err(|e| HlxError::parse(format!(
                    "Cannot resolve import '{}': {}",
                    import.path, e
                )))?
        } else {
            // Absolute or dotted path - use resolver
            let mut resolver = ModuleResolver::new();
            // Don't add parent to search paths to avoid conflicts
            let program = resolver.resolve(&import.path)?;
            imported_modules.insert(import.path.clone(), program);
            continue;
        };

        // Load and parse the file
        let source = std::fs::read_to_string(&module_path)
            .map_err(|e| HlxError::parse(format!(
                "Failed to read module '{}': {}",
                import.path, e
            )))?;

        let parser = crate::HlxaParser::new();
        let program = parser.parse(&source)
            .map_err(|e| HlxError::parse(format!(
                "Failed to parse module '{}': {}",
                import.path, e
            )))?;

        imported_modules.insert(import.path.clone(), program);
    }

    // Build a map of imported function names to their definitions
    let mut imported_functions: HashMap<String, Block> = HashMap::new();

    for (module_path, imported_program) in &imported_modules {
        // For each imported program, find what functions are exported
        let mut exports_set = HashSet::new();

        // Collect exports from modules
        for module in &imported_program.modules {
            for export in &module.exports {
                match export {
                    ExportKind::Function(name) => {
                        exports_set.insert(name.clone());
                    }
                    _ => {}, // TODO: Handle other export types
                }
            }
        }

        // Add ALL blocks from the imported module (not just exported ones)
        // Exported functions may depend on internal helper functions
        for module in &imported_program.modules {
            for block in &module.blocks {
                imported_functions.insert(block.name.clone(), block.clone());
            }
        }

        // Also include program-level blocks
        for block in &imported_program.blocks {
            imported_functions.insert(block.name.clone(), block.clone());
        }

        // Verify that explicitly imported functions are actually exported
        if let Some(items) = main_program.imports.iter()
            .find(|imp| &imp.path == module_path)
            .and_then(|imp| imp.items.as_ref())
        {
            for item in items {
                if !exports_set.contains(&item.name) {
                    return Err(HlxError::parse(format!(
                        "Function '{}' not exported by '{}'",
                        item.name,
                        module_path
                    )));
                }
            }
        }
    }

    // Imports are already verified above

    // Create the linked program by merging imported functions
    let mut linked_blocks = Vec::new();

    // Add all imported functions first
    for (_, block) in imported_functions {
        linked_blocks.push(block);
    }

    // Then add all the main program's blocks
    linked_blocks.extend(main_program.blocks.clone());

    // Also include blocks from main program's modules
    for module in &main_program.modules {
        linked_blocks.extend(module.blocks.clone());
    }

    Ok(Program {
        name: main_program.name,
        imports: vec![], // Clear imports - they've been resolved
        modules: vec![], // Flatten modules
        blocks: linked_blocks,
    })
}

/// Check if a function is imported from a specific module path
fn is_function_imported(imports: &[Import], module_path: &str, function_name: &str) -> bool {
    for import in imports {
        if import.path == module_path {
            if let Some(items) = &import.items {
                return items.iter().any(|item| item.name == function_name);
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::*;

    #[test]
    fn test_no_imports() {
        let program = Program {
            name: "test".to_string(),
            imports: vec![],
            modules: vec![],
            blocks: vec![],
        };

        let result = link_program(program.clone(), Path::new("test.hlxa")).unwrap();
        assert_eq!(result.blocks.len(), 0);
    }

    #[test]
    fn test_import_not_found() {
        let program = Program {
            name: "test".to_string(),
            imports: vec![Import {
                path: "./nonexistent.hlxa".to_string(),
                alias: None,
                items: Some(vec![ImportItem {
                    name: "foo".to_string(),
                    alias: None,
                }]),
                span: Span::default(),
            }],
            modules: vec![],
            blocks: vec![],
        };

        let result = link_program(program, Path::new("test.hlxa"));
        assert!(result.is_err());
    }
}
