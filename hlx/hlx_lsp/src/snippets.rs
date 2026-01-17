use tower_lsp::lsp_types::*;
use std::collections::HashMap;

/// Context in which a snippet is being requested
#[derive(Debug, Clone, PartialEq)]
pub enum SnippetContext {
    /// At the top level of a program (outside functions)
    TopLevel,
    /// Inside a function body
    FunctionBody,
    /// After a let statement (suggesting what to do with the variable)
    AfterLet,
    /// In an expression position
    Expression,
    /// At the start of a line (statement position)
    StatementStart,
}

/// A code snippet template
#[derive(Debug, Clone)]
pub struct Snippet {
    /// Trigger text (what the user types to activate)
    pub trigger: String,
    /// Display label shown in completion menu
    pub label: String,
    /// Detailed description of what this snippet does
    pub description: String,
    /// The template text with placeholders
    /// Uses VSCode snippet format: ${1:default}, ${2:name}, $0 (final cursor position)
    pub template: String,
    /// Contexts where this snippet is valid
    pub valid_contexts: Vec<SnippetContext>,
    /// Sort priority (lower = appears first)
    pub priority: u32,
}

/// Provides context-aware code snippets
pub struct SnippetProvider {
    snippets: Vec<Snippet>,
    /// Usage count for ranking (trigger -> count)
    usage_stats: HashMap<String, u32>,
}

impl SnippetProvider {
    pub fn new() -> Self {
        let mut provider = Self {
            snippets: Vec::new(),
            usage_stats: HashMap::new(),
        };

        provider.register_default_snippets();
        provider
    }

