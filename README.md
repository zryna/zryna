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
- a separate internal protocol-v4 M3 syntax contract with bounded nominal data declarations,
  flat type and expression arenas, exact UTF-8 spans, a syntax-only TypeScript 6 worker, and an
  opaque source-map-bound Rust verifier; this does not activate `data-ownership-v1` or change
  protocol v2/v3;
- an internal aggregate-layout authority, a Copy struct/enum/fixed-array semantic lowerer, and a
  separate `DataOwnershipV1` Universal IR verifier that bind one exact source, target-neutral type
  universe, both admitted storage layouts, scalar ABI, and the declared ownership-runtime contract
  identity behind opaque read-only views; the in-progress Issue #81 checkpoint additionally lowers
  bounded private functions with String literals, explicit clone, checked
  concatenation, moves, return cleanup, and mutable root-local replacement, plus canonical Vec
  construction, explicit clone for exact `Vec<bool>`, `Vec<i32>`, and `Vec<String>`, moves, return,
  push, checked Copy-element indexing, and supported root-local replacement. String/Vec producers and
  one-argument owned identity calls are supported internally,
  together with one canonical top-level no-phi `if`/`else`; branch-local owners drop in reverse and
  incoming Vec mutation fails before right-hand-side evaluation. A separate bounded terminal
  String/exact-Vec `if`/`else` carries one owned-producing arm result through a canonical owned join
  and excludes the joined value from return cleanup. One bounded top-level no-carried-owner
  `while` reevaluates its bool condition in a canonical header, reverse-drops iteration-local
  owners before its backedge, restores the exact incoming state on both loop edges, and then
  reaches one final return. Its stable-place subset may replace one mutable outer String after
  complete RHS preparation or push a Copy element into one mutable outer exact Vec; the outer
  place identity remains unchanged and no owned header phi is introduced. The same parameter-free private
  straight-line aggregate gate now constructs, moves, explicitly clones, returns,
  and deterministically drops bounded owned Struct, FixedArray, and root Enum graphs containing
  Copy or String leaves. A fallible structural clone derives the exact String-leaf count and active
  root-enum variant from sealed layout and verifier state, retains its source, and reverse-drops only
  the initialized result prefix before pre-existing roots. Mutable whole-root assignment for those
  supported aggregates prepares a distinct replacement while preserving the old root, then commits
  with sealed recursive old-value cleanup. It proves one exact non-Copy owner while
  preserving addressable Copy storage and binds every cleanup plan to one exact site and role. The
  private String route reports moved
  bindings as M3011; the bounded aggregate and enum route reports moved owners as M3014;
  unresolved binding names report M3002;
  cumulative 8 MiB String-literal limits fail closed. A bounded internal fault/drop-trace oracle
  authenticates every admitted implemented String/Vec/aggregate-clone failure disposition and trap identity,
  preserves pre-commit owners, excludes uncommitted results, and pins reverse cleanup without
  executing an allocator or target runtime. General structural Vec clone beyond String elements,
  nested aggregate clone graphs containing Enum, Vec, Shared, or Weak values, owned projections,
  projected assignment, partial moves, general owned phi joins, owned loop-carried phi joins,
  repeated/nested branches or loops, general scope exits, and public owned values remain unfinished
  and unavailable; Vec loop replacement, owned-element Vec loop push, `break`, `continue`,
  loop-body return, and post-loop effects remain excluded;
- an internal ownership-runtime ABI v1 authority that verifies the exact 17-operation declaration
  set, target symbols and signatures, authenticated layout-derived records, checked header evidence,
  Vec allocation/reserve rules, and all 12 canonical Shared/Weak control transitions behind opaque
  immutable views; raw-storage and String behavior remain sealed declarations rather than executed
  state models, and it supplies no allocator, runtime, backend, artifact, driver route, CLI selector,
  or public aggregate ABI;
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
  `.o` publication;
- driver-owned discovery of one canonical Linux GNU link toolchain, sealed one-invocation harness
  generation, audited create-only `.elf` publication, bounded execution, and full-width typed
  `i32` result observation;
- public `zryna build` and `zryna run` commands for explicit `javascript`, `webassembly`, `native`,
  and `all` selections, with mandatory architecture validation, one shared verified program,
  typed `i32` invocations, and create-only atomic output bundles.
