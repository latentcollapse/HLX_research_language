use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use hlx_compiler::{HlxaParser, parser::Parser};

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
}

impl Backend {
    pub fn new(client: Client) -> Self {
        Self { client }
    }

            async fn validate_document(&self, uri: Url, text: String) {

                let parser = HlxaParser::new();

                let diagnostics = match parser.parse(&text) {

                    Ok(_) => vec![],

                    Err(e) => {

                        let msg = match e {

                            hlx_core::HlxError::ParseError { message } => message,

                            _ => format!("{}", e),

                        };



                        // TODO: Extract actual position from error message
                        // For now, show error at line 0
                        let pos = Position { line: 0, character: 0 };



                        vec![Diagnostic {

                            range: Range {

                                start: pos,

                                end: Position { line: 0, character: 100 },

                            },

                            severity: Some(DiagnosticSeverity::ERROR),

                            message: msg,

                            ..Default::default()

                        }]

                    }

                };

        

                self.client.publish_diagnostics(uri, diagnostics, None).await;

            }

        

    

        fn get_position(&self, text: &str, offset: usize) -> Position {

            let mut line = 0;

            let mut character = 0;

    

            for (i, c) in text.char_indices() {

                if i >= offset {

                    break;

                }

                if c == '\n' {

                    line += 1;

                    character = 0;

                } else {

                    character += 1;

                }

            }

    

            Position { line, character }

        }

    }

    