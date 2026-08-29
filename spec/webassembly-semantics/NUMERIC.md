# WebAssembly numeric lowering

The WebAssembly backend consumes only `VerifiedProgram` and emits a core WebAssembly module directly. It must not route through JavaScript output, native MIR, LLVM IR, or a frontend provider.

Initial mapping:

```text
I32Add(lhs, rhs) → i32.add
```

WebAssembly `i32.add` wraps modulo 2³², so its signed `i32` result matches the Zryna operation and the JavaScript `(lhs + rhs) | 0` mapping. Exported scalar values use an explicitly versioned ABI. Binary validation and runtime execution are required before an artifact is reported as complete.

Later integer and floating-point operations require the same width, signedness, overflow, conversion, comparison, trap, and serialization specification as every other backend. Host bindings may normalize representations for an API, but may not alter Zryna semantics.
