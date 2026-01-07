use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use hlx_compiler::HlxaParser;

pub struct Backend {
    client: Client,
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
                    trigger_characters: Some(vec![ ".".to_string(), ":".to_string() ]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),
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
        self.validate_document(params.text_document.uri, params.text_document.text).await;
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        // Since we use Full sync, the last item has the full text
        if let Some(event) = params.content_changes.pop() {
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
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
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
}
