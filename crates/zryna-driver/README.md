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

`compile_javascript` connects real source to the deterministic JavaScript backend and publishes one
new `.mjs` artifact through the target-neutral `ArtifactOutputRoot` capability (with a compatible
`JavaScriptOutputRoot` alias). The capability is derived only from an absolute
workspace path's exact `.zryna/out` location and rejects missing, non-directory, symbolic-link, and
Windows reparse-point components throughout the persistent path chain. The artifact stem is one
portable ASCII filename component. Publication is create-only: complete bytes are written,
flushed, and synchronized through a new sibling temporary file before the absent final name is
committed with a hard link. Existing destinations are never replaced, and a failed build never
reports a new artifact. Non-fatal provider and publication warnings remain observable on success.

Generated modules are imported and executed by the integration suite with Node.js 22.22.1. The
driver does not yet expose a public Node runner or build/run CLI.

`compile_webassembly` independently connects the same authenticated source-to-verified-IR path to
the direct core WebAssembly backend. The backend returns private bytes only after explicit
WebAssembly 1.0 validation and the narrower import-free `I32V1` structural audit. The driver then
publishes `<stem>.wasm` create-only through the same revalidated `.zryna/out` capability and atomic
byte writer used by JavaScript. The public publisher accepts only the sealed validated artifact;
it cannot publish arbitrary WebAssembly bytes. Same-stem `.mjs` and `.wasm` files may coexist.

Node.js 22.22.1 exercises the published module with the standard WebAssembly API as a conformance
harness, not a production build phase or browser claim.

`compile_native_object` independently runs source → verified IR → verified native MIR → exact
target selection → audited ELF object → create-only `<stem>.o` publication. It reuses
`ArtifactOutputRoot`, portable stems, and the atomic byte publisher, so `.mjs`, `.wasm`, and `.o`
may coexist without replacement. Unsupported targets and every earlier failure create no object.
Product linking/running, the atomic three-target CLI, and differential execution remain later M1
gates.
