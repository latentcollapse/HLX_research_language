use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use hlx_compiler::HlxaParser;
use dashmap::DashMap;
use std::sync::Arc;

mod contracts;
mod ai_diagnostics;
mod patterns;
mod confidence;

use contracts::{ContractCache, ContractCatalogue};
use ai_diagnostics::AIDiagnosticBuilder;
use patterns::PatternLibrary;
use confidence::ConfidenceAnalyzer;

pub struct Backend {
    client: Client,
    document_map: DashMap<String, String>,
    contracts: Option<Arc<ContractCatalogue>>,
    patterns: Arc<PatternLibrary>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), ":".to_string(), "@".to_string()]) ,
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "HLX Language Server initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.document_map.insert(params.text_document.uri.to_string(), params.text_document.text.clone());
        self.validate_document(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // Since we use Full sync, the last item has the full text
        if let Some(event) = params.content_changes.pop() {
            self.document_map.insert(params.text_document.uri.to_string(), event.text.clone());
            self.validate_document(params.text_document.uri, event.text).await;
        }
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let mut items = Vec::new();

        // Check if we're typing a contract (@ID)
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;

        let is_contract_context = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
            self.is_typing_contract(&doc_ref, position)
        } else {
            false
        };

        if is_contract_context {
            // Show contract completions with smart snippets and context filtering
            if let Some(catalogue) = &self.contracts {
                // Determine what context we're in
                let context = if let Some(doc_ref) = self.document_map.get(uri.as_str()) {
                    self.get_contract_context(&doc_ref, position)
                } else {
                    ContractContext::General
                };

                // Map context to filter string
                let filter_key = match context {
                    ContractContext::MathExpression => "math",
                    ContractContext::ValueExpression => "value",
                    ContractContext::ControlFlow => "control",
                    ContractContext::IOOperation => "io",
                    ContractContext::FieldValue => "field",
                    ContractContext::General => "general",
                };

                // Get filtered contract IDs based on context
                let filtered_ids = catalogue.filter_by_relevance(filter_key);

                for id in filtered_ids {
                    if let Some(spec) = catalogue.get_contract(&id) {
                        let label = format!("@{}", id);
                        let snippet = catalogue.generate_snippet(&id)
                            .unwrap_or_else(|| format!("@{} {{ }}", id));

                        items.push(CompletionItem {
                            label: label.clone(),
                            kind: Some(CompletionItemKind::STRUCT),
                            detail: Some(format!("{} - {}", spec.name, spec.tier)),
                            documentation: Some(Documentation::MarkupContent(MarkupContent {
                                kind: MarkupKind::Markdown,
                                value: spec.description.clone(),
                            })),
                            insert_text: Some(snippet),
                            insert_text_format: Some(InsertTextFormat::SNIPPET),
                            ..Default::default()
                        });
                    }
                }
            }
        }

        // Basic Static Completion (keywords, builtins)
        let keywords = vec![
            // Keywords
            ("fn", CompletionItemKind::KEYWORD, "Function definition"),
            ("let", CompletionItemKind::KEYWORD, "Variable declaration"),
            ("if", CompletionItemKind::KEYWORD, "Conditional"),
            ("else", CompletionItemKind::KEYWORD, "Alternative"),
            ("loop", CompletionItemKind::KEYWORD, "Loop construct"),
            ("return", CompletionItemKind::KEYWORD, "Return value"),
            ("program", CompletionItemKind::KEYWORD, "Program definition"),
            ("break", CompletionItemKind::KEYWORD, "Break loop"),
            ("continue", CompletionItemKind::KEYWORD, "Continue loop"),
            ("true", CompletionItemKind::KEYWORD, "Boolean true"),
            ("false", CompletionItemKind::KEYWORD, "Boolean false"),
            ("null", CompletionItemKind::KEYWORD, "Null value"),

            // Builtins & Constants
            ("DEFAULT_MAX_ITER", CompletionItemKind::CONSTANT, "Safety constant (1,000,000)"),
            ("print", CompletionItemKind::FUNCTION, "Print value(s)"),
            ("len", CompletionItemKind::FUNCTION, "Get length of array/string"),
            ("to_int", CompletionItemKind::FUNCTION, "Convert to integer"),
            ("slice", CompletionItemKind::FUNCTION, "Slice array"),
            ("append", CompletionItemKind::FUNCTION, "Append to array"),
            ("type", CompletionItemKind::FUNCTION, "Get value type"),
            ("read_file", CompletionItemKind::FUNCTION, "Read file content"),
            ("http_request", CompletionItemKind::FUNCTION, "Make HTTP request"),
            ("json_parse", CompletionItemKind::FUNCTION, "Parse JSON string"),
        ];

        for (label, kind, detail) in keywords {
            items.push(CompletionItem {
                label: label.to_string(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                ..Default::default()
            });
        }

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;

        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let word = self.get_word_at_position(&doc, position);
            if let Some(word) = word {
                // Check if it's a contract (@ID)
                if word.starts_with('@') {
                    let contract_id = word.trim_start_matches('@');
                    if let Some(catalogue) = &self.contracts {
                        if let Some(hover_doc) = catalogue.format_hover_doc(contract_id) {
                            return Ok(Some(Hover {
                                contents: HoverContents::Markup(MarkupContent {
                                    kind: MarkupKind::Markdown,
                                    value: hover_doc,
                                }),
                                range: None,
                            }));
                        }
                    }
                }

                // Known symbols
                let hover_text = match word.as_str() {
                    "fn" => "## Function Definition\n`fn name(args) { ... }`",
                    "let" => "## Variable Declaration\n`let name = value;`",
                    "if" => "## Conditional\n`if (cond) { ... } else { ... }`",
                    "loop" => "## Loop Construct\n`loop (condition, max_iter) { ... }`\n\n**Note:** Use `DEFAULT_MAX_ITER` for the bound.",
                    "DEFAULT_MAX_ITER" => "## Safety Constant\nValue: `1,000,000`\n\nUse this for all loops to ensure termination.",
                    "to_int" => "## Builtin: to_int\n`to_int(value) -> int`\n\nConverts a value to an integer (truncates floats).",
                    "len" => "## Builtin: len\n`len(container) -> int`\n\nReturns the length of a string, array, or object.",
                    "print" => "## Builtin: print\n`print(val)`\n\nPrints a value to stdout.",
                    "append" => "## Builtin: append\n`append(array, item) -> array`\n\nReturns a new array with the item appended.",
                    "slice" => "## Builtin: slice\n`slice(array, start, len) -> array`\n\nReturns a slice of the array.",
                    "type" => "## Builtin: type\n`type(val) -> string`\n\nReturns the type name of the value.",
                    "http_request" => "## Builtin: http_request\n`http_request(method, url, body, headers) -> Response`\n\nMakes an HTTP request. Returns object with status, body, headers.",
                    "json_parse" => "## Builtin: json_parse\n`json_parse(json_string) -> Value`\n\nParses a JSON string into an HLX value.",
                    _ => return Ok(None),
                };

                return Ok(Some(Hover {
                    contents: HoverContents::Markup(MarkupContent {
                        kind: MarkupKind::Markdown,
                        value: hover_text.to_string(),
                    }),
                    range: None,
                }));
            }
        }

        Ok(None)
    }
}