- a deterministic M2 ECMAScript backend that consumes only sealed `ControlFlowV1`
  views, lowers exact scalar operations, direct calls, branches, loops, and parallel block edges,
  and enforces typed `i32`/`bool` entry wrappers.
- a direct M2 core WebAssembly backend that consumes the same sealed views, emits only
  capability-minimal audited core bytes, and executes typed `i32`/`bool` exports from those exact
  validated bytes.
- an M2 Linux x86-64 native backend that consumes independently verified native MIR,
  emits local typed bodies plus scalar-ABI wrappers, audits exact call-graph-bound ELF relocations,
  and links/runs typed `i32`/`bool` invocations.
- an explicit `--profile control-flow-v1` public path that authenticates one bounded multi-file
  module graph, lowers it once, dispatches the same sealed authority to selected targets, and
  commits one deterministic `zryna-manifest-v2.json` atomic bundle. Omitting `--profile` preserves
  every M1 CLI and manifest-v1 contract.

### Default M1 profile

When `--profile` is omitted, the TypeScript adapter emits protocol v2 and rejects parse errors or unsupported syntax without
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
fixture under conformance tests. The native backend uses pinned pure-Rust Cranelift, selects only
`x86_64-unknown-linux-gnu`, and audits the encoded ELF object without invoking an installed tool.
Separately, the driver can prove canonical `/usr/bin/gcc` and GNU ld capabilities, link one sealed
typed invocation through a generated C11 harness, audit and create-only publish an executable, and
observe its exact four-byte result under bounded process controls. The public CLI composes these
library boundaries for the current `I32V1` slice. The repository-owned
[M1 conformance suite](docs/M1_CONFORMANCE.md) runs its ordered observations against fixed expected
values and each other on JavaScript, core WebAssembly, and Linux x86-64 native. Source-level `bool`
remains verifier-gated; Boolean evidence is limited to the typed scalar-ABI carrier contract.

The explicit M2 `--profile control-flow-v1` path is separate from that preserved default. It
accepts the documented multi-file scalar control-flow subset, including typed `bool`, and reaches
the sealed M2 verified program consumed by all three backends. See
[M2 conformance](docs/M2_CONFORMANCE.md) for the exact supported and rejected boundaries.

## Build and run

The public CLI accepts exactly one workspace-relative `.zry` entrypoint and requires an explicit
target. That is the complete M1 source set when `--profile` is omitted and the root of one explicit
relative-import module graph under `--profile control-flow-v1`. These examples use an exact Node.js
22.22.1 executable:

```bash
cargo run --locked -p zryna -- build examples/universal/add.zry --target all --name add-all --node /absolute/path/to/node
cargo run --locked -p zryna -- run examples/universal/add.zry --target all --name add-all --export add --arg=i32:20 --arg=i32:22 --node /absolute/path/to/node
cargo run --locked -p zryna -- build src/main.zry --profile control-flow-v1 --target all --name app-m2 --node /absolute/path/to/node
```

Build publishes `.mjs`, `.wasm`, and `.o`; run publishes `.mjs`, `.wasm`, and an invocation-
specific `.elf`. The selected artifacts and deterministic manifest appear together only after one
create-only atomic bundle commit below `.zryna/out`. M1 writes manifest v1; the explicit M2 profile
writes [manifest v2](docs/M2_MANIFEST_V2.md). See the [CLI reference](docs/CLI.md) for exact
syntax, single-target commands, typed argument grammar, output layout, JSON, exit statuses,
security properties, and platform limits. The repository-owned
[M2 conformance gate](docs/M2_CONFORMANCE.md) now authenticates the fixed-oracle aggregate
three-target evidence. The `next` compiler status therefore includes the explicit
`control-flow-v1` profile while omission of `--profile` preserves M1. Issue #57 records the
separate authenticated website import, deployment, and live-provenance evidence; that external
publication boundary does not broaden the compiler's implemented surface.

## Run the foundation gate

Requirements:

- Rust 1.97.1
- exact Node.js 22.22.1 for the authenticated frontend and CLI JavaScript/WebAssembly host
- pnpm 11.18.0
- Linux `/usr/bin/gcc` targeting `x86_64-linux-gnu` (GCC 12–15) with GNU ld 2.38–2.46 for native
  executable conformance; pure object generation does not require it