    /// Register all default HLX snippets
    fn register_default_snippets(&mut self) {
        // Function snippet
        self.add_snippet(Snippet {
            trigger: "fn".to_string(),
            label: "fn - Function definition".to_string(),
            description: "Create a new function with parameters and body".to_string(),
            template: "fn ${1:name}(${2:params}) {\n    ${0:// body}\n}".to_string(),
            valid_contexts: vec![SnippetContext::TopLevel, SnippetContext::StatementStart],
            priority: 10,
        });

        // Contract creation snippet
        self.add_snippet(Snippet {
            trigger: "@contract".to_string(),
            label: "@contract - Contract definition".to_string(),
            description: "Create a contract with field definitions".to_string(),
            template: "@${1:14} {\n    @0: ${2:field1},\n    @1: ${3:field2},\n    @2: ${4:field3}\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::Expression, SnippetContext::StatementStart],
            priority: 15,
        });

        // Error handling pattern
        self.add_snippet(Snippet {
            trigger: "@try".to_string(),
            label: "@try - Error handling pattern".to_string(),
            description: "Add error handling for a result value".to_string(),
            template: "if (${1:result}.is_error()) {\n    ${2:handle_error}(${1:result}.error);\n    return ${3:default_value};\n}\nlet ${4:value} = ${1:result}.unwrap();$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 20,
        });

        // Latent space transaction
        self.add_snippet(Snippet {
            trigger: "@lstx".to_string(),
            label: "@lstx - Latent space transaction".to_string(),
            description: "Create a latent space transaction block".to_string(),
            template: "ls.transaction {\n    let handle = ls.collapse ${1:table} ${2:namespace} ${3:value};\n    ${4:// operations}\n    ls.resolve handle;\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 25,
        });

        // If-else statement
        self.add_snippet(Snippet {
            trigger: "if".to_string(),
            label: "if - Conditional statement".to_string(),
            description: "If statement with optional else block".to_string(),
            template: "if (${1:condition}) {\n    ${2:// then}\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 30,
        });

        // If-else with both branches
        self.add_snippet(Snippet {
            trigger: "ifelse".to_string(),
            label: "ifelse - If-else statement".to_string(),
            description: "If statement with else block".to_string(),
            template: "if (${1:condition}) {\n    ${2:// then}\n} else {\n    ${3:// else}\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 31,
        });

        // Loop statement
        self.add_snippet(Snippet {
            trigger: "loop".to_string(),
            label: "loop - Infinite loop".to_string(),
            description: "Create an infinite loop with break condition".to_string(),
            template: "loop {\n    ${1:// body}\n    if (${2:condition}) {\n        break;\n    }\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 35,
        });

        // Let statement
        self.add_snippet(Snippet {
            trigger: "let".to_string(),
            label: "let - Variable declaration".to_string(),
            description: "Declare a new variable".to_string(),
            template: "let ${1:name} = ${2:value};$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 40,
        });

        // Let with type annotation
        self.add_snippet(Snippet {
            trigger: "lett".to_string(),
            label: "lett - Typed variable declaration".to_string(),
            description: "Declare a variable with explicit type".to_string(),
            template: "let ${1:name}: ${2:Type} = ${3:value};$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 41,
        });

        // Return statement
        self.add_snippet(Snippet {
            trigger: "ret".to_string(),
            label: "ret - Return statement".to_string(),
            description: "Return a value from the function".to_string(),
            template: "return ${1:value};$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 45,
        });

        // Array creation
        self.add_snippet(Snippet {
            trigger: "arr".to_string(),
            label: "arr - Array literal".to_string(),
            description: "Create an array with elements".to_string(),
            template: "[${1:element1}, ${2:element2}, ${3:element3}]$0".to_string(),
            valid_contexts: vec![SnippetContext::Expression, SnippetContext::StatementStart],
            priority: 50,
        });

        // Print debugging
        self.add_snippet(Snippet {
            trigger: "pr".to_string(),
            label: "pr - Print debug".to_string(),
            description: "Print a value for debugging".to_string(),
            template: "print(${1:value});$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 55,
        });

        // Program wrapper
        self.add_snippet(Snippet {
            trigger: "prog".to_string(),
            label: "prog - Program definition".to_string(),
            description: "Create a program with main function".to_string(),
            template: "program ${1:name} {\n    fn main() {\n        ${0:// body}\n    }\n}".to_string(),
            valid_contexts: vec![SnippetContext::TopLevel],
            priority: 5,
        });

        // Match-like if-else chain
        self.add_snippet(Snippet {
            trigger: "match".to_string(),
            label: "match - Multi-way branch".to_string(),
            description: "Create a multi-way conditional (match-like)".to_string(),
            template: "if (${1:value} == ${2:case1}) {\n    ${3:// case 1}\n} else if (${1:value} == ${4:case2}) {\n    ${5:// case 2}\n} else {\n    ${6:// default}\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::FunctionBody, SnippetContext::StatementStart],
            priority: 32,
        });

        // Test function
        self.add_snippet(Snippet {
            trigger: "test".to_string(),
            label: "test - Test function".to_string(),
            description: "Create a test function with assertions".to_string(),
            template: "fn test_${1:name}() {\n    let result = ${2:function_to_test}();\n    assert(result == ${3:expected});\n}$0".to_string(),
            valid_contexts: vec![SnippetContext::TopLevel],
            priority: 60,
        });
    }

    /// Add a snippet to the provider
    pub fn add_snippet(&mut self, snippet: Snippet) {
        self.snippets.push(snippet);
    }

    /// Detect the context at a given position in the text
    pub fn detect_context(&self, text: &str, position: Position) -> SnippetContext {
        let line_idx = position.line as usize;
        let lines: Vec<&str> = text.lines().collect();

        if line_idx >= lines.len() {
            return SnippetContext::TopLevel;
        }

        // Check if we're at the top level (no function wrapper above)
        let mut in_function = false;
        let mut brace_depth = 0;

        for (idx, line) in lines.iter().enumerate() {
            if idx > line_idx {
                break;
            }

            let trimmed = line.trim();

            // Track function definitions
            if trimmed.starts_with("fn ") {
                in_function = true;
            }

            // Track brace depth
            brace_depth += line.matches('{').count() as i32;
            brace_depth -= line.matches('}').count() as i32;

            // If we're back to depth 0, we're out of the function
            if in_function && brace_depth == 0 && trimmed.ends_with('}') {
                in_function = false;
            }
        }

        // Check current line context
        let current_line = lines[line_idx].trim();

        // After a let statement
        if line_idx > 0 {
            let prev_line = lines[line_idx - 1].trim();
            if prev_line.starts_with("let ") && prev_line.ends_with(';') {
                return SnippetContext::AfterLet;
            }
        }

        // At the start of a line (statement position)
        if position.character <= current_line.len().saturating_sub(current_line.trim_start().len()) as u32 + 1 {
            if in_function && brace_depth > 0 {
                return SnippetContext::FunctionBody;
            } else if !in_function {
                return SnippetContext::TopLevel;
            } else {
                return SnippetContext::StatementStart;
            }
        }

        // Inside an expression
        if in_function && brace_depth > 0 {
            SnippetContext::FunctionBody
        } else {
            SnippetContext::TopLevel
        }
    }

