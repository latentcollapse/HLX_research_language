use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use hlx_compiler::HlxaParser;
use dashmap::DashMap;

pub struct Backend {
    client: Client,
    document_map: DashMap<String, String>,
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
                    trigger_characters: Some(vec![".".to_string(), ":".to_string()]) ,
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

    async fn completion(&self, _params: CompletionParams) -> Result<Option<CompletionResponse>> {
        // Basic Static Completion
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
        ];

        let items: Vec<CompletionItem> = keywords.into_iter().map(|(label, kind, detail)| {
            CompletionItem {
                label: label.to_string(),
                kind: Some(kind),
                detail: Some(detail.to_string()),
                ..Default::default()
            }
        }).collect();

        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        
        let doc_ref = self.document_map.get(uri.as_str());
        if let Some(doc) = doc_ref {
            let word = self.get_word_at_position(&doc, position);
            if let Some(word) = word {
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
        Self {
            client,
            document_map: DashMap::new(),
        }
    }

    async fn validate_document(&self, uri: Url, text: String) {
        let parser = HlxaParser::new();
        let diagnostics = match parser.parse_diagnostics(&text) {
            Ok(_) => vec![], // No errors
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

        self.client.publish_diagnostics(uri, diagnostics, None).await;
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