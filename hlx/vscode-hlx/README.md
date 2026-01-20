# HLX Language Support for VS Code

Language support for HLX with syntax highlighting and Language Server Protocol integration.

## Features

- **Syntax Highlighting**: Keywords, types, functions, strings, numbers, and comments
- **Real-time Syntax Checking**: Immediate feedback on syntax errors via LSP
- **Bracket Matching**: Auto-closing and matching for `{}`, `[]`, `()`
- **Comment Support**: Line comments with `//`

## Installation

### From Source

1. Build the HLX LSP server:
   ```bash
   cd /home/matt/hlx-compiler/hlx
   cargo build --release --bin hlx_lsp
   ```

2. Install the extension:
   ```bash
   cd vscode-hlx
   bun install
   bun run compile
   ```

3. Copy the extension to VS Code extensions directory:
   ```bash
   cp -r . ~/.vscode/extensions/hlx-language-0.1.0/
   ```

   Or install via VS Code:
   - Press `F1` → `Developer: Install Extension from Location`
   - Select the `vscode-hlx` directory

## Configuration

The extension looks for the `hlx_lsp` binary at `../target/release/hlx_lsp` relative to the extension. You can override this in VS Code settings:

```json
{
  "hlx.lsp.path": "/path/to/hlx_lsp"
}
```

## Supported File Extensions

- `.hlx` - HLX ASCII source files
- `.hlxc` - HLX compiled/canonical files

## Language Features

### Keywords
`fn`, `let`, `program`, `if`, `else`, `loop`, `break`, `continue`, `return`, `and`, `or`, `not`

### Types
`bool`, `int`, `float`, `string`, `object`

### Built-in Functions
`print`, `len`, `ord`, `chr`, `push`, `pop`, `type`

## Development

To watch for changes during development:

```bash
bun run watch
```

## Requirements

- VS Code 1.75.0 or higher
- HLX compiler with LSP server (`hlx_lsp`)

## License

Same as HLX compiler (see parent project)
