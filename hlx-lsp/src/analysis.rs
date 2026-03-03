use hlx_runtime::AstParser;
use tower_lsp::lsp_types::*;

/// Get diagnostics from parsing HLX code
pub fn get_diagnostics(content: &str) -> Vec<Diagnostic> {
    // Try to parse the program
    match AstParser::parse(content) {
        Ok(_) => vec![], // No errors
        Err(e) => {
            // Convert ParseError to Diagnostic
            vec![Diagnostic {
                range: Range {
                    start: Position { line: 0, character: 0 },
                    end: Position { line: 0, character: 1 },
                },
                severity: Some(DiagnosticSeverity::ERROR),
                code: None,
                code_description: None,
                source: Some("hlx".to_string()),
                message: format!("Parse error: {:?}", e),
                related_information: None,
                tags: None,
                data: None,
            }]
        }
    }
}

/// Get hover information at a position
pub fn get_hover_info(content: &str, line: u32, character: u32) -> Option<String> {
    let word = get_word_at_position(content, line, character)?;
    
    // Check builtins
    if let Some(desc) = get_builtin_description(&word) {
        return Some(desc);
    }
    
    // Check for keywords
    let keywords = vec![
        ("agent", "Defines a recursive agent with governance"),
        ("recursive", "Marks an agent as recursively self-improving"),
        ("fn", "Defines a function"),
        ("let", "Defines a variable"),
        ("if", "Conditional statement"),
        ("else", "Alternative branch for if"),
        ("loop", "Loop with condition and max iterations"),
        ("return", "Return from function"),
        ("import", "Import from another module"),
        ("export", "Export for other modules"),
        ("struct", "Define a data structure"),
        ("module", "Define a module"),
        ("govern", "Governance block"),
        ("scale", "Multi-agent coordination"),
        ("cycle", "Reasoning cycle (H/L)"),
        ("modify", "Self-modification block"),
        ("proof", "Proof gate"),
        ("consensus", "Consensus gate"),
        ("human", "Human authorization gate"),
    ];
    
    for (kw, desc) in keywords {
        if kw == word {
            return Some(format!("**{}** (keyword)\n\n{}", kw, desc));
        }
    }
    
    None
}

/// Get completion items
pub fn get_completions(_content: &str) -> Vec<CompletionItem> {
    let mut items = Vec::new();
    
    // Add keywords
    let keywords = vec![
        "agent", "recursive", "fn", "let", "if", "else", "loop",
        "return", "import", "export", "struct", "module", "govern",
        "scale", "cycle", "modify", "proof", "consensus", "human",
    ];
    for kw in keywords {
        items.push(CompletionItem {
            label: kw.to_string(),
            kind: Some(CompletionItemKind::KEYWORD),
            ..Default::default()
        });
    }
    
    // Add builtins
    for (name, desc) in get_all_builtins() {
        items.push(CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some("builtin".to_string()),
            documentation: Some(Documentation::String(desc)),
            ..Default::default()
        });
    }
    
    items
}

/// Go to definition
pub fn goto_definition(_content: &str, _line: u32, _character: u32) -> Option<Location> {
    None
}

/// Get document symbols (outline view)
#[allow(deprecated)]
pub fn get_document_symbols(content: &str) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();
    
    for (line_num, line) in content.lines().enumerate() {
        let line = line.trim();
        
        // Agent definitions
        if line.starts_with("agent ") || line.starts_with("recursive agent ") {
            if let Some(name) = extract_name_after_keyword(line, "agent") {
                symbols.push(DocumentSymbol {
                    name,
                    detail: Some("agent".to_string()),
                    kind: SymbolKind::CLASS,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    selection_range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    children: None,
                });
            }
        }
        
        // Function definitions
        if line.starts_with("fn ") || line.starts_with("export fn ") {
            if let Some(name) = extract_name_after_keyword(line, "fn") {
                symbols.push(DocumentSymbol {
                    name,
                    detail: Some("function".to_string()),
                    kind: SymbolKind::FUNCTION,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    selection_range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    children: None,
                });
            }
        }
        
        // Struct definitions
        if line.starts_with("struct ") {
            if let Some(name) = extract_name_after_keyword(line, "struct") {
                symbols.push(DocumentSymbol {
                    name,
                    detail: Some("struct".to_string()),
                    kind: SymbolKind::STRUCT,
                    tags: None,
                    deprecated: None,
                    range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    selection_range: Range {
                        start: Position { line: line_num as u32, character: 0 },
                        end: Position { line: line_num as u32, character: line.len() as u32 },
                    },
                    children: None,
                });
            }
        }
    }
    
    symbols
}

fn get_word_at_position(content: &str, line: u32, character: u32) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    if line as usize >= lines.len() {
        return None;
    }
    
    let line_content = lines[line as usize];
    let chars: Vec<char> = line_content.chars().collect();
    
    if character as usize >= chars.len() {
        return None;
    }
    
    let mut start = character as usize;
    let mut end = character as usize;
    
    while start > 0 && is_word_char(chars[start - 1]) {
        start -= 1;
    }
    
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }
    
    if start >= end {
        return None;
    }
    
    Some(chars[start..end].iter().collect())
}

fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == ':'
}

fn extract_name_after_keyword(line: &str, keyword: &str) -> Option<String> {
    let parts: Vec<&str> = line.split_whitespace().collect();
    for (i, part) in parts.iter().enumerate() {
        if *part == keyword && i + 1 < parts.len() {
            let name = parts[i + 1];
            return Some(name.trim_matches(|c: char| !c.is_alphanumeric() && c != '_').to_string());
        }
    }
    None
}

fn get_builtin_description(name: &str) -> Option<String> {
    let descriptions: std::collections::HashMap<&str, &str> = [
        ("print", "Print a value to stdout"),
        ("println", "Print a value to stdout with newline"),
        ("strlen", "Get the length of a string"),
        ("concat", "Concatenate two strings"),
        ("sqrt", "Calculate square root"),
        ("abs", "Calculate absolute value"),
        ("bond", "Call bonded LLM (Qwen3)"),
    ].iter().cloned().collect();
    
    descriptions.get(name).map(|&s| s.to_string())
}

fn get_all_builtins() -> Vec<(String, String)> {
    vec![
        ("print".to_string(), "Print a value to stdout".to_string()),
        ("println".to_string(), "Print with newline".to_string()),
        ("strlen".to_string(), "Get string length".to_string()),
        ("concat".to_string(), "Concatenate strings".to_string()),
        ("sqrt".to_string(), "Square root".to_string()),
        ("bond".to_string(), "Call bonded LLM".to_string()),
    ]
}
