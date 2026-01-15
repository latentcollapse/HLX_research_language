//! Module Resolution System
//!
//! Handles module discovery, loading, and dependency resolution.
//! Supports both file-based and embedded stdlib modules.

use crate::ast::{Program, Import, Module};
use crate::parser::Parser;
use crate::hlxa::HlxaParser;
use hlx_core::{Result, HlxError};
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::{Path, PathBuf};
use std::fs;

/// Module resolution context
pub struct ModuleResolver {
    /// Search paths for modules (in order of priority)
    pub search_paths: Vec<PathBuf>,
    /// Cache of loaded modules (path -> Program)
    module_cache: HashMap<PathBuf, Program>,
    /// Embedded stdlib modules
    stdlib_modules: HashMap<String, &'static str>,
}

impl ModuleResolver {
    /// Create a new resolver with default search paths
    pub fn new() -> Self {
        let mut search_paths = vec![
            PathBuf::from("."),           // Current directory
            PathBuf::from("./lib"),       // Local lib directory
            PathBuf::from("./modules"),   // Local modules directory
        ];

        // Add HLX_PATH environment variable paths
        if let Ok(hlx_path) = std::env::var("HLX_PATH") {
            for path in hlx_path.split(':') {
                search_paths.push(PathBuf::from(path));
            }
        }

        Self {
            search_paths,
            module_cache: HashMap::new(),
            stdlib_modules: Self::init_stdlib(),
        }
    }

    /// Add a custom search path
    pub fn add_search_path(&mut self, path: PathBuf) {
        self.search_paths.push(path);
    }

    /// Resolve a module path to a Program
    ///
    /// Module paths use dot notation: "std.math" → "std/math.hlxa"
    pub fn resolve(&mut self, module_path: &str) -> Result<Program> {
        // Check if it's a stdlib module
        if let Some(source) = self.stdlib_modules.get(module_path) {
            return self.parse_source(source, module_path);
        }

        // Convert module path to file path
        let file_path = self.module_path_to_file(module_path)?;

        // Check cache
        if let Some(program) = self.module_cache.get(&file_path) {
            return Ok(program.clone());
        }

        // Try to find and load the file
        let source = self.find_and_load(&file_path)?;
        let program = self.parse_source(&source, module_path)?;

        // Cache the result
        self.module_cache.insert(file_path, program.clone());

        Ok(program)
    }

    /// Resolve all imports in a program recursively
    pub fn resolve_imports(&mut self, program: &Program) -> Result<HashMap<String, Program>> {
        let mut resolved = HashMap::new();
        let mut to_visit = VecDeque::new();
        let mut visited = HashSet::new();

        // Start with top-level imports
        for import in &program.imports {
            to_visit.push_back(import.path.clone());
        }

        // Also check module imports
        for module in &program.modules {
            for import in &module.imports {
                to_visit.push_back(import.path.clone());
            }
        }

        // BFS to resolve all transitive dependencies
        while let Some(module_path) = to_visit.pop_front() {
            if visited.contains(&module_path) {
                continue;
            }

            visited.insert(module_path.clone());

            let module_program = self.resolve(&module_path)?;

            // Add this module's imports to the queue
            for import in &module_program.imports {
                if !visited.contains(&import.path) {
                    to_visit.push_back(import.path.clone());
                }
            }

            resolved.insert(module_path, module_program);
        }

        Ok(resolved)
    }

    /// Convert module path to file path
    /// "std.math" → "std/math.hlxa"
    fn module_path_to_file(&self, module_path: &str) -> Result<PathBuf> {
        let mut path = PathBuf::new();

        for segment in module_path.split('.') {
            if segment.is_empty() {
                return Err(HlxError::parse(format!("Invalid module path: {}", module_path)));
            }
            path.push(segment);
        }

        path.set_extension("hlxa");
        Ok(path)
    }

    /// Find a file in search paths and load it
    fn find_and_load(&self, relative_path: &Path) -> Result<String> {
        for search_path in &self.search_paths {
            let full_path = search_path.join(relative_path);

            if full_path.exists() && full_path.is_file() {
                return fs::read_to_string(&full_path)
                    .map_err(|e| HlxError::parse(format!("Failed to read {}: {}", full_path.display(), e)));
            }
        }

        Err(HlxError::parse(format!(
            "Module not found: {} (searched in: {})",
            relative_path.display(),
            self.search_paths.iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }

    /// Parse source code into a Program
    fn parse_source(&self, source: &str, module_name: &str) -> Result<Program> {
        let parser = HlxaParser;
        parser.parse(source)
            .map_err(|e| HlxError::parse(format!("Failed to parse module {}: {}", module_name, e)))
    }

    /// Initialize embedded stdlib modules
    fn init_stdlib() -> HashMap<String, &'static str> {
        let mut stdlib = HashMap::new();

        // std.math - Mathematical operations
        stdlib.insert("std.math".to_string(), r#"
module math {
    fn pi() { return 3.14159265358979323846; }
    fn e() { return 2.71828182845904523536; }
    fn tau() { return 6.28318530717958647692; }

    fn abs(x) {
        if x < 0 {
            return -x;
        } else {
            return x;
        }
    }

    fn max(a, b) {
        if a > b {
            return a;
        } else {
            return b;
        }
    }

    fn min(a, b) {
        if a < b {
            return a;
        } else {
            return b;
        }
    }
}
"#);

        // std.array - Array utilities
        stdlib.insert("std.array".to_string(), r#"
module array {
    fn empty() { return []; }
    fn is_empty(arr) { return len(arr) == 0; }
    fn first(arr) { return arr[0]; }
    fn last(arr) { return arr[len(arr) - 1]; }
}
"#);

        // std.string - String utilities
        stdlib.insert("std.string".to_string(), r#"
module string {
    fn empty() { return ""; }
    fn is_empty(s) { return strlen(s) == 0; }
}
"#);

        stdlib
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
    fn test_module_path_to_file() {
        let resolver = ModuleResolver::new();
        let path = resolver.module_path_to_file("std.math").unwrap();
        assert_eq!(path, PathBuf::from("std/math.hlxa"));
    }

    #[test]
    fn test_nested_module_path() {
        let resolver = ModuleResolver::new();
        let path = resolver.module_path_to_file("mylib.utils.strings").unwrap();
        assert_eq!(path, PathBuf::from("mylib/utils/strings.hlxa"));
    }

    #[test]
    fn test_resolve_stdlib() {
        let mut resolver = ModuleResolver::new();
        let program = resolver.resolve("std.math").unwrap();
        assert_eq!(program.name, "");
        assert!(!program.modules.is_empty());
    }
}
