# HLX Changelog

## [0.1.0-beta] — 2026-03-02

HLX reaches MVP: a functional, usable programming language with neurosymbolic runtime.

### Language Features
- **Lambdas/closures**: `|x| x * 2`, `|a, b| a + b`, `|| expr` — first-class functions via `Value::Function` + `CallDyn` opcode
- **Compound assignment**: `+=`, `-=`, `*=`, `/=`, `%=` — properly tokenized and parsed
- **Ternary operator**: `x > 3 ? "yes" : "no"`
- **For-in loops**: `for item in array { ... }`
- **Break/continue**: loop control flow
- **Method call syntax**: `arr.len()` transforms to `len(arr)`
- **Array literals**: `[1, 2, 3]` — fixed lowering (Push opcode byte alignment)
- **Array indexing**: `arr[i]` via Index expression + Get opcode
- **Switch/case**: pattern matching on values
- **Short-circuit evaluation**: `&&` and `||`

### Standard Library
- **Math**: `sin`, `cos`, `tan`
- **Arrays**: `push`, `sort`, `len`, `get_at`, `set_at`
- **Strings**: `concat`, `strcmp`, `ord`, `char`, `str_char_at`, `substring`, `str_len`, `to_string`
- **I/O**: `file_read`, `file_write`, `println`, `print`
- **System**: `shell`, `sleep`, `clock_ms` / `current_time`
- **Debug**: `assert`, `type_of`

### Runtime
- **Stack traces**: function call chain in error messages
- **Debug mode**: `--debug` / `-d` flag traces every opcode with register states
- **Memory limits**: `--max-array-size`, `--max-string-size`
- **Timeout**: `--timeout-ms` wall-clock execution limit
- **Step limit**: configurable max instruction count

### Neurosymbolic Core (hlx-runtime)
- 72 security tests passing
- RSI (Recursive Self-Improvement) pipeline with gate control
- Homeostasis system with non-Newtonian resistance
- Promotion levels: Seedling → Sapling → Sprout → Mature
- Memory pool: observations, patterns, questions, history
- Bond system: symbiote state machine for LLM bonding
- LoRA adapter management with integrity verification
- Forgetting guard with retention testing
- Training gate with gradient explosion/NaN detection
- Governance engine with rate limiting and severity caps
- Shader attestation for compute verification

### APE (Axiom Policy Engine)
- G1-G6 conscience predicates (Rocq/Coq formal proofs)
- Trust algebra and authorization gates
- Integrity system with provenance chains
- BitSeed Python class with belief system (Phase 19)

### Developer Experience
- Source line mapping in error messages
- Bytecode serialization with integrity hashing
- Module resolver for imports
- AST visitor pattern and renderer
- LSP server (hlx-lsp)

### Bug Fixes
- **Critical**: Compound assignment (`+=` etc.) was silently miscompiled as no-op — lexer had no compound tokens, `parse_primary` default returned nil instead of erroring
- **Critical**: For-loop register clobbering — loop state in temp registers got overwritten by body expressions; fixed with reserved registers 240-243
- **Array literal lowering**: Push opcode emitted wrong byte count; fixed with Const(empty) + Push(dst, elem) pattern
- **`__top_level__` merge**: consolidated module-level let statements into single function
- **hlx-bond**: proper Qwen3 no-think mode via empty `<think></think>` block

---

## [0.0.1-alpha] — 2026-02-28

Initial experimental branch. Core bytecode VM, AST parser, lowerer, basic opcodes.
