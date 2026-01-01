#!/usr/bin/env python3
"""
Cross-platform Shader Compiler for HLX
Compiles GLSL shaders to SPIR-V using 'glslc'.
"""

import os
import subprocess
import sys
from pathlib import Path

def main():
    # Detect glslc
    try:
        subprocess.run(["glslc", "--version"], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    except FileNotFoundError:
        print("❌ Error: 'glslc' not found in PATH.")
        print("  - Windows: Install the Vulkan SDK (https://vulkan.lunarg.com/)")
        print("  - Linux: Install 'glslc' or 'shaderc' package")
        print("  - macOS: brew install glslc")
        sys.exit(1)

    # Paths
    script_dir = Path(__file__).parent.resolve()
    project_root = script_dir.parent
    shader_dir = project_root / "shader"
    output_dir = shader_dir / "spv"

    if not shader_dir.exists():
        print(f"❌ Error: Shader directory not found: {shader_dir}")
        sys.exit(1)

    output_dir.mkdir(exist_ok=True)

    # Compile
    shaders = list(shader_dir.glob("*.glsl")) + list(shader_dir.glob("*.comp")) + \
              list(shader_dir.glob("*.vert")) + list(shader_dir.glob("*.frag"))
    
    if not shaders:
        print("⚠️  No shaders found to compile.")
        return

    print(f"🛠️  Compiling {len(shaders)} shaders to {output_dir}...")
    
    success_count = 0
    fail_count = 0

    for shader in shaders:
        out_file = output_dir / (shader.name + ".spv")
        cmd = [
            "glslc",
            str(shader),
            "-o", str(out_file),
            "--target-env=vulkan1.2"  # Ensure compatibility
        ]
        
        try:
            subprocess.run(cmd, check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
            print(f"  ✅ {shader.name}")
            success_count += 1
        except subprocess.CalledProcessError as e:
            print(f"  ❌ {shader.name}")
            print(e.stderr.decode().strip())
            fail_count += 1

    print("-" * 40)
    if fail_count == 0:
        print(f"✨ Success! All {success_count} shaders compiled.")
    else:
        print(f"⚠️  Completed with errors: {success_count} succeeded, {fail_count} failed.")
        sys.exit(1)

if __name__ == "__main__":
    main()
