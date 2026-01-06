import * as path from 'path';
import { workspace, ExtensionContext } from 'vscode';
import {
    LanguageClient,
    LanguageClientOptions,
    ServerOptions,
} from 'vscode-languageclient/node';

let client: LanguageClient;

export function activate(context: ExtensionContext) {
    // Get LSP binary path from config or use default
    const config = workspace.getConfiguration('hlx');
    let serverPath = config.get<string>('lsp.path');

    if (!serverPath) {
        // Use bundled LSP binary
        // Assuming the binary is in ../target/release/hlx_lsp relative to extension
        serverPath = path.join(context.extensionPath, '..', '..', 'target', 'release', 'hlx_lsp');
    }

    // Server options: run the hlx_lsp binary
    const serverOptions: ServerOptions = {
        command: serverPath,
        args: [],
    };

    // Client options: document selector and sync options
    const clientOptions: LanguageClientOptions = {
        documentSelector: [
            { scheme: 'file', language: 'hlx' }
        ],
        synchronize: {
            // Notify the server about file changes to .hlxa and .hlxc files
            fileEvents: workspace.createFileSystemWatcher('**/*.{hlxa,hlxc}')
        }
    };

    // Create and start the language client
    client = new LanguageClient(
        'hlx-lsp',
        'HLX Language Server',
        serverOptions,
        clientOptions
    );

    client.start();
}

export function deactivate(): Thenable<void> | undefined {
    if (!client) {
        return undefined;
    }
    return client.stop();
}
