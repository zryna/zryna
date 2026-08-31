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

The current public success profile is the one-file, explicitly typed `i32` subset documented by
`zryna-semantics`. Source-level `bool` is checked and lowered but remains rejected by the current
`I32V1` IR profile until every active backend supports it. The separate internal
`discover_module_closure` boundary resolves bounded protocol-v3 module graphs through a retained
`WorkspaceSourceRoot`, authenticates one final source map, and seals ordered modules, named-binding
edges, source hashes, and a canonical graph digest. It is documented in
[M2 deterministic module closure](../../docs/M2_MODULE_CLOSURE.md). That closure can enter internal
straight-line and control-flow semantics, but multiple files remain disabled in the public compiler
until the complete M2 backend and profile path is verified.

Internal M2 tests pass the same verifier-sealed `ControlFlowV1` program to the independent
JavaScript and direct core WebAssembly emitters. WebAssembly execution sends the exact validated
artifact bytes over bounded standard input to an inline pinned Node module, so no staged script or
module pathname is reopened. Typed `i32` and canonical `bool` lanes are normalized through the
shared scalar ABI. This internal path does not activate protocol v3, manifest v2, or a public M2
command.

`compile_javascript` connects real source to the deterministic JavaScript backend and publishes one
new `.mjs` artifact through the target-neutral `ArtifactOutputRoot` capability (with a compatible
`JavaScriptOutputRoot` alias). The capability is derived only from an absolute
workspace path's exact `.zryna/out` location and rejects missing, non-directory, symbolic-link, and
Windows reparse-point components throughout the persistent path chain. The artifact stem is one
portable ASCII filename component. Publication is create-only: complete bytes are written,
flushed, and synchronized through a new sibling temporary file before the absent final name is
committed with a hard link. Existing destinations are never replaced, and a failed build never
reports a new artifact. Non-fatal provider and publication warnings remain observable on success.

Generated modules are imported and executed by the integration suite and public CLI with a
validated exact Node.js 22.22.1 runtime. The driver owns the bounded, no-shell execution boundary;
the CLI owns only request parsing and rendering.

`compile_webassembly` independently connects the same authenticated source-to-verified-IR path to
the direct core WebAssembly backend. The backend returns private bytes only after explicit
WebAssembly 1.0 validation and the narrower import-free `I32V1` structural audit. The driver then
publishes `<stem>.wasm` create-only through the same revalidated `.zryna/out` capability and atomic
byte writer used by JavaScript. The public publisher accepts only the sealed validated artifact;
it cannot publish arbitrary WebAssembly bytes. Same-stem `.mjs` and `.wasm` files may coexist.

Node.js 22.22.1 exercises the published module with the standard WebAssembly API as a conformance
harness and as the public CLI host. This remains core, import-free WebAssembly execution, not a
browser or DOM claim.

`compile_native_object` independently runs source → verified IR → verified native MIR → exact
target selection → audited ELF object → create-only `<stem>.o` publication. It reuses
`ArtifactOutputRoot`, portable stems, and the atomic byte publisher, so `.mjs`, `.wasm`, and `.o`
may coexist without replacement. Unsupported targets and every earlier failure create no object.

`discover_linux_native_toolchain` creates a fail-closed capability for canonical `/usr/bin/gcc`,
its exact GNU x86-64 target, a supported GCC version, and its canonical supported GNU linker.
`compile_native_invocation` validates one typed invocation through Universal IR's embedded scalar
ABI authority, emits the same sealed object in memory, builds one private generated C harness, and
links it without a shell under bounded process and staging rules. The resulting ELF executable is
independently audited and create-only published as `<stem>.elf`. `run_native_invocation` accepts
only that opaque publication capability, writes its retained audited bytes into a fresh private
stage so public-path replacement cannot substitute code, bounds and isolates the child process
group, decodes its exact four-byte result channel, and returns the ABI authority's typed
`ScalarOutcome`.

The public CLI composes these library boundaries through driver-owned build and run requests. Each
request performs the mandatory architecture gate first, authenticates and verifies one entrypoint
once, and dispatches the same `VerifiedProgram` in JavaScript, WebAssembly, native order. Run
validates one typed `i32` invocation before execution. Selected artifacts, ordered results, and a
deterministic manifest are synchronized in one compiler-owned transaction directory and exposed
only by one create-only directory rename to `<stem>.build` or `<stem>.run`. Unix applies mode
`0700`; Windows relies on private ACLs inherited from the validated output root. There is no
partially advertised bundle. The driver reports ordered target observations without defining a
second runtime comparison semantics. The repository-owned
[M1 conformance suite](../../docs/M1_CONFORMANCE.md) compares the public observations, fixed
expected values, and committed manifest. See the [CLI reference](../../docs/CLI.md) and
[`EXECUTABLE.md`](../../spec/native-semantics/EXECUTABLE.md) for the normative security and behavior
contract.