```bash
pnpm install --frozen-lockfile
pnpm m3:runtime-abi:quick
pnpm m3:owned:quick
pnpm m3:data:quick
pnpm m3:syntax:quick
pnpm m2:quick
pnpm preflight
pnpm m0:check
```

`pnpm m3:data:quick` is the narrowest Issue #79 edit-loop check. It authenticates the protocol-v4
fixtures, derives both aggregate layouts, verifies the Copy-only data IR, and runs the internal
Pair oracle without building public backends or waiting for remote CI. It skips only the large
exact/first-extra authenticated value-budget proof; `pnpm preflight` retains that boundary test for
the final local gate.

`pnpm m3:owned:quick` is the seconds-scale Issue #81 foundation check. It runs the focused
DataOwnershipV1 IR and semantic tests plus both modules' opacity doctests, including bounded UTF-8
String creation, clone and checked concatenation, Vec construction/exact Copy- or String-element
clone/push/Copy indexing, owner
transfer, private root-local replacement, site-bound cleanup roles, partial drop metadata, and the
retained runtime-ABI authority. It does not claim projections, owned calls/CFG, general scope
exits, a runtime, a backend, a CLI route, or a public profile. The full `pnpm preflight` gate
retains ignored proportional boundaries and the complete repository regression surface.

`pnpm m3:runtime-abi:quick` is the focused Issue #80 declaration-authority check. It verifies the
exact operation vocabulary, canonical JSON fixture, target declarations, layout binding, checked
header evidence, operation-bound atomic-failure claims, sealed-layout Vec stride and checked byte
amplification, Shared/Weak transitions, opacity, and ordinary negative cases without claiming or
executing a runtime. It runs 25 ordinary unit tests and two compile-fail doctests while leaving
three proportional boundary tests ignored. The full `pnpm preflight` gate includes all 28 unit
tests and both compile-fail doctests.

`pnpm m2:quick` is the narrowest edit-loop check for deterministic M2 JavaScript, WebAssembly, and
native objects; native MIR; module closure; retained workspace and native-stage security; and
internal M2 semantics. On Linux x86-64 it also links and runs the typed M2 call/branch/loop fixture;
other hosts prove the closed unsupported-host boundary. The native MIR command runs both the
unchanged M1 proof and the separate M2 verifier. This command avoids the broad driver/runtime
integration suite and is suitable on both Linux and Windows before running the broader gate.

`pnpm preflight` is the fast edit-loop gate. It stops on the first portable contract, formatting,
M2 driver-security, workspace-check, frontend, or syntax failure. Driver tests run before the
broader workspace check so closure and retained-filesystem regressions fail early. The complete
`pnpm m0:check` command remains the cross-platform merge authority.

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

See [CLI reference](docs/CLI.md), [Architecture](docs/ARCHITECTURE.md), [Syntax protocol v2](docs/SYNTAX_PROTOCOL_V2.md),
[Strict workspace contract](docs/STRICT_WORKSPACE.md), [Frontend providers](docs/FRONTENDS.md), and
[M2 deterministic module closure](docs/M2_MODULE_CLOSURE.md),
[M2 control-flow semantics](docs/M2_CONTROL_FLOW_SEMANTICS.md),
[M2 deterministic JavaScript backend](docs/M2_JAVASCRIPT_BACKEND.md),
[M2 direct core WebAssembly backend](docs/M2_WEBASSEMBLY_BACKEND.md), [Roadmap](docs/ROADMAP.md),
[M3 ownership runtime ABI authority](docs/M3_OWNERSHIP_RUNTIME_ABI.md),
[M3 Copy aggregate semantics](docs/M3_COPY_AGGREGATE_SEMANTICS.md),
[M3 owned-data semantic design contract](docs/M3_OWNED_DATA_SEMANTICS.md),
[M0 conformance](docs/M0_CONFORMANCE.md), and
[compiler documentation bundles](docs/DOCUMENTATION_BUNDLES.md).

## Project identity

Zryna is an independent project and is not affiliated with, endorsed by, or sponsored by Microsoft. TypeScript is a trademark of the Microsoft group of companies. Compatibility references describe technical interoperability only.

- Website: [zryna.com](https://zryna.com)
- Source: [github.com/zryna/zryna](https://github.com/zryna/zryna)

## License

Licensed under the Apache License, Version 2.0. See [LICENSE](LICENSE).
