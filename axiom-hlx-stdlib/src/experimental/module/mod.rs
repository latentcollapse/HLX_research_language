//! Module Resolution System (Section 5.1)
//!
//! Parses `axiom.project` manifests, resolves import paths,
//! and supports multi-file compilation with deterministic ordering.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use crate::error::{AxiomError, AxiomResult, ErrorKind};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::Program;

/// Parsed project manifest from axiom.project
#[derive(Debug, Clone)]
pub struct ProjectManifest {
    pub name: String,
    pub version: String,
    pub axiom_version: String,
    pub modules: HashMap<String, String>,
    pub inference_mode: String,
    pub scale_max_agents: u64,
    pub scale_mode: String,
    pub default_time_bound: String,
    pub default_memory_bound: String,
}

impl Default for ProjectManifest {
    fn default() -> Self {
        ProjectManifest {
            name: "unnamed".to_string(),
            version: "0.1.0".to_string(),
            axiom_version: "2.4".to_string(),
            modules: HashMap::new(),
            inference_mode: "guard".to_string(),
            scale_max_agents: 200,
            scale_mode: "independent".to_string(),
            default_time_bound: "5000ms".to_string(),
            default_memory_bound: "64mb".to_string(),
        }
    }
}

/// Parse a TOML-like axiom.project manifest
pub fn parse_manifest(content: &str) -> AxiomResult<ProjectManifest> {
    let mut manifest = ProjectManifest::default();
    let mut current_section = String::new();

    for line in content.lines() {
        let line = line.trim();

        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        // Section header
        if line.starts_with('[') && line.ends_with(']') {
            current_section = line[1..line.len() - 1].to_string();
            continue;
        }

        // Key = value
        if let Some(eq_pos) = line.find('=') {
            let key = line[..eq_pos].trim().to_string();
            let raw_val = line[eq_pos + 1..].trim();
            // Strip quotes
            let value = if raw_val.starts_with('"') && raw_val.ends_with('"') {
                raw_val[1..raw_val.len() - 1].to_string()
            } else {
                raw_val.to_string()
            };

            match current_section.as_str() {
                "project" => match key.as_str() {
                    "name" => manifest.name = value,
                    "version" => manifest.version = value,
                    "axiom_version" => manifest.axiom_version = value,
                    _ => {}
                },
                "modules" => {
                    manifest.modules.insert(key, value);
                }
                "inference" => match key.as_str() {
                    "default_mode" => manifest.inference_mode = value,
                    _ => {}
                },
                "scale" => match key.as_str() {
                    "max_agents" => {
                        manifest.scale_max_agents = value.parse().unwrap_or(200);
                    }
                    "mode" => manifest.scale_mode = value,
                    _ => {}
                },
                "bounds" => match key.as_str() {
                    "default_time" => manifest.default_time_bound = value,
                    "default_memory" => manifest.default_memory_bound = value,
                    _ => {}
                },
                _ => {}
            }
        }
    }

    Ok(manifest)
}

/// Resolved module — a parsed program with its source path
#[derive(Debug)]
pub struct ResolvedModule {
    pub name: String,
    pub path: PathBuf,
    pub program: Program,
}

/// Module resolver — loads and resolves multi-file Axiom projects
pub struct ModuleResolver {
    /// Base directory for relative path resolution
    base_dir: PathBuf,
    /// Project manifest (if available)
    pub manifest: Option<ProjectManifest>,
    /// Cache of already-resolved modules (path -> program)
    resolved: HashMap<String, ResolvedModule>,
}

impl ModuleResolver {
    pub fn new(base_dir: &Path) -> Self {
        ModuleResolver {
            base_dir: base_dir.to_path_buf(),
            manifest: None,
            resolved: HashMap::new(),
        }
    }

    /// Load a project manifest from the given path
    pub fn load_manifest(&mut self, manifest_path: &Path) -> AxiomResult<()> {
        let content = std::fs::read_to_string(manifest_path).map_err(|e| AxiomError {
            kind: ErrorKind::UndefinedFunction,
            message: format!("Cannot read manifest '{}': {}", manifest_path.display(), e),
            span: None,
        })?;
        self.manifest = Some(parse_manifest(&content)?);
        Ok(())
    }

    /// Resolve an import path to an absolute file path
    pub fn resolve_path(&self, import_path: &str) -> PathBuf {
        // Check manifest module aliases first
        if let Some(ref manifest) = self.manifest {
            for (alias, path) in &manifest.modules {
                if import_path == alias || import_path.ends_with(path) {
                    return self.base_dir.join(path);
                }
            }
        }

        // Direct relative path resolution
        if import_path.ends_with(".axm") {
            self.base_dir.join(import_path)
        } else {
            self.base_dir.join(format!("{}.axm", import_path))
        }
    }

