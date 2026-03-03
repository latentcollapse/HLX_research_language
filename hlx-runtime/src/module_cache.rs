//! Module Cache for Multi-File Compilation (Phase 3.1, 3.2)
//!
//! Provides depth-first recursive compilation with cycle detection.
//! This is the new infrastructure for Phase 3 module system.

use crate::ast::{Function, Item};
use crate::ast_parser::AstParser;
use crate::{Bytecode, Lowerer};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Compiled module with bytecode and export table (Phase 3.1)
#[derive(Debug, Clone)]
pub struct CompiledModule {
    pub path: PathBuf,
    pub bytecode: Bytecode,
    /// Export table: function name -> (start_pc, param_count)
    pub exports: HashMap<String, (u32, u32)>,
    /// AST functions for re-export (used when this module is imported by another)
    pub ast_functions: HashMap<String, Function>,
}

/// Module cache for multi-file compilation (Phase 3.1)
#[derive(Debug, Clone)]
pub struct ModuleCache {
    /// Map from absolute file path to compiled module
    modules: HashMap<PathBuf, CompiledModule>,
    /// Track modules currently being compiled (for cycle detection)
    in_progress: HashSet<PathBuf>,
}

impl ModuleCache {
    pub fn new() -> Self {
        ModuleCache {
            modules: HashMap::new(),
            in_progress: HashSet::new(),
        }
    }

    /// Get compiled module from cache
    pub fn get(&self, path: &Path) -> Option<&CompiledModule> {
        self.modules.get(path)
    }

    /// Check if module is being compiled (cycle detection)
    pub fn is_compiling(&self, path: &Path) -> bool {
        self.in_progress.contains(path)
    }

    /// Mark module as being compiled
    pub fn start_compiling(&mut self, path: PathBuf) {
        self.in_progress.insert(path);
    }

    /// Mark module as finished compiling
    pub fn finish_compiling(&mut self, path: &Path) {
        self.in_progress.remove(path);
    }

    /// Insert compiled module into cache
    pub fn insert(&mut self, path: PathBuf, module: CompiledModule) {
        self.modules.insert(path, module);
    }

    /// Resolve and compile a module with all its imports (Phase 3.2)
    ///
    /// This is a depth-first recursive compilation:
    /// 1. Check for circular imports
    /// 2. Return cached if already compiled
    /// 3. Mark as in-progress
    /// 4. Parse the file
    /// 5. For each import, recursively compile it first
    /// 6. Lower this module with all imported functions available
    /// 7. Cache and return
    pub fn compile_recursive(
        &mut self,
        file_path: &Path,
        search_paths: &[PathBuf],
    ) -> Result<CompiledModule, String> {
        // Cycle detection
        if self.is_compiling(file_path) {
            return Err(format!(
                "Circular import detected: {} is already being compiled",
                file_path.display()
            ));
        }

        // Return cached module if already compiled
        if let Some(module) = self.get(file_path) {
            return Ok(module.clone());
        }

        // Mark as in-progress
        self.start_compiling(file_path.to_path_buf());

        // Read and parse source
        let source = std::fs::read_to_string(file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        let ast = AstParser::parse(&source).map_err(|e| {
            format!(
                "Failed to parse {}: line {} - {}",
                file_path.display(),
                e.line,
                e.message
            )
        })?;

        // Collect imports and compile them first (depth-first)
        let mut imported_functions: HashMap<String, Function> = HashMap::new();
        for item in &ast.items {
            if let Item::Import(import) = item {
                let import_path = resolve_import_path(&import.module, file_path, search_paths)?;
                let imported_module = self.compile_recursive(&import_path, search_paths)?;

                // Add imported AST functions to available imports
                for (name, func) in &imported_module.ast_functions {
                    imported_functions.insert(name.clone(), func.clone());
                }
            }
        }

        // Lower this module's AST to bytecode with imported functions
        let (bytecode, functions) = Lowerer::lower_with_imports(&ast, imported_functions)
            .map_err(|e| format!("Failed to lower {}: {}", file_path.display(), e.message))?;

        // Build export tables
        let mut exports = HashMap::new();
        let mut ast_functions = HashMap::new();

        for (name, (pc, params)) in &functions {
            exports.insert(name.clone(), (*pc as u32, *params as u32));
        }

        // Also extract AST functions from this module for re-export
        for item in &ast.items {
            if let Item::Function(func) = item {
                ast_functions.insert(func.name.clone(), func.clone());
            }
        }

        // Create compiled module
        let module = CompiledModule {
            path: file_path.to_path_buf(),
            bytecode,
            exports,
            ast_functions,
        };

        // Cache and return
        self.finish_compiling(file_path);
        self.insert(file_path.to_path_buf(), module.clone());

        Ok(module)
    }
}

/// Resolve an import path relative to the importing file
fn resolve_import_path(
    module: &str,
    importing_file: &Path,
    search_paths: &[PathBuf],
) -> Result<PathBuf, String> {
    let style = detect_style(module);

    match style {
        ImportStyle::Path => {
            // Path import: resolve relative to importing file's directory
            let base_dir = importing_file.parent().ok_or_else(|| {
                "Cannot resolve relative import: importing file has no parent directory".to_string()
            })?;
            let path = base_dir.join(module);

            if path.exists() {
                return Ok(path);
            }
            // Try with .hlx extension
            let with_ext = path.with_extension("hlx");
            if with_ext.exists() {
                return Ok(with_ext);
            }
            Err(format!("Path import not found: {}", path.display()))
        }
        ImportStyle::Module => {
            // Module import: search in stdlib paths
            let module_path = module.replace("::", "/");
            for base in search_paths {
                let file_path = base.join(&module_path).with_extension("hlx");
                if file_path.exists() {
                    return Ok(file_path);
                }
                // Check for mod.hlx in directory
                let alt_path = base.join(&module_path).join("mod.hlx");
                if alt_path.exists() {
                    return Ok(alt_path);
                }
            }
            Err(format!(
                "Module import not found: {} (searched in {:?})",
                module, search_paths
            ))
        }
    }
}

/// Import style for module resolution
#[derive(Debug, Clone, PartialEq)]
pub enum ImportStyle {
    /// Path import: `./path.hlx` or `../lib.hlx`
    Path,
    /// Module import: `std::math` or `hil::infer`
    Module,
}

/// Detect import style from module string
fn detect_style(module: &str) -> ImportStyle {
    if module.starts_with("./") || module.starts_with("../") || module.contains('/') {
        ImportStyle::Path
    } else if module.contains("::") {
        ImportStyle::Module
    } else {
        ImportStyle::Module
    }
}
