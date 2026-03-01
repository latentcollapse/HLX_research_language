//! Policy Loading Module
//!
//! Provides policy loading without execution. Policies are parsed from .axm files
//! and provide declarations (intents, contracts, enums) that can be used for
//! verification or execution.
//!
//! This is the "config with teeth" layer - policy files are the product.

use std::path::Path;
use std::collections::HashMap;
use crate::error::{AxiomError, AxiomResult};
use crate::lexer::Lexer;
use crate::parser::Parser;
use crate::parser::ast::{IntentDecl, ContractDecl, EnumDecl, Item};
use crate::checker::TypeChecker;

/// A loaded policy containing declarations but no execution state
#[derive(Debug, Clone)]
pub struct Policy {
    /// Intent declarations from the policy
    pub intents: HashMap<String, IntentDecl>,
    /// Contract declarations from the policy
    pub contracts: HashMap<String, ContractDecl>,
    /// Enum declarations from the policy
    pub enums: HashMap<String, EnumDecl>,
    /// Module name
    pub module_name: String,
}

impl Policy {
    /// Get a list of all intent names in this policy
    pub fn intent_names(&self) -> Vec<&str> {
        self.intents.keys().map(|s| s.as_str()).collect()
    }

    /// Get an intent declaration by name
    pub fn get_intent(&self, name: &str) -> Option<&IntentDecl> {
        self.intents.get(name)
    }

    /// Get a contract declaration by name
    pub fn get_contract(&self, name: &str) -> Option<&ContractDecl> {
        self.contracts.get(name)
    }

    /// Get an enum declaration by name
    pub fn get_enum(&self, name: &str) -> Option<&EnumDecl> {
        self.enums.get(name)
    }
}

/// Policy loader - loads and parses .axm files without execution
pub struct PolicyLoader;

impl PolicyLoader {
    /// Load a policy from a file path
    ///
    /// # Example
    /// ```no_run
    /// use axiom_lang::policy::PolicyLoader;
    ///
    /// let policy = PolicyLoader::load_file("security.axm")?;
    /// println!("Loaded {} intents", policy.intents.len());
    /// # Ok::<(), axiom_lang::error::AxiomError>(())
    /// ```
    pub fn load_file(path: impl AsRef<Path>) -> AxiomResult<Policy> {
        let path = path.as_ref();
        let source = std::fs::read_to_string(path).map_err(|e| AxiomError {
            kind: crate::error::ErrorKind::HaltUnknown,
            message: format!("Failed to read policy file {}: {}", path.display(), e),
            span: None,
        })?;

        Self::load_source(&source)
    }

    /// Load a policy from source code string
    ///
    /// # Example
    /// ```no_run
    /// use axiom_lang::policy::PolicyLoader;
    ///
    /// let source = r#"
    ///     module my_policy {
    ///         intent ReadFile {
    ///             takes: path: String;
    ///             gives: content: String;
    ///             effect: READ;
    ///             conscience: path_safety;
    ///         }
    ///     }
    /// "#;
    ///
    /// let policy = PolicyLoader::load_source(source)?;
    /// # Ok::<(), axiom_lang::error::AxiomError>(())
    /// ```
    pub fn load_source(source: &str) -> AxiomResult<Policy> {
        // Step 1: Lex
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize()?;

        // Step 2: Parse
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program()?;

        // Step 3: Type check
        let mut checker = TypeChecker::new();
        checker.check_program(&program)?;

        // Step 4: Extract declarations into Policy
        let mut intents = HashMap::new();
        let mut contracts = HashMap::new();
        let mut enums = HashMap::new();

        for item in &program.module.items {
            match item {
                Item::Intent(intent) => {
                    intents.insert(intent.name.clone(), intent.clone());
                }
                Item::Contract(contract) => {
                    contracts.insert(contract.name.clone(), contract.clone());
                }
                Item::Enum(enum_decl) => {
                    enums.insert(enum_decl.name.clone(), enum_decl.clone());
                }
                // We don't need functions for pure verification, but they're in the AST
                _ => {}
            }
        }

        Ok(Policy {
            intents,
            contracts,
            enums,
            module_name: program.module.name.clone(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_simple_policy() {
        let source = r#"
            module test_policy {
                intent ReadFile {
                    takes: path: String;
                    gives: content: String;
                    effect: READ;
                    conscience: path_safety;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        assert_eq!(policy.module_name, "test_policy");
        assert_eq!(policy.intents.len(), 1);
        assert!(policy.intents.contains_key("ReadFile"));
    }

    #[test]
    fn test_load_policy_with_multiple_intents() {
        let source = r#"
            module multi_intent {
                intent ReadFile {
                    takes: path: String;
                    gives: content: String;
                    effect: READ;
                }

                intent WriteFile {
                    takes: path: String, content: String;
                    gives: success: bool;
                    effect: WRITE;
                    conscience: path_safety, no_exfiltrate;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        assert_eq!(policy.intents.len(), 2);
        assert!(policy.intents.contains_key("ReadFile"));
        assert!(policy.intents.contains_key("WriteFile"));
    }

    #[test]
    fn test_policy_with_contract() {
        let source = r#"
            module with_contract {
                contract Point {
                    @0: x: f64,
                    @1: y: f64,
                }

                intent ProcessPoint {
                    takes: p: Point;
                    gives: result: f64;
                    effect: NOOP;
                }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        assert_eq!(policy.contracts.len(), 1);
        assert!(policy.contracts.contains_key("Point"));
        assert_eq!(policy.intents.len(), 1);
    }

    #[test]
    fn test_intent_names() {
        let source = r#"
            module test {
                intent A { takes: x: i64; gives: y: i64; effect: NOOP; }
                intent B { takes: x: i64; gives: y: i64; effect: NOOP; }
                intent C { takes: x: i64; gives: y: i64; effect: NOOP; }
            }
        "#;

        let policy = PolicyLoader::load_source(source).unwrap();
        let mut names = policy.intent_names();
        names.sort();
        assert_eq!(names, vec!["A", "B", "C"]);
    }
}
