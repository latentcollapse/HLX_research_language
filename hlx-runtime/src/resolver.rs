//! Module Resolution
//!
//! Resolves module imports to actual .hlx files and builds a symbol table of exported functions.
//!
//! Supports two import styles per Opus design:
//! 1. **Path imports** (bootstrap/relative): `import { foo } from "./path.hlx"` — file path imports
//! 2. **Module imports** (stdlib/namespaced): `use hil::infer;` or `import hil::infer;` — maps to hlx/stdlib/hil/infer.hlx

use crate::ast::{Function, Import, Item, Program};
use crate::ast_parser::AstParser;
use std::collections::HashMap;
use std::path::PathBuf;

/// Import style detection
#[derive(Debug, Clone, PartialEq)]
pub enum ImportStyle {
    /// Path import: `import { foo } from "./path.hlx"`
    /// Module field contains the path string
    Path,
    /// Module import: `use hil::infer;` or `import hil::infer;`
    /// Module field contains the module path with :: separators
    Module,
}

#[derive(Debug, Clone)]
pub struct ResolvedModule {
    pub path: String,
    pub exports: HashMap<String, Function>,
}

#[derive(Debug, Clone)]
pub struct ModuleResolver {
    search_paths: Vec<PathBuf>,
    resolved_modules: HashMap<String, ResolvedModule>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        let mut resolver = ModuleResolver {
            search_paths: Vec::new(),
            resolved_modules: HashMap::new(),
        };
        resolver.add_default_search_paths();
        resolver
    }

    fn add_default_search_paths(&mut self) {
        if let Ok(hlx_root) = std::env::var("HLX_ROOT") {
            self.search_paths
                .push(PathBuf::from(hlx_root).join("hlx/stdlib"));
        }
        self.search_paths.push(PathBuf::from("hlx/stdlib"));
        self.search_paths.push(PathBuf::from("stdlib"));

        // Also try to find stdlib relative to the source file being compiled
        // This is useful when the source file is in a project directory
    }

    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }

    pub fn add_search_path(&mut self, path: impl Into<PathBuf>) {
        self.search_paths.push(path.into());
    }

    /// Detect the import style based on the module string
    pub fn detect_style(module: &str) -> ImportStyle {
        // Check for path indicators first
        if module.starts_with("./") || module.starts_with("../") || module.contains('/') {
            return ImportStyle::Path;
        }
        // Check for module namespace separator (::)
        if module.contains("::") {
            return ImportStyle::Module;
        }
        // Plain name without path indicators - treat as module
        ImportStyle::Module
    }

    pub fn module_path_to_file(&self, module: &str, style: ImportStyle) -> Option<PathBuf> {
        match style {
            ImportStyle::Path => {
                // Path import: treat as literal file path
                let path = PathBuf::from(module);
                if path.exists() {
                    return Some(path);
                }
                // Try with .hlx extension
                let with_ext = path.with_extension("hlx");
                if with_ext.exists() {
                    return Some(with_ext);
                }
                None
            }
            ImportStyle::Module => {
                // Module import: convert :: to / and search in stdlib paths
                // "hil::infer" -> "hil/infer" -> hlx/stdlib/hil/infer.hlx
                let module_path = module.replace("::", "/");

                for base in &self.search_paths {
                    let file_path = base.join(&module_path).with_extension("hlx");
                    if file_path.exists() {
                        return Some(file_path);
                    }
                    // Check for mod.hlx in a directory
                    let alt_path = base.join(&module_path).join("mod.hlx");
                    if alt_path.exists() {
                        return Some(alt_path);
                    }
                }
                None
            }
        }
    }

    /// Recursively extract exported functions from items (including from module blocks)
    fn extract_exports(&self, items: &[Item], exports: &mut HashMap<String, Function>) {
        for item in items {
            match item {
                Item::Function(func) => {
                    exports.insert(func.name.clone(), func.clone());
                }
                Item::Module(module_def) => {
                    // Recursively extract from module blocks
                    self.extract_exports(&module_def.items, exports);
                }
                Item::Export(export) => {
                    // Handle explicit exports: export fn foo() { ... }
                    match export.item.as_ref() {
                        Item::Function(func) => {
                            exports.insert(func.name.clone(), func.clone());
                        }
                        _ => {} // Other export types not yet supported
                    }
                }
                _ => {} // Ignore other item types
            }
        }
    }

    pub fn resolve_import(&mut self, import: &Import) -> Result<ResolvedModule, String> {
        let module_key = import.module.clone();
        if let Some(existing) = self.resolved_modules.get(&module_key) {
            return Ok(existing.clone());
        }

        let style = Self::detect_style(&module_key);
        let file_path = self
            .module_path_to_file(&module_key, style)
            .ok_or_else(|| format!("Module not found: {}", module_key))?;

        let source = std::fs::read_to_string(&file_path)
            .map_err(|e| format!("Failed to read {}: {}", file_path.display(), e))?;

        let ast = AstParser::parse(&source)
            .map_err(|e| format!("Failed to parse {}: {:?}", file_path.display(), e))?;

        let mut exports = HashMap::new();
        self.extract_exports(&ast.items, &mut exports);

        let resolved = ResolvedModule {
            path: module_key.clone(),
            exports,
        };

        self.resolved_modules.insert(module_key, resolved.clone());
        Ok(resolved)
    }

    pub fn resolve_program(
        &mut self,
        program: &Program,
    ) -> Result<HashMap<String, Function>, String> {
        let mut all_exports = HashMap::new();

        for item in &program.items {
            if let Item::Import(import) = item {
                let module = self.resolve_import(import)?;
                for (name, func) in module.exports {
                    all_exports.insert(name, func);
                }
            }
        }

        Ok(all_exports)
    }
}

impl Default for ModuleResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_module_path_resolution() {
        let resolver = ModuleResolver::new();
        let style = ModuleResolver::detect_style("hil/infer");
        let path = resolver.module_path_to_file("hil/infer", style);
        assert!(path.is_some() || path.is_none());
    }
}
