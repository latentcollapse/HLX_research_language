//! Symbol Index
//!
//! Tracks all symbols (functions, variables, contracts) in the workspace.
//! Enables Go to Definition, Find References, and other navigation features.

use tower_lsp::lsp_types::*;
use dashmap::DashMap;

/// A symbol in the codebase
#[derive(Debug, Clone)]
pub struct Symbol {
    pub name: String,
    pub kind: SymbolKind,
    pub location: Location,
    pub scope: SymbolScope,
    pub detail: Option<String>,
}

/// Symbol scope information
#[derive(Debug, Clone, PartialEq)]
pub enum SymbolScope {
    Global,
    Function(String),  // Function name
    Block(usize),      // Block depth
}

/// Symbol index for the workspace
pub struct SymbolIndex {
    /// All symbols indexed by name
    symbols: DashMap<String, Vec<Symbol>>,

    /// References indexed by symbol name and location
    references: DashMap<String, Vec<Location>>,
}

impl SymbolIndex {
    pub fn new() -> Self {
        Self {
            symbols: DashMap::new(),
            references: DashMap::new(),
        }
    }

    /// Index a document and extract all symbols
    pub fn index_document(&self, uri: &Url, text: &str) {
        // Clear existing symbols for this document
        self.clear_document(uri);

        let mut current_scope = SymbolScope::Global;
        let mut scope_stack: Vec<SymbolScope> = vec![SymbolScope::Global];

        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();

            // 1. Function definitions
            if let Some(symbol) = self.parse_function_def(trimmed, uri, line_idx) {
                current_scope = SymbolScope::Function(symbol.name.clone());
                scope_stack.push(current_scope.clone());
                self.add_symbol(symbol);
            }

            // 2. Variable declarations
            if let Some(symbol) = self.parse_variable_decl(trimmed, uri, line_idx, &current_scope) {
                self.add_symbol(symbol);
            }

            // 3. Contract invocations (references)
            for contract_ref in self.parse_contract_refs(trimmed, uri, line_idx) {
                self.add_reference(&contract_ref.0, contract_ref.1);
            }

            // 4. Variable references
            for var_ref in self.parse_variable_refs(trimmed, uri, line_idx) {
                self.add_reference(&var_ref.0, var_ref.1);
            }

            // Track scope changes
            if trimmed.contains('{') {
                let depth = scope_stack.len();
                scope_stack.push(SymbolScope::Block(depth));
            }

            if trimmed.contains('}') && scope_stack.len() > 1 {
                scope_stack.pop();
                current_scope = scope_stack.last().cloned().unwrap_or(SymbolScope::Global);
            }
        }
    }

    /// Parse function definition
    fn parse_function_def(&self, line: &str, uri: &Url, line_idx: usize) -> Option<Symbol> {
        if !line.starts_with("fn ") {
            return None;
        }

        // Extract: fn NAME(params) {
        let after_fn = line[3..].trim();
        let paren_pos = after_fn.find('(')?;
        let name = after_fn[..paren_pos].trim().to_string();

        // Extract parameters for detail
        let params_end = after_fn.find(')')?;
        let params = &after_fn[paren_pos + 1..params_end];

        Some(Symbol {
            name: name.clone(),
            kind: SymbolKind::FUNCTION,
            location: Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 0,
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: line.len() as u32,
                    },
                },
            },
            scope: SymbolScope::Global,
            detail: Some(format!("fn {}({})", name, params)),
        })
    }

    /// Parse variable declaration
    fn parse_variable_decl(
        &self,
        line: &str,
        uri: &Url,
        line_idx: usize,
        scope: &SymbolScope,
    ) -> Option<Symbol> {
        if !line.starts_with("let ") {
            return None;
        }

        // Extract: let NAME = value;
        let after_let = line[4..].trim();
        let eq_pos = after_let.find('=')?;
        let name = after_let[..eq_pos].trim().to_string();

        Some(Symbol {
            name: name.clone(),
            kind: SymbolKind::VARIABLE,
            location: Location {
                uri: uri.clone(),
                range: Range {
                    start: Position {
                        line: line_idx as u32,
                        character: 4, // After "let "
                    },
                    end: Position {
                        line: line_idx as u32,
                        character: (4 + name.len()) as u32,
                    },
                },
            },
            scope: scope.clone(),
            detail: Some(format!("let {}", name)),
        })
    }

    /// Parse contract references
    fn parse_contract_refs(&self, line: &str, uri: &Url, line_idx: usize) -> Vec<(String, Location)> {
        let mut refs = Vec::new();

        for (i, ch) in line.chars().enumerate() {
            if ch == '@' {
                // Extract contract ID
                let rest = &line[i + 1..];
                let id_len = rest.chars().take_while(|c| c.is_numeric()).count();

                if id_len > 0 {
                    let contract_id = format!("@{}", &rest[..id_len]);

                    refs.push((
                        contract_id,
                        Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: i as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: (i + 1 + id_len) as u32,
                                },
                            },
                        },
                    ));
                }
            }
        }

        refs
    }

    /// Parse variable references
    fn parse_variable_refs(&self, line: &str, uri: &Url, line_idx: usize) -> Vec<(String, Location)> {
        let mut refs = Vec::new();

        // Simple identifier extraction (alphanumeric + underscore)
        let mut current_word = String::new();
        let mut word_start = 0;

        for (i, ch) in line.chars().enumerate() {
            if ch.is_alphanumeric() || ch == '_' {
                if current_word.is_empty() {
                    word_start = i;
                }
                current_word.push(ch);
            } else if !current_word.is_empty() {
                // Skip keywords
                if !self.is_keyword(&current_word) {
                    refs.push((
                        current_word.clone(),
                        Location {
                            uri: uri.clone(),
                            range: Range {
                                start: Position {
                                    line: line_idx as u32,
                                    character: word_start as u32,
                                },
                                end: Position {
                                    line: line_idx as u32,
                                    character: i as u32,
                                },
                            },
                        },
                    ));
                }
                current_word.clear();
            }
        }

        // Handle word at end of line
        if !current_word.is_empty() && !self.is_keyword(&current_word) {
            refs.push((
                current_word,
                Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position {
                            line: line_idx as u32,
                            character: word_start as u32,
                        },
                        end: Position {
                            line: line_idx as u32,
                            character: line.len() as u32,
                        },
                    },
                },
            ));
        }

        refs
    }

    /// Check if a word is a keyword
    fn is_keyword(&self, word: &str) -> bool {
        matches!(
            word,
            "fn" | "let" | "if" | "else" | "loop" | "return" | "break" | "continue"
                | "true" | "false" | "null" | "program"
        )
    }

    /// Add a symbol to the index
    fn add_symbol(&self, symbol: Symbol) {
        let name = symbol.name.clone();
        self.symbols.entry(name).or_insert_with(Vec::new).push(symbol);
    }

    /// Add a reference to the index
    fn add_reference(&self, name: &str, location: Location) {
        self.references
            .entry(name.to_string())
            .or_insert_with(Vec::new)
            .push(location);
    }

    /// Remove all symbols and references for a document (for cleanup on close)
    pub fn remove_document(&self, uri: &Url) {
        self.clear_document(uri);
    }

    /// Clear all symbols for a document
    fn clear_document(&self, uri: &Url) {
        // Remove symbols from this document
        self.symbols.retain(|_, symbols| {
            symbols.retain(|sym| &sym.location.uri != uri);
            !symbols.is_empty()
        });

        // Remove references from this document
        self.references.retain(|_, refs| {
            refs.retain(|loc| &loc.uri != uri);
            !refs.is_empty()
        });
    }

    /// Find the definition of a symbol at a position
    pub fn find_definition(&self, position: &Position, _uri: &Url, text: &str) -> Option<Location> {
        // Get the word at this position
        let word = self.get_word_at_position(text, position)?;

        // Find symbol definition
        if let Some(symbols) = self.symbols.get(&word) {
            // Return the first definition (prefer local scope)
            return symbols.first().map(|sym| sym.location.clone());
        }

        None
    }

    /// Find all references to a symbol
    pub fn find_references(&self, position: &Position, _uri: &Url, text: &str) -> Vec<Location> {
        // Get the word at this position
        let word = match self.get_word_at_position(text, position) {
            Some(w) => w,
            None => return vec![],
        };

        // Get all references
        let mut refs = Vec::new();

        if let Some(locations) = self.references.get(&word) {
            refs.extend(locations.clone());
        }

        // Also include the definition
        if let Some(symbols) = self.symbols.get(&word) {
            for sym in symbols.iter() {
                refs.push(sym.location.clone());
            }
        }

        refs
    }

    /// Get all symbols in a document
    pub fn get_document_symbols(&self, uri: &Url) -> Vec<DocumentSymbol> {
        let mut result = Vec::new();

        for entry in self.symbols.iter() {
            for symbol in entry.value() {
                if &symbol.location.uri == uri {
                    result.push(DocumentSymbol {
                        name: symbol.name.clone(),
                        detail: symbol.detail.clone(),
                        kind: symbol.kind,
                        tags: None,
                        range: symbol.location.range,
                        selection_range: symbol.location.range,
                        children: None,
                        #[allow(deprecated)]
                        deprecated: None,
                    });
                }
            }
        }

        // Sort by line number
        result.sort_by_key(|sym| sym.range.start.line);

        result
    }

    /// Search symbols across workspace
    pub fn search_symbols(&self, query: &str) -> Vec<SymbolInformation> {
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        for entry in self.symbols.iter() {
            let name = entry.key();
            if name.to_lowercase().contains(&query_lower) {
                for symbol in entry.value() {
                    results.push(SymbolInformation {
                        name: symbol.name.clone(),
                        kind: symbol.kind,
                        tags: None,
                        location: symbol.location.clone(),
                        container_name: None,
                        #[allow(deprecated)]
                        deprecated: None,
                    });
                }
            }
        }

        results
    }

    /// Get word at position
    fn get_word_at_position(&self, text: &str, position: &Position) -> Option<String> {
        let line = text.lines().nth(position.line as usize)?;
        let char_idx = position.character as usize;

        if char_idx >= line.len() {
            return None;
        }

        // Find start of word
        let start = line[..char_idx]
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| i + 1)
            .unwrap_or(0);

        // Find end of word
        let end = line[char_idx..]
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|i| char_idx + i)
            .unwrap_or(line.len());

        if start < end {
            Some(line[start..end].to_string())
        } else {
            None
        }
    }
}