    /// Load and parse a single .axm file
    pub fn load_module(&mut self, path: &Path) -> AxiomResult<&ResolvedModule> {
        let path_str = path.display().to_string();

        if self.resolved.contains_key(&path_str) {
            return Ok(&self.resolved[&path_str]);
        }

        let source = std::fs::read_to_string(path).map_err(|e| AxiomError {
            kind: ErrorKind::UndefinedFunction,
            message: format!("Cannot read module '{}': {}", path.display(), e),
            span: None,
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;

        let module_name = program.module.name.clone();
        self.resolved.insert(
            path_str.clone(),
            ResolvedModule {
                name: module_name,
                path: path.to_path_buf(),
                program,
            },
        );

        Ok(&self.resolved[&path_str])
    }

    /// Load the main file and all its transitive imports
    pub fn load_with_imports(&mut self, main_path: &Path) -> AxiomResult<Vec<String>> {
        let mut load_order = Vec::new();
        self.load_recursive(main_path, &mut load_order, &mut Vec::new())?;
        Ok(load_order)
    }

    fn load_recursive(
        &mut self,
        path: &Path,
        load_order: &mut Vec<String>,
        visited: &mut Vec<String>,
    ) -> AxiomResult<()> {
        let path_str = path.display().to_string();
        if visited.contains(&path_str) {
            return Ok(()); // Already visited — no cycles
        }
        visited.push(path_str.clone());

        // Load the module
        let source = std::fs::read_to_string(path).map_err(|e| AxiomError {
            kind: ErrorKind::UndefinedFunction,
            message: format!("Cannot read module '{}': {}", path.display(), e),
            span: None,
        })?;

        let mut lexer = Lexer::new(&source);
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;

        // Process imports first (depth-first)
        let imports: Vec<String> = program
            .module
            .items
            .iter()
            .filter_map(|item| {
                if let crate::parser::ast::Item::Import(imp) = item {
                    Some(imp.path.clone())
                } else {
                    None
                }
            })
            .collect();

        for import_path in imports {
            let resolved_path = self.resolve_path(&import_path);
            if resolved_path.exists() {
                self.load_recursive(&resolved_path, load_order, visited)?;
            }
        }

        // Store the module
        let module_name = program.module.name.clone();
        self.resolved.insert(
            path_str.clone(),
            ResolvedModule {
                name: module_name,
                path: path.to_path_buf(),
                program,
            },
        );
        load_order.push(path_str);

        Ok(())
    }

    /// Get a resolved module by path
    pub fn get_module(&self, path_str: &str) -> Option<&ResolvedModule> {
        self.resolved.get(path_str)
    }

    /// Get all resolved modules in load order
    pub fn modules(&self) -> &HashMap<String, ResolvedModule> {
        &self.resolved
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_manifest() {
        let content = r#"
# Axiom Project Manifest
[project]
name = "test-project"
version = "1.0.0"
axiom_version = "2.4"

[modules]
std_io = "stdlib/io.axm"
std_tensor = "stdlib/tensor.axm"

[inference]
default_mode = "arx"

[scale]
max_agents = 100
mode = "collaborative"

[bounds]
default_time = "3000ms"
default_memory = "128mb"
"#;
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.name, "test-project");
        assert_eq!(manifest.version, "1.0.0");
        assert_eq!(manifest.axiom_version, "2.4");
        assert_eq!(manifest.modules.len(), 2);
        assert_eq!(manifest.modules["std_io"], "stdlib/io.axm");
        assert_eq!(manifest.inference_mode, "arx");
        assert_eq!(manifest.scale_max_agents, 100);
        assert_eq!(manifest.scale_mode, "collaborative");
        assert_eq!(manifest.default_time_bound, "3000ms");
        assert_eq!(manifest.default_memory_bound, "128mb");
    }

    #[test]
    fn test_parse_minimal_manifest() {
        let content = r#"
[project]
name = "minimal"
"#;
        let manifest = parse_manifest(content).unwrap();
        assert_eq!(manifest.name, "minimal");
        assert_eq!(manifest.version, "0.1.0"); // default
        assert_eq!(manifest.inference_mode, "guard"); // default
    }

    #[test]
    fn test_path_resolution() {
        let resolver = ModuleResolver::new(Path::new("/project"));
        assert_eq!(
            resolver.resolve_path("stdlib/io.axm"),
            PathBuf::from("/project/stdlib/io.axm")
        );
        assert_eq!(
            resolver.resolve_path("mymod"),
            PathBuf::from("/project/mymod.axm")
        );
    }

    #[test]
    fn test_manifest_with_module_alias_resolution() {
        let mut resolver = ModuleResolver::new(Path::new("/project"));
        let content = r#"
[modules]
std_io = "stdlib/io.axm"
"#;
        resolver.manifest = Some(parse_manifest(content).unwrap());
        assert_eq!(
            resolver.resolve_path("std_io"),
            PathBuf::from("/project/stdlib/io.axm")
        );
    }
}
