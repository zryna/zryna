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
    ├── direct JavaScript backend → .js
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
- Architecture validation is required to fail closed; M0 remains open until adversarial scan, manifest, and dependency-graph fixtures prove that contract.
- Build, test, package, and release flows may not provide a skip-architecture switch.
- Generated output stays inside declared output roots.
- Symlinks are forbidden inside controlled source components.
- Unsupported functionality must produce a stable diagnostic, never a silent fallback.

## Current vertical slice

The repository currently establishes and tests:

- an authoritative `zryna.workspace.json` contract;
- a Rust architecture validator and `zryna architecture check` command;
- provider-neutral frontend handshake and snapshot types;
- an isolated TypeScript 6 syntax adapter pinned to `@typescript/typescript6@6.0.2`;
- a verified target-neutral IR for `i32` parameters, literals, and wrapping addition;
- direct JavaScript emission from verified IR;
- native MIR lowering and textual LLVM IR emission as a backend-boundary proof.

No WebAssembly backend exists yet. Textual LLVM IR is not yet object or executable emission. The first executable milestone will connect source lowering to direct JavaScript and WebAssembly emission, then add native code generation and linking, with all three targets checked for matching behavior.

## Run the foundation checks

Requirements:

- Rust 1.97.1
- Node.js 22.22.1 or newer
- pnpm 11.18.0

```bash
pnpm install --frozen-lockfile
cargo fetch --locked
cargo run -p zryna -- architecture check
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

Zryna is an independent project and is not affiliated with, endorsed by, or sponsored by Microsoft. TypeScript is a trademark of the Microsoft group of companies. Compatibility references describe technical interoperability only.

- Website: [zryna.com](https://zryna.com)
- Source: [github.com/zryna/zryna](https://github.com/zryna/zryna)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
