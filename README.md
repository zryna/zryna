# Zryna

**Zryna** (pronounced *ZREE-na*) is developed at [zryna.com](https://zryna.com).

Zryna is an experimental, strict, three-target programming language project. Its source syntax begins as a deliberately restricted TypeScript-compatible subset, while its semantics, typed intermediate representation, JavaScript output, WebAssembly output, and native output belong to Zryna.

The project is at the foundation stage. It is not yet a production compiler.

## Goal

One checked source program should produce three artifacts with specified matching behavior:

```text
.zry source
    ↓
replaceable frontend reader
    ↓
Zryna syntax snapshot
    ↓
Zryna strict semantics
    ↓
verified Universal IR
    ├── direct JavaScript backend → .mjs
    ├── direct WebAssembly backend → .wasm
    └── native lowering → MIR → codegen → executable
```

The TypeScript 6 adapter is a bootstrap reader, not the language authority. It does not define Zryna types, build Zryna IR, or emit JavaScript. A future TypeScript 7 provider and a native Zryna frontend must implement the same versioned frontend contract.

## Non-negotiable rules

- `any` and implicit `any` are not part of the Zryna universal profile.
- Exact numeric types have target-independent specified behavior.
- JavaScript, WebAssembly, and native backends consume only verified Zryna IR.
- Backend crates never depend on a TypeScript adapter.
- The TypeScript adapter never constructs Zryna IR.
- Architecture validation is required to fail closed; the M0 conformance registry and required
  Linux and Windows checks prove the scanner, manifest, dependency graph, and compiler trust
  boundaries before later compiler work can merge.
- Build, test, package, and release flows may not provide a skip-architecture switch.
- Generated output stays inside declared output roots.
- Symlinks are forbidden inside controlled source components.
- Unsupported functionality must produce a stable diagnostic, never a silent fallback.

## Current vertical slice

The repository currently establishes and tests:

- an authoritative `zryna.workspace.json` contract;
- a Rust architecture validator and `zryna architecture check` command;
- an exact provider-neutral frontend handshake and fail-closed protocol-v2 executable syntax;
- a fail-closed executable syntax protocol v2 with a shared JSON Schema, bounded flat
  expression arenas, and source-map-backed verified Rust types;
- an isolated TypeScript 6 syntax adapter with its compiler implementation locked to `6.0.3`;
- Zryna-owned name resolution, strict source checking, and deterministic lowering from a verified
  protocol-v2 snapshot to unverified Universal IR;
- a driver-owned authenticated source-to-verified-IR path that preserves provider warnings and
  stops provider, semantic, or IR errors before any backend;
- a bounded, sealed `I32V1` Universal IR trust boundary for `i32` parameters, literals, and
  wrapping addition;
- a verified scalar ABI v1 authority for `i32` and `bool` signatures, deterministic JavaScript,
  core WebAssembly, and Linux x86-64 export mappings, and typed host observations;
- deterministic direct ECMAScript-module emission from verified IR, with strict scalar ABI
  argument and result validation for the current `i32` execution profile;
- driver-owned, create-only publication of complete `.mjs` artifacts through a validated
  capability for the workspace's declared `.zryna/out` directory;
- deterministic direct core WebAssembly 1.0 emission from verified `I32V1` IR, with sealed export
  names, an import-free capability audit, pinned binary validation, and create-only `.wasm`
  publication through the same validated output capability;
- native MIR lowering through an independent `VerifiedMirModule` gate that retains scalar ABI v1
  authority, plus deterministic Linux x86-64 ELF relocatable-object emission and create-only
  `.o` publication.

The TypeScript adapter emits protocol v2 and rejects parse errors or unsupported syntax without
silently producing a smaller program. The first strict semantic subset requires one source file,
one or more exported functions, explicit `i32` or `bool` annotations, parameter references,
literals, one return, and `i32` addition. `any`, missing annotations, invalid names, unresolved
references, unsupported types, invalid arithmetic, and unsupported entrypoint shapes produce
stable diagnostics. Source-level `bool` is checked and lowered, but the current `I32V1` verifier
intentionally rejects it until every backend implements the same profile; only the documented
`i32` subset currently reaches `VerifiedProgram`.

The JavaScript and WebAssembly backends consume the sealed scalar ABI mapping for the current
`I32V1` profile. JavaScript enforces canonical host carriers and exact arity. WebAssembly emits
only type, function, export, and code sections; it has no imports or ambient capabilities. Every
module passes an explicitly pinned WebAssembly 1.0 validator and narrower structural/operator
audit before publication. Both targets preserve wrapping `i32` addition and run the shared source
fixture under conformance tests. The native object path uses pinned pure-Rust Cranelift, selects
only `x86_64-unknown-linux-gnu`, audits the encoded ELF, and invokes no production compiler,
assembler, linker, or executable. Linux test code links an object only to verify the System V ABI.
This does not yet expose a public build/run CLI; source-level `bool` remains verifier-gated and
product linking/running is the next M1 gate.

## Run the foundation gate

Requirements:

- Rust 1.97.1
- Node.js 22.22.1 or newer
- pnpm 11.18.0
- a Linux C toolchain providing `cc` for the native ABI conformance tests (Linux only; not used by
  production object generation)

```bash
pnpm install --frozen-lockfile
pnpm m0:check
```

The canonical command validates and executes every registered Rust, protocol, and adapter proof
suite without a skip-architecture mode. See [M0 conformance](docs/M0_CONFORMANCE.md) for the exact
coverage and unsupported status.

## Repository map

```text
apps/        user-facing commands
crates/      Rust compiler contracts and phases
adapters/    isolated replaceable frontend providers
runtime/     target runtimes behind stable ABI contracts
spec/        normative language and target semantics
docs/        architecture, workflows, and roadmap
examples/    small conformance-oriented source programs
schemas/     editor-facing schemas; runtime validation remains authoritative
tests/       cross-component and cross-target suites
toolchains/  pinned upstream toolchain metadata, never patched source
editors/     future thin editor integrations
```

See [Architecture](docs/ARCHITECTURE.md), [Syntax protocol v2](docs/SYNTAX_PROTOCOL_V2.md),
[Strict workspace contract](docs/STRICT_WORKSPACE.md), [Frontend providers](docs/FRONTENDS.md), and
[Roadmap](docs/ROADMAP.md), and [M0 conformance](docs/M0_CONFORMANCE.md).

## Project identity

Zryna is an independent project and is not affiliated with, endorsed by, or sponsored by Microsoft. TypeScript is a trademark of the Microsoft group of companies. Compatibility references describe technical interoperability only.

- Website: [zryna.com](https://zryna.com)
- Source: [github.com/zryna/zryna](https://github.com/zryna/zryna)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
