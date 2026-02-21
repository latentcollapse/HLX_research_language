#!/bin/bash
# Build script for compiling GLSL compute shaders to SPIR-V

set -e

SHADER_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

echo "🔨 Building Vulkan compute shaders..."

# Check for glslangValidator
if ! command -v glslangValidator &> /dev/null; then
    echo "❌ glslangValidator not found!"
    echo "Install with: sudo apt-get install glslang-tools (Ubuntu/Debian)"
    echo "           or: brew install glslang (macOS)"
    exit 1
fi

# Compile each compute shader
COMPUTE_SHADERS=(
    "pointwise_add"
    "gemm"
    "activation"
    "softmax"
    "layernorm"
    "cross_entropy"
    "elementwise"
    "reduction"
    "conv2d"
    "pooling"
    "batchnorm"
    "dropout"
    "transpose"
    "gaussian_blur"
    "sobel"
)

# Graphics shaders (vertex/fragment)
VERTEX_SHADERS=(
    "basic"
)

FRAGMENT_SHADERS=(
    "basic"
    "pbr"
)

SUCCESS_COUNT=0
FAIL_COUNT=0

# Compile compute shaders
echo "🔹 Compute Shaders:"
for shader in "${COMPUTE_SHADERS[@]}"; do
    INPUT="${SHADER_DIR}/${shader}.comp"
    OUTPUT="${SHADER_DIR}/${shader}.spv"

    if [ ! -f "$INPUT" ]; then
        echo "⚠️  Skipping $shader.comp (source not found)"
        continue
    fi

    echo -n "  $shader.comp... "

    if glslangValidator -V "$INPUT" -o "$OUTPUT" 2>&1 | grep -q "ERROR"; then
        echo "❌ FAILED"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        glslangValidator -V "$INPUT" -o "$OUTPUT"
    else
        SIZE=$(stat -f%z "$OUTPUT" 2>/dev/null || stat -c%s "$OUTPUT" 2>/dev/null)
        echo "✅ ($SIZE bytes)"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done

# Compile vertex shaders
echo ""
echo "🔹 Vertex Shaders:"
for shader in "${VERTEX_SHADERS[@]}"; do
    INPUT="${SHADER_DIR}/${shader}.vert"
    OUTPUT="${SHADER_DIR}/${shader}_vert.spv"

    if [ ! -f "$INPUT" ]; then
        echo "⚠️  Skipping $shader.vert (source not found)"
        continue
    fi

    echo -n "  $shader.vert... "

    if glslangValidator -V "$INPUT" -o "$OUTPUT" 2>&1 | grep -q "ERROR"; then
        echo "❌ FAILED"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        glslangValidator -V "$INPUT" -o "$OUTPUT"
    else
        SIZE=$(stat -f%z "$OUTPUT" 2>/dev/null || stat -c%s "$OUTPUT" 2>/dev/null)
        echo "✅ ($SIZE bytes)"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done

# Compile fragment shaders
echo ""
echo "🔹 Fragment Shaders:"
for shader in "${FRAGMENT_SHADERS[@]}"; do
    INPUT="${SHADER_DIR}/${shader}.frag"
    OUTPUT="${SHADER_DIR}/${shader}_frag.spv"

    if [ ! -f "$INPUT" ]; then
        echo "⚠️  Skipping $shader.frag (source not found)"
        continue
    fi

    echo -n "  $shader.frag... "

    if glslangValidator -V "$INPUT" -o "$OUTPUT" 2>&1 | grep -q "ERROR"; then
        echo "❌ FAILED"
        FAIL_COUNT=$((FAIL_COUNT + 1))
        glslangValidator -V "$INPUT" -o "$OUTPUT"
    else
        SIZE=$(stat -f%z "$OUTPUT" 2>/dev/null || stat -c%s "$OUTPUT" 2>/dev/null)
        echo "✅ ($SIZE bytes)"
        SUCCESS_COUNT=$((SUCCESS_COUNT + 1))
    fi
done

echo ""
echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
echo "📊 Results: $SUCCESS_COUNT succeeded, $FAIL_COUNT failed"

if [ $FAIL_COUNT -eq 0 ]; then
    echo "✅ All shaders compiled successfully!"
    exit 0
else
    echo "❌ Some shaders failed to compile"
    exit 1
fi
