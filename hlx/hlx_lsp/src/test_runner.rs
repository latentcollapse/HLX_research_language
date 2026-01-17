//! Test Runner Integration
//!
//! Integrates HLX test execution into the LSP.
//! Provides CodeLens "Run Test" buttons and inline test results.

use serde::{Deserialize, Serialize};
use tower_lsp::lsp_types::{CodeLens, Command, Position, Range};
use hlx_compiler::ast::{Block, Program};
use std::collections::HashMap;

/// Test function metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestFunction {
    pub name: String,
    pub range: Range,
    pub test_type: TestType,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestType {
    Unit,
    Integration,
    Contract,
    Performance,
}

/// Test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub test_name: String,
    pub status: TestStatus,
    pub duration_ms: u64,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Running,
}

/// Test runner provider
pub struct TestRunnerProvider {
    /// Discovered tests
    tests: HashMap<String, Vec<TestFunction>>,
    /// Recent test results
    results: HashMap<String, TestResult>,
}

impl TestRunnerProvider {
    pub fn new() -> Self {
        Self {
            tests: HashMap::new(),
            results: HashMap::new(),
        }
    }

    /// Discover tests in a program
    pub fn discover_tests(&mut self, uri: &str, program: &Program) {
        let mut tests = Vec::new();

        for block in &program.blocks {
            if let Some(test_func) = self.check_if_test(block) {
                tests.push(test_func);
            }
        }

        self.tests.insert(uri.to_string(), tests);
    }

    /// Check if a block is a test function
    fn check_if_test(&self, block: &Block) -> Option<TestFunction> {
        let name = &block.name;

        // Test functions start with "test_"
        if !name.starts_with("test_") {
            return None;
        }

        // Determine test type from name
        let test_type = if name.contains("integration") {
            TestType::Integration
        } else if name.contains("contract") {
            TestType::Contract
        } else if name.contains("perf") || name.contains("bench") {
            TestType::Performance
        } else {
            TestType::Unit
        };

        Some(TestFunction {
            name: name.clone(),
            range: Range {
                start: Position { line: 0, character: 0 }, // TODO: Get from span
                end: Position { line: 0, character: 0 },
            },
            test_type,
        })
    }

    /// Generate CodeLens for tests
    pub fn generate_code_lenses(&self, uri: &str) -> Vec<CodeLens> {
        let mut lenses = Vec::new();

        if let Some(tests) = self.tests.get(uri) {
            for test in tests {
                // "Run Test" lens
                lenses.push(CodeLens {
                    range: test.range,
                    command: Some(Command {
                        title: format!("▶ Run Test"),
                        command: "hlx.runTest".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(uri),
                            serde_json::json!(test.name),
                        ]),
                    }),
                    data: None,
                });

                // "Debug Test" lens
                lenses.push(CodeLens {
                    range: test.range,
                    command: Some(Command {
                        title: "🐛 Debug".to_string(),
                        command: "hlx.debugTest".to_string(),
                        arguments: Some(vec![
                            serde_json::json!(uri),
                            serde_json::json!(test.name),
                        ]),
                    }),
                    data: None,
                });

                // Show last result if available
                if let Some(result) = self.results.get(&test.name) {
                    let status_icon = match result.status {
                        TestStatus::Passed => "✅",
                        TestStatus::Failed => "❌",
                        TestStatus::Skipped => "⊘",
                        TestStatus::Running => "⏳",
                    };

                    lenses.push(CodeLens {
                        range: test.range,
                        command: Some(Command {
                            title: format!("{} {}ms", status_icon, result.duration_ms),
                            command: "hlx.showTestResult".to_string(),
                            arguments: Some(vec![serde_json::json!(test.name)]),
                        }),
                        data: None,
                    });
                }
            }
        }

        lenses
    }

    /// Get all tests in a file
    pub fn get_tests(&self, uri: &str) -> Option<&Vec<TestFunction>> {
        self.tests.get(uri)
    }

    /// Store test result
    pub fn store_result(&mut self, result: TestResult) {
        self.results.insert(result.test_name.clone(), result);
    }

    /// Get test result
    pub fn get_result(&self, test_name: &str) -> Option<&TestResult> {
        self.results.get(test_name)
    }

    /// Run all tests in a file (placeholder)
    pub fn run_tests(&mut self, uri: &str) -> Vec<TestResult> {
        let mut results = Vec::new();

        if let Some(tests) = self.tests.get(uri) {
            // Clone to avoid borrow checker issues
            let test_names: Vec<String> = tests.iter().map(|t| t.name.clone()).collect();

            for test_name in test_names {
                // TODO: Actually execute test
                let result = TestResult {
                    test_name: test_name.clone(),
                    status: TestStatus::Passed, // Mock result
                    duration_ms: 10,
                    message: None,
                };

                self.store_result(result.clone());
                results.push(result);
            }
        }

        results
    }

    /// Check if a function name looks like a test
    fn check_if_test_name(&self, name: &str) -> bool {
        name.starts_with("test_")
    }
}

impl Default for TestRunnerProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection() {
        // Simplified test - just test the detection logic
        let provider = TestRunnerProvider::new();

        // Test that test_ prefix is recognized
        assert!(provider.check_if_test_name("test_addition"));
        assert!(provider.check_if_test_name("test_integration_flow"));
        assert!(!provider.check_if_test_name("regular_function"));
    }
}
