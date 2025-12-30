#!/usr/bin/env python3
"""
HLX Reproducibility Runner
Cross-platform script to build, compile shaders, and run benchmarks.
"""

import os
import subprocess
import sys
import platform
import time
from pathlib import Path

def run_step(description, cmd, cwd=None, env=None):
    print(f"\n👉 {description}...")
    try:
        start = time.time()
        # On Windows, shell=True might be needed for some commands, but usually not for subprocess.run list
        subprocess.run(cmd, check=True, cwd=cwd, env=env)
        print(f"   ✅ Done in {time.time() - start:.1f}s")
    except subprocess.CalledProcessError:
        print(f"   ❌ Failed!")
        sys.exit(1)
    except FileNotFoundError:
        print(f"   ❌ Command not found: {cmd[0]}")
        sys.exit(1)

def main():
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    
    print("╔══════════════════════════════════════════════════════╗")
    print("║     HLX Reproduction Suite                           ║")
    print("║     Cross-Platform (Windows/Linux/macOS)             ║")
    print("╚══════════════════════════════════════════════════════╝")
    print(f"OS: {platform.system()} {platform.release()}")
    print(f"Root: {project_root}")

    # 1. Compile Shaders
    run_step(
        "Compiling Shaders",
        [sys.executable, str(script_dir / "compile_shaders.py")]
    )

    # 2. Build Rust Project
    cargo_cmd = ["cargo", "build", "--release", "--bin", "train_transformer_full"]
    run_step(
        "Building Compiler (Rust)",
        cargo_cmd,
        cwd=project_root
    )

    # 3. Check for corpus
    corpus_path = project_root / "corpus.jsonl"
    if not corpus_path.exists():
        print("\n⚠️  corpus.jsonl not found!")
        # Try to use test_corpus.jsonl if available
        test_corpus = project_root / "test_corpus.jsonl"
        if test_corpus.exists():
            print("   Using test_corpus.jsonl instead.")
            import shutil
            shutil.copy(test_corpus, corpus_path)
        else:
            print("   Please ensure 'corpus.jsonl' is in the project root.")
            sys.exit(1)

    # 4. Run Benchmark
    binary_name = "train_transformer_full"
    if platform.system() == "Windows":
        binary_name += ".exe"
    
    binary_path = project_root / "target" / "release" / binary_name
    
    if not binary_path.exists():
        print(f"\n❌ Binary not found at: {binary_path}")
        sys.exit(1)

    print("\n🚀 Running Training Benchmark...")
    print("   (This may take a few seconds/minutes)")
    
    # Run the binary
    # We pipe output to stdout so the user sees progress
    try:
        subprocess.run([str(binary_path)], cwd=project_root, check=True)
    except subprocess.CalledProcessError:
        print("\n❌ Benchmark run failed/crashed.")
        print("   If on AMD/Intel, check for driver compatibility.")
        sys.exit(1)

    print("\n✨ Reproduction Complete!")
    print("   Check 'checkpoints/training_curve.csv' for loss values.")
    print("   Target loss to beat (CUDA): 0.5128")
    print("   HLX Reference (NVIDIA): 0.4783")

if __name__ == "__main__":
    main()
