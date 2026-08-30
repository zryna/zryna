# Zryna compiler driver

The only compiler component allowed to orchestrate frontend, verification, and backend phases.

`analyze_sources` accepts one authoritative `SourceMap` and a configured process frontend. It
returns only the opaque protocol-v2 snapshot that the frontend crate has authenticated, decoded,
bounded, and verified against that exact map. Raw worker bytes and untrusted syntax DTOs are not a
driver-facing contract.

`lower_verified_syntax` applies the provider-error stop gate, constructs the sealed semantic
input, performs Zryna-owned name resolution and strict lowering, and runs the mandatory Universal
IR verifier. `compile_to_verified_ir` owns the complete authenticated source-to-verified-IR path.
It returns `SourceToIrSuccess` only when backends may safely consume the program. Non-fatal provider
warnings remain observable on that success value; frontend failures and compiler rejections remain
distinct error categories.

The current success profile is the one-file, explicitly typed `i32` subset documented by
`zryna-semantics`. Source-level `bool` is checked and lowered but remains rejected by the current
`I32V1` IR profile until every active backend supports it. Multiple files remain disabled until
module resolution is specified.

`emit_verified` still proves only the existing direct JavaScript and textual native-backend
boundaries. Direct WebAssembly emission, object generation, linking, target execution, and a
user-facing build/run CLI remain later M1 gates.
