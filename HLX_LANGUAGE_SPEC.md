# HLX Language Specification (v0.1)

HLX (Heuristic Logic eXchange) is a neurosymbolic language designed for intent-driven agents. It prioritizes deterministic execution, latent space transparency, and direct LLM bonding.

## 1. Module System

HLX uses a hierarchical module system. Files define modules implicitly or explicitly.

- **Declaration**: `module Name { ... }`
- **Imports**: 
  - `import { foo, bar } from "./path";` (Relative import)
  - `use hil::infer;` (Canonical stdlib import using `::` namespacing)
- **Qualified Access**: `infer::reason(...)`

## 2. Type System

HLX is statically typed with local inference.

- **Primitives**: 
  - `i64`: 64-bit integer
  - `f64`: 64-bit float
  - `bool`: boolean (`true`, `false`)
  - `String`: UTF-8 string
  - `void`: Empty return type
- **Collections**:
  - `Array<T>`: Dynamic growable array.
  - `Map<K, V>`: Key-value dictionary.
- **Complex**:
  - `Option<T>`: Optional value (`some(x)` or `nil`).
  - `struct`: User-defined data structures.

## 3. Functions

Functions are the primary unit of execution.

- **Syntax**: `fn name(param: Type) -> ReturnType { ... }`
- **Exporting**: `export fn ...` makes the function visible to other modules.
- **Higher-Order**: Supports lambdas `|params| body` and passing functions as values.

## 4. Variables

- **Immutable**: `let x: Type = value;` (Can be shadowed but not reassigned)
- **Mutable**: HLX prioritizes functional updates, but `let mut` is supported in some variants.

## 5. Control Flow

- **Branching**: `if condition { ... } else { ... }`
- **Bounded Loops**: `loop(condition, max_iters) { ... }` (Safety-first iteration)
- **Collection Loops**: `for x in collection { ... }` (Sugars to index iteration)

## 6. Conscience & Governance (APE)

HLX integrates directly with the Axiom Policy Engine (APE).

- **Intent**: `intent Name { takes: [...], gives: [...] }`
- **Clauses**: 
  - `takes`: Input dependencies.
  - `gives`: Output guarantees.
  - `pre/post`: Constraints checked by the Conscience.
- **Governance**: Every `do intent` is audited by the Conscience before refraction.

## 7. Neurosymbolic Bonding

- **Bond Builtin**: `bond("prompt")` or `infer::reason("query", context)`
- **Behavior**: Suspends VM execution to call a registered LLM (e.g., Qwen3) via the Bond Protocol (HELLO/SYNC/READY).

## 8. Builtins

- **Math**: `abs(x)`, `sqrt(x)`, `min(a,b)`, `max(a,b)`, `clamp(x,lo,hi)`, `pow(b,e)`, `sin(x)`, `cos(x)`, `floor(x)`, `ceil(x)`
- **IO**: `print(x)`, `println(x)`
- **String/Array**: `len(coll)`, `concat(a,b)`, `push(arr, val)`, `pop(arr)`
- **System**: `clock_ms()`, `exec_shell("cmd")`

## 9. Example

```hlx
use hil::infer;

export fn calculate_density(mass: f64, volume: f64) -> f64 {
    if volume <= 0.0 { return 0.0; }
    return mass / volume;
}

fn main() {
    let items = [1.0, 2.5, 3.0];
    let mut sum = 0.0;
    
    for val in items {
        sum += val;
    }
    
    let answer = infer::reason("What is the sum?", {"sum": sum});
    println(answer);
}
```