impl Default for SymbolIndex {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_indexing() {
        let index = SymbolIndex::new();
        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "fn add(a, b) {\n    return a + b;\n}";

        index.index_document(&uri, code);

        let symbols = index.symbols.get("add").unwrap();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].kind, SymbolKind::FUNCTION);
    }

    #[test]
    fn test_variable_indexing() {
        let index = SymbolIndex::new();
        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "let x = 42;\nlet y = x + 10;";

        index.index_document(&uri, code);

        assert!(index.symbols.contains_key("x"));
        assert!(index.symbols.contains_key("y"));
    }

    #[test]
    fn test_find_definition() {
        let index = SymbolIndex::new();
        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "let x = 42;\nlet y = x;";

        index.index_document(&uri, code);

        // Find definition of 'x' at line 1
        let pos = Position { line: 1, character: 8 };
        let def = index.find_definition(&pos, &uri, code);

        assert!(def.is_some());
        let location = def.unwrap();
        assert_eq!(location.range.start.line, 0); // Defined on line 0
    }

    #[test]
    fn test_find_references() {
        let index = SymbolIndex::new();
        let uri = Url::parse("file:///test.hlxa").unwrap();
        let code = "let x = 42;\nlet y = x;\nlet z = x;";

        index.index_document(&uri, code);

        // Find references to 'x'
        let pos = Position { line: 0, character: 4 };
        let refs = index.find_references(&pos, &uri, code);

        // Should find definition + 2 references = 3 total
        assert!(refs.len() >= 3);
    }
}
