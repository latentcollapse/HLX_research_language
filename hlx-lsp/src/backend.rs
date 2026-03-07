use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::analysis;

pub struct HlxBackend {
    pub client: Client,
    pub documents: Arc<RwLock<HashMap<Url, String>>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for HlxBackend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Options(
                    TextDocumentSyncOptions {
                        open_close: Some(true),
                        change: Some(TextDocumentSyncKind::FULL),
                        will_save: None,
                        will_save_wait_until: None,
                        save: Some(SaveOptions::default().into()),
                    },
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "::".to_string()]),
                    completion_item: None,
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
            ..Default::default()
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "HLX LSP server initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let mut docs = self.documents.write().await;
        docs.insert(params.text_document.uri.clone(), params.text_document.text.clone());
        
        // Analyze and publish diagnostics
        let diagnostics = analysis::get_diagnostics(&params.text_document.text);
        self.client.publish_diagnostics(params.text_document.uri, diagnostics, None).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let mut docs = self.documents.write().await;
        if let Some(change) = params.content_changes.first() {
            docs.insert(params.text_document.uri.clone(), change.text.clone());
            
            // Re-analyze and publish diagnostics
            let diagnostics = analysis::get_diagnostics(&change.text);
            self.client.publish_diagnostics(params.text_document.uri, diagnostics, None).await;
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let mut docs = self.documents.write().await;
        docs.remove(&params.text_document.uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let docs = self.documents.read().await;
        if let Some(content) = docs.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;
            
            if let Some(hover_info) = analysis::get_hover_info(content, position.line, position.character) {
                return Ok(Some(Hover {
                    contents: HoverContents::Scalar(MarkedString::String(hover_info)),
                    range: None,
                }));
            }
        }
        Ok(None)
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let docs = self.documents.read().await;
        let uri = params.text_document_position.text_document.uri;
        if let Some(content) = docs.get(&uri) {
            let items = analysis::get_completions(content);
            return Ok(Some(CompletionResponse::Array(items)));
        }
        Ok(None)
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let docs = self.documents.read().await;
        if let Some(content) = docs.get(&params.text_document_position_params.text_document.uri) {
            let position = params.text_document_position_params.position;
            
            let uri = params.text_document_position_params.text_document.uri.clone();
            if let Some(location) = analysis::goto_definition(content, position.line, position.character, uri) {
                return Ok(Some(GotoDefinitionResponse::Scalar(location)));
            }
        }
        Ok(None)
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let docs = self.documents.read().await;
        if let Some(content) = docs.get(&params.text_document.uri) {
            let symbols = analysis::get_document_symbols(content);
            return Ok(Some(DocumentSymbolResponse::Nested(symbols)));
        }
        Ok(None)
    }
}