impl Backend {
    pub fn new(client: Client) -> Self {
        // Try to load contract catalogue
        let catalogue_path = std::env::var("HLX_CONTRACT_CATALOGUE")
            .unwrap_or_else(|_| "../CONTRACT_CATALOGUE.json".to_string());

        let contracts = match ContractCache::new(&catalogue_path) {
            Ok(cache) => {
                eprintln!("✓ Loaded contract catalogue from {}", catalogue_path);
                Some(cache.clone_arc())
            }
            Err(e) => {
                eprintln!("⚠ Failed to load contract catalogue: {}",

 e);
                eprintln!("  Contracts will not be available in autocomplete/hover");
                None
            }
        };

        // Load pattern library
        let patterns = Arc::new(PatternLibrary::new());
        eprintln!("✓ Loaded {} HLX patterns", patterns.patterns.len());

        Self {
            client,
            document_map: DashMap::new(),
            contracts,
            patterns,
        }
    }

    async fn validate_document(&self, uri: Url, text: String) {
        let parser = HlxaParser::new();
        let mut diagnostics = match parser.parse_diagnostics(&text) {
            Ok(_) => vec![], // No parser errors
            Err(errors) => {
                let mut diags = Vec::new();
                for (msg, offset) in errors {
                    let pos = self.offset_to_position(&text, offset);

                    // Highlight the word at that position, or just one char
                    let end_pos = Position {
                        line: pos.line,
                        character: pos.character + 1
                    };

                    diags.push(Diagnostic {
                        range: Range {
                            start: pos,
                            end: end_pos,
                        },
                        severity: Some(DiagnosticSeverity::ERROR),
                        code: None,
                        code_description: None,
                        source: Some("hlx".to_string()),
                        message: msg,
                        related_information: None,
                        tags: None,
                        data: None,
                    });
                }
                diags
            }
        };

        // Add contract signature validation
        if let Some(catalogue) = &self.contracts {
            let contract_diags = self.validate_contract_signatures(&text, catalogue);
            diagnostics.extend(contract_diags);
        }

        self.client.publish_diagnostics(uri, diagnostics, None).await;
    }

