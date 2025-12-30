#!/bin/bash
# HLX "One-Liner" Bootstrap Script
# Usage: curl -sSL https://raw.githubusercontent.com/latentcollapse/hlx-compiler/main/install.sh | bash

set -e

echo -e "\033[1;36m"
echo "╔══════════════════════════════════════════════════════╗"
echo "║           HLX Compiler Installer                     ║"
echo "║        Deterministic Vulkan Compute                  ║"
echo "╚══════════════════════════════════════════════════════╝"
echo -e "\033[0m"

# 1. Check Dependencies
echo "🔍 Checking dependencies..."

MISSING_DEPS=0

check_cmd() {
    if ! command -v "$1" &> /dev/null;
    then
        echo -e "  ❌ \033[1;31m$1\033[0m not found."
        MISSING_DEPS=1
    else
        echo -e "  ✅ \033[1;32m$1\033[0m found."
    fi
}

check_cmd "cargo"
check_cmd "python3"
check_cmd "glslc"

if [ $MISSING_DEPS -eq 1 ]; then
    echo -e "\n\033[1;33m⚠️  Missing dependencies!\033[0m"
    echo "Please install the missing tools:"
    echo "  - Rust:   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    echo "  - Vulkan: Install Vulkan SDK (https://vulkan.lunarg.com/)"
    echo "            Ubuntu: sudo apt install vulkan-sdk"
    echo "            Arch:   sudo pacman -S vulkan-devel shaderc"
    echo "            macOS:  brew install vulkan-sdk"
    exit 1
fi

# 2. Clone/Update Repo (if not running from within it)
if [ ! -f "Cargo.toml" ] || ! grep -q "hlx_vulkan" "Cargo.toml"; then
    echo -e "\n📦 Cloning HLX Compiler..."
    git clone https://github.com/latentcollapse/hlx-compiler.git
    cd hlx-compiler
else
    echo -e "\n📂 Already in project directory."
fi

# 3. Compile Shaders
echo -e "\n🛠️  Compiling Shaders..."
python3 scripts/compile_shaders.py

# 4. Build
echo -e "\n🦀 Building with Cargo (Release)..."
cargo build --release --bin train_transformer_full

# 5. Run ?
echo -e "\n✨ Build Complete!"
read -p "Do you want to run the benchmark now? (y/N) " -n 1 -r
echo
if [[ $REPLY =~ ^[Yy]$ ]]; then
    # Ensure corpus
    if [ ! -f "corpus.jsonl" ] && [ -f "test_corpus.jsonl" ]; then
        cp test_corpus.jsonl corpus.jsonl
    fi
    ./target/release/train_transformer_full
else
    echo "Run manually with: ./target/release/train_transformer_full"
fi
