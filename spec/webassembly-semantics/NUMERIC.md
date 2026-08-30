# WebAssembly numeric lowering

The implemented `wasm-web` core `I32V1` profile consumes only `VerifiedProgram` and emits a core
WebAssembly 1.0 module directly. It does not route through JavaScript output, native MIR, LLVM IR,
or a frontend provider.

Initial mapping:

```text
I32Add(lhs, rhs) → i32.add
```

The current binary profile contains only type, function, export, and code sections. Functions may
use `local.get`, `i32.const`, `i32.add`, and `end`, with no declared locals. Imports, tables,
memory, globals, start, elements, data, tags, custom sections, non-function exports, and all other
instructions are rejected. Every complete binary passes the pinned WebAssembly 1.0 validator and
this narrower audit before the backend seals it for create-only publication.

WebAssembly `i32.add` wraps modulo 2³², so its signed `i32` result matches the Zryna operation and
the JavaScript `(lhs + rhs) | 0` mapping. Exported scalar values use scalar ABI v1. Runtime
execution is mandatory conformance evidence, not a per-build publication phase. The current tests
use Node's standard WebAssembly API; this is not a browser test or a strict typed host wrapper.

Later integer and floating-point operations require the same width, signedness, overflow, conversion, comparison, trap, and serialization specification as every other backend. Host bindings may normalize representations for an API, but may not alter Zryna semantics.
