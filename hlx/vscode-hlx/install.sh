#!/bin/bash
# Install HLX VS Code Extension

set -e

EXTENSION_DIR="$HOME/.vscode/extensions/hlx-language-0.1.0"

echo "Installing HLX Language Extension..."

# Create extension directory
mkdir -p "$EXTENSION_DIR"

# Copy extension files
cp -r package.json language-configuration.json README.md "$EXTENSION_DIR/"
cp -r out "$EXTENSION_DIR/"
cp -r syntaxes "$EXTENSION_DIR/"

echo "✓ Extension installed to: $EXTENSION_DIR"
echo ""
echo "Next steps:"
echo "1. Restart VS Code (or run 'Developer: Reload Window')"
echo "2. Open a .hlxa file"
echo "3. The LSP should activate automatically"
echo ""
echo "To test: open test_lsp.hlxa in VS Code and check for syntax errors"
