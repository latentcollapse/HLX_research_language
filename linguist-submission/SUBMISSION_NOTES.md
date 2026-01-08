# HLX Linguist Submission

## Language Information

**Name**: HLX (Helix Language)
**Type**: Programming Language
**Status**: Self-hosting compiler implemented
**Repository**: https://github.com/latentcollapse/hlx-compiler

## File Extensions

- `.hlxa` - HLX ASCII source files
- `.hlxc` - HLX compiled/canonical files

## Key Features

- Self-hosting: Compiler written in HLX, compiles itself
- Deterministic: Same input always produces identical output
- Tensor-native: First-class tensor operations
- Human-AI collaboration focused

## Files to Submit

1. **languages.yml entry** - Language definition
2. **hlx.tmLanguage.json** - TextMate grammar for syntax highlighting

## Sample Code

```hlx
program fibonacci {
    fn fib(n) -> int {
        if n <= 1 {
            return n;
        }
        return fib(n - 1) + fib(n - 2);
    }

    fn main() {
        let result = fib(10);
        print(result);
        return 0;
    }
}
```

## Verification

The language has a working compiler with bootstrap verification:
- Stage 1: Rust compiler compiles HLX source
- Stage 2: HLX compiler (compiled by Rust) compiles itself
- Stage 3: HLX compiler (compiled by Stage 2) compiles itself
- Verification: Stage 2 == Stage 3 (bytewise identical)

## References

- Compiler: https://github.com/latentcollapse/hlx-compiler
- VS Code Extension: Included in repository
- License: Apache 2.0