    /// Get snippets that match the given prefix and context
    pub fn get_completions(
        &self,
        prefix: &str,
        context: SnippetContext,
    ) -> Vec<CompletionItem> {
        let prefix_lower = prefix.to_lowercase();

        let mut items: Vec<CompletionItem> = self
            .snippets
            .iter()
            .filter(|snippet| {
                // Match prefix
                snippet.trigger.to_lowercase().starts_with(&prefix_lower)
                    && snippet.valid_contexts.contains(&context)
            })
            .map(|snippet| {
                let usage_count = self.usage_stats.get(&snippet.trigger).copied().unwrap_or(0);

                CompletionItem {
                    label: snippet.label.clone(),
                    kind: Some(CompletionItemKind::SNIPPET),
                    detail: Some(snippet.description.clone()),
                    documentation: Some(Documentation::String(format!(
                        "Trigger: `{}`\n\n{}",
                        snippet.trigger, snippet.description
                    ))),
                    insert_text: Some(snippet.template.clone()),
                    insert_text_format: Some(InsertTextFormat::SNIPPET),
                    // Lower sort text = higher priority
                    sort_text: Some(format!("{:04}_{}", snippet.priority - usage_count, snippet.trigger)),
                    filter_text: Some(snippet.trigger.clone()),
                    ..Default::default()
                }
            })
            .collect();

        // Sort by priority (already encoded in sort_text)
        items.sort_by(|a, b| a.sort_text.cmp(&b.sort_text));

        items
    }

    /// Record that a snippet was used (for ranking)
    pub fn record_usage(&mut self, trigger: &str) {
        *self.usage_stats.entry(trigger.to_string()).or_insert(0) += 1;
    }

    /// Get the number of registered snippets
    pub fn count(&self) -> usize {
        self.snippets.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_snippet_provider_creation() {
        let provider = SnippetProvider::new();
        assert!(provider.count() > 0);
    }

    #[test]
    fn test_context_detection_top_level() {
        let provider = SnippetProvider::new();
        let text = "program test {\n\n}";
        let position = Position { line: 0, character: 0 };
        let context = provider.detect_context(text, position);
        assert_eq!(context, SnippetContext::TopLevel);
    }

    #[test]
    fn test_context_detection_function_body() {
        let provider = SnippetProvider::new();
        let text = "program test {\n    fn main() {\n        \n    }\n}";
        let position = Position { line: 2, character: 8 };
        let context = provider.detect_context(text, position);
        assert_eq!(context, SnippetContext::FunctionBody);
    }

    #[test]
    fn test_get_completions_filter_by_prefix() {
        let provider = SnippetProvider::new();
        let completions = provider.get_completions("fn", SnippetContext::TopLevel);

        // Should find "fn" snippet
        assert!(completions.iter().any(|c| c.filter_text.as_ref().unwrap() == "fn"));
    }

    #[test]
    fn test_get_completions_filter_by_context() {
        let provider = SnippetProvider::new();

        // "loop" should be available in function body
        let completions = provider.get_completions("loop", SnippetContext::FunctionBody);
        assert!(completions.iter().any(|c| c.filter_text.as_ref().unwrap() == "loop"));

        // But not at top level
        let completions = provider.get_completions("loop", SnippetContext::TopLevel);
        assert!(!completions.iter().any(|c| c.filter_text.as_ref().unwrap() == "loop"));
    }

    #[test]
    fn test_usage_tracking() {
        let mut provider = SnippetProvider::new();

        provider.record_usage("fn");
        provider.record_usage("fn");

        assert_eq!(provider.usage_stats.get("fn"), Some(&2));
    }
}
