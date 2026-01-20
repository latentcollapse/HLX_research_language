#!/bin/bash
# Merge all bootstrap components into a single monolithic compiler

set -e

echo "Merging bootstrap compiler components..."

OUTPUT="compiler.hlx"

# Start with header
cat > "$OUTPUT" << 'EOF'
// ═══════════════════════════════════════════════════════════════════════════════
// HLX BOOTSTRAP COMPILER - MONOLITHIC
// Complete self-compiling compiler: Lexer + Parser + Lowerer + Emitter
// ═══════════════════════════════════════════════════════════════════════════════

program bootstrap_compiler {

EOF

# Extract lexer (everything except program wrapper and main())
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "// PHASE 1: LEXER - Source Code → Tokens" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "" >> "$OUTPUT"

# Get lexer content (skip first 6 lines "program hlx_lexer {", last line "}", and main())
sed -n '7,436p' lexer.hlx >> "$OUTPUT"

# Extract parser (everything except program wrapper and main())
echo "" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "// PHASE 2: PARSER - Tokens → AST" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "" >> "$OUTPUT"

# Get parser content (skip first line "program parser {", last lines with main())
sed -n '4,816p' parser.hlx >> "$OUTPUT"

# Extract lowerer (everything except program wrapper and main())
echo "" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "// PHASE 3: LOWERER - AST → Bytecode Instructions" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "" >> "$OUTPUT"

# Get lowerer content
sed -n '4,811p' lower.hlx >> "$OUTPUT"

# Extract emitter (everything except program wrapper and main())
echo "" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "// PHASE 4: EMITTER - Bytecode Instructions → Binary" >> "$OUTPUT"
echo "// ═══════════════════════════════════════════════════════════════════════════════" >> "$OUTPUT"
echo "" >> "$OUTPUT"

# Get emitter content
sed -n '4,327p' emit.hlx >> "$OUTPUT"

# Add pipeline function
cat >> "$OUTPUT" << 'EOF'

// ═══════════════════════════════════════════════════════════════════════════════
// FULL COMPILATION PIPELINE
// ═══════════════════════════════════════════════════════════════════════════════

fn compile(source: String) -> [i64] {
    print("=== HLX Bootstrap Compiler ===\n");

    // Phase 1: Lexical Analysis
    print("[1/4] Lexing...\n");
    let tokens = tokenize(source);
    let token_count = array_len(tokens);
    print("  Tokens: ");
    print(token_count);
    print("\n");

    // Phase 2: Parsing
    print("[2/4] Parsing...\n");
    let ast = parse_program(tokens);
    print("  AST constructed\n");

    // Phase 3: Lowering
    print("[3/4] Lowering...\n");
    let instructions = lower_program(ast);
    let inst_count = array_len(instructions);
    print("  Instructions: ");
    print(inst_count);
    print("\n");

    // Phase 4: Emission
    print("[4/4] Emitting...\n");
    let binary = emit_bytecode(instructions);
    let binary_size = array_len(binary);
    print("  Binary size: ");
    print(binary_size);
    print(" bytes\n");

    print("=== Compilation Complete ===\n");

    return binary;
}

// ═══════════════════════════════════════════════════════════════════════════════
// MAIN: Test the Full Compiler
// ═══════════════════════════════════════════════════════════════════════════════

fn main() -> i64 {
    print("═══════════════════════════════════════════════════════════════\n");
    print("HLX Bootstrap Compiler - Monolithic Self-Hosting Version\n");
    print("═══════════════════════════════════════════════════════════════\n\n");

    // Test program: Simple arithmetic
    let source = "program test { fn add(a: i64, b: i64) -> i64 { return a + b; } fn main() -> i64 { let x = add(10, 32); return x; } }";

    print("Source:\n");
    print(source);
    print("\n\n");

    // Compile it
    let binary = compile(source);

    print("\n✓ Compilation successful!\n");
    print("Binary output ready (");
    print(array_len(binary));
    print(" bytes)\n\n");

    print("Next step: Self-compilation test\n");
    print("(Compile this compiler with itself!)\n");

    return 0;
}

}
EOF

echo "✓ Merged into $OUTPUT"
wc -l "$OUTPUT"
