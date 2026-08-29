# UTS

UTS is an experimental, strict, dual-target programming language project. Its source syntax begins as a deliberately restricted TypeScript-compatible subset, while its semantics, typed intermediate representation, JavaScript output, and native output belong to UTS.

The project is at the foundation stage. It is not yet a production compiler.

## Goal

One checked source program should produce two artifacts with specified matching behavior:

```text
.uts source
    ↓
replaceable frontend reader
    ↓
UTS syntax snapshot
    ↓
UTS strict semantics
    ↓
verified Universal IR
    ├── direct JavaScript backend → .js
    └── native lowering → MIR → codegen → executable
```

The TypeScript 6 adapter is a bootstrap reader, not the language authority. It does not define UTS types, build UTS IR, or emit JavaScript. A future TypeScript 7 provider and a native UTS frontend must implement the same versioned frontend contract.

## Non-negotiable rules

- `any` and implicit `any` are not part of the UTS universal profile.
- Exact numeric types have target-independent specified behavior.
- JavaScript and native backends consume only verified UTS IR.
- Backend crates never depend on a TypeScript adapter.
- The TypeScript adapter never constructs UTS IR.
- Architecture validation is fail-closed; an incomplete scan never passes.
- Build, test, package, and release flows may not provide a skip-architecture switch.
- Generated output stays inside declared output roots.
- Symlinks are forbidden inside controlled source components.
- Unsupported functionality must produce a stable diagnostic, never a silent fallback.

## Current vertical slice

The repository currently establishes and tests:

- an authoritative `uts.workspace.json` contract;
- a Rust architecture validator and `uts architecture check` command;
- provider-neutral frontend handshake and snapshot types;
- an isolated TypeScript 6 syntax adapter pinned to `@typescript/typescript6@6.0.2`;
- a verified target-neutral IR for `i32` parameters, literals, and wrapping addition;
- direct JavaScript emission from verified IR;
- native MIR lowering and textual LLVM IR emission as a backend-boundary proof.

Textual LLVM IR is not yet object or executable emission. The first real native milestone will add a code-generation implementation and linker integration after MIR invariants are frozen.

## Run the foundation checks

Requirements:

- Rust 1.97.1
- Node.js 22.22.1 or newer
- pnpm 11.18.0

```bash
pnpm install --frozen-lockfile
cargo run -p uts-cli -- architecture check
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
pnpm adapter:check
pnpm adapter:test
```

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

See [Architecture](docs/ARCHITECTURE.md), [Strict workspace contract](docs/STRICT_WORKSPACE.md), [Frontend providers](docs/FRONTENDS.md), and [Roadmap](docs/ROADMAP.md).

## Project identity

UTS is an independent project and is not affiliated with, endorsed by, or sponsored by Microsoft. TypeScript is a trademark of the Microsoft group of companies. Compatibility references describe technical interoperability only.

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