    /// Validate contract field signatures
    fn validate_contract_signatures(&self, text: &str, catalogue: &ContractCatalogue) -> Vec<Diagnostic> {
        let mut diagnostics = Vec::new();
        let ai_diag = AIDiagnosticBuilder::new(catalogue);

        // Regex to find contract invocations: @123 { field: value, ... }
        // This is a simple pattern matcher - not a full parser
        for (line_idx, line) in text.lines().enumerate() {
            let mut chars = line.chars().enumerate().peekable();

            while let Some((i, ch)) = chars.next() {
                if ch == '@' {
                    // Found @ symbol, try to parse contract ID
                    let mut id_str = String::new();

                    // Collect digits
                    while let Some((_, digit_ch)) = chars.peek() {
                        if digit_ch.is_numeric() {
                            id_str.push(*digit_ch);
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    if id_str.is_empty() {
                        continue;
                    }

                    // Skip whitespace
                    while let Some((_, ws_ch)) = chars.peek() {
                        if ws_ch.is_whitespace() {
                            chars.next();
                        } else {
                            break;
                        }
                    }

                    // Check for opening brace
                    if let Some((brace_pos, '{')) = chars.next() {
                        // Found contract invocation, validate fields
                        if let Some(spec) = catalogue.get_contract(&id_str) {
                            // Extract field names until closing brace
                            let remaining = &line[brace_pos + 1..];
                            if let Some(close_brace) = remaining.find('}') {
                                let fields_section = &remaining[..close_brace];

                                // Parse field names (simple split by comma)
                                for field_pair in fields_section.split(',') {
                                    let field_pair = field_pair.trim();
                                    if field_pair.is_empty() {
                                        continue;
                                    }

                                    // Extract field name (before ':')
                                    if let Some(colon_pos) = field_pair.find(':') {
                                        let field_name = field_pair[..colon_pos].trim();

                                        // Check if this field exists in contract spec
                                        if !spec.fields.contains_key(field_name) {
                                            // Calculate position of field name
                                            let field_offset = brace_pos + 1 +
                                                fields_section[..fields_section.find(field_name).unwrap_or(0)].len();

                                            let pos = Position {
                                                line: line_idx as u32,
                                                character: field_offset as u32,
                                            };

                                            let end_pos = Position {
                                                line: line_idx as u32,
                                                character: (field_offset + field_name.len()) as u32,
                                            };

                                            // Use AI-optimized diagnostic
                                            let valid_fields: Vec<String> = spec.fields.keys().cloned().collect();
                                            let ai_diagnostic = ai_diag.unknown_field(
                                                Range { start: pos, end: end_pos },
                                                field_name,
                                                &id_str,
                                                &valid_fields
                                            );
                                            diagnostics.push(ai_diagnostic.to_diagnostic());
                                        }
                                    }
                                }

                                // Check for missing required fields
                                let provided_fields: std::collections::HashSet<String> =
                                    fields_section.split(',')
                                        .filter_map(|pair| {
                                            pair.trim().split(':').next().map(|s| s.trim().to_string())
                                        })
                                        .collect();

                                for (field_name, field_spec) in &spec.fields {
                                    if field_spec.required && !provided_fields.contains(field_name) {
                                        let pos = Position {
                                            line: line_idx as u32,
                                            character: i as u32,
                                        };

                                        let end_pos = Position {
                                            line: line_idx as u32,
                                            character: (brace_pos + close_brace + 2) as u32,
                                        };

                                        // Use AI-optimized diagnostic
                                        let ai_diagnostic = ai_diag.missing_required_field(
                                            Range { start: pos, end: end_pos },
                                            field_name,
                                            &id_str
                                        );
                                        diagnostics.push(ai_diagnostic.to_diagnostic());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        diagnostics
    }

    fn offset_to_position(&self, text: &str, offset: usize) -> Position {
        let mut line = 0;
        let mut last_line_start = 0;
        
        for (i, c) in text.char_indices() {
            if i >= offset {
                break;
            }
            if c == '\n' {
                line += 1;
                last_line_start = i + 1;
            }
        }
        
        let character = if offset >= last_line_start {
            (offset - last_line_start) as u32
        } else {
            0
        };

        Position { line, character }
    }

    fn get_word_at_position(&self, text: &str, pos: Position) -> Option<String> {
        let line = text.lines().nth(pos.line as usize)?;
        let char_idx = pos.character as usize;

        if char_idx >= line.len() {
            return None;
        }

        // Check if we're on an @ symbol (contract start)
        if char_idx > 0 && line.chars().nth(char_idx - 1) == Some('@') {
            // Look ahead for contract ID
            let start = char_idx;
            let end = line[char_idx..]
                .find(|c: char| !c.is_numeric())
                .map(|i| char_idx + i)
                .unwrap_or(line.len());

            if start < end {
                return Some(format!("@{}", &line[start..end]));
            }
        }

        // Check if we're inside a contract ID (@123)
        if line.chars().nth(char_idx) == Some('@') ||
           (char_idx > 0 && line[..char_idx].chars().rev().take_while(|c| c.is_numeric()).count() > 0) {
            // Find the @ symbol
            let at_pos = line[..=char_idx]
                .rfind('@')
                .unwrap_or(char_idx);

            // Find end of contract ID
            let end = line[at_pos+1..]
                .find(|c: char| !c.is_numeric())
                .map(|i| at_pos + 1 + i)
                .unwrap_or(line.len());

            if at_pos + 1 < end {
                return Some(format!("@{}", &line[at_pos+1..end]));
            }
        }

        // Find start of word (regular identifiers)
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

    fn is_typing_contract(&self, text: &str, pos: Position) -> bool {
        if let Some(line) = text.lines().nth(pos.line as usize) {
            let char_idx = pos.character as usize;
            if char_idx > 0 && char_idx <= line.len() {
                // Check if previous character is '@'
                if line.chars().nth(char_idx - 1) == Some('@') {
                    return true;
                }
                // Check if we're typing digits after '@'
                if let Some(at_pos) = line[..char_idx].rfind('@') {
                    let between = &line[at_pos+1..char_idx];
                    if between.chars().all(|c| c.is_numeric()) {
                        return true;
                    }
                }
            }
        }
        false
    }

    /// Determine contract context for smart filtering
    fn get_contract_context(&self, text: &str, pos: Position) -> ContractContext {
        let line_idx = pos.line as usize;

        // Look at current line and a few lines before for context
        let context_lines: Vec<&str> = text.lines()
            .skip(line_idx.saturating_sub(3))
            .take(4)
            .collect();

        let current_line = context_lines.last().unwrap_or(&"");

        // Check for specific contexts
        if current_line.contains("let ") && current_line.contains("=") {
            // Variable assignment - values and operations
            return ContractContext::ValueExpression;
        }

        if current_line.contains("if ") || current_line.contains("loop ") {
            // Control flow context
            return ContractContext::ControlFlow;
        }

        // Check for mathematical operators nearby
        if current_line.contains("+") || current_line.contains("-") ||
           current_line.contains("*") || current_line.contains("/") {
            return ContractContext::MathExpression;
        }

        // Check if inside function parameters or object fields
        if let Some(last_open) = current_line.rfind('{') {
            if let Some(last_close) = current_line.rfind('}') {
                if last_open > last_close {
                    // Inside braces, likely a field value
                    return ContractContext::FieldValue;
                }
            } else {
                // Open brace but no close - definitely inside
                return ContractContext::FieldValue;
            }
        }

        // Check for print/io context
        if current_line.contains("print") || context_lines.iter().any(|l| l.contains("http_request")) {
            return ContractContext::IOOperation;
        }

        // Default: show all user-facing contracts
        ContractContext::General
    }
}

/// Context types for smart contract filtering
#[derive(Debug, Clone, PartialEq)]
enum ContractContext {
    General,           // Show all user-facing contracts
    ValueExpression,   // Variable assignments, return values
    MathExpression,    // Math operations
    ControlFlow,       // if/loop/pattern matching
    FieldValue,        // Inside { } braces
    IOOperation,       // I/O and external calls
}