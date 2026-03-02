use tower_lsp::{LspService, Server};
use std::sync::Arc;
use tokio::sync::RwLock;

mod backend;
mod analysis;

use backend::HlxBackend;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| HlxBackend {
        client,
        documents: Arc::new(RwLock::new(std::collections::HashMap::new())),
    });

    Server::new(stdin, stdout, socket).serve(service).await;
}
