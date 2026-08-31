# Compiler status

Status channel: `next`

Zryna is an experimental compiler project. It is not production-ready, and the current executable
profile is intentionally narrow.

## Implemented M1 slice

- One workspace-relative `.zry` entrypoint is read through the pinned TypeScript 6 syntax provider,
  checked by Zryna semantics, and admitted through one verified Universal IR authority.
- The executable `I32V1` profile supports exported functions with explicit `i32` parameters and
  results, in-range decimal literals, parameter references, and signed wrapping `i32` addition.
- `zryna build` emits direct ECMAScript modules, import-free core WebAssembly modules, and audited
  Linux x86-64 ELF objects through explicit `javascript`, `webassembly`, `native`, or `all` target
  selection.
- `zryna run` executes JavaScript, core WebAssembly, and Linux x86-64 native artifacts for one typed
  scalar invocation and commits one complete create-only bundle.
- The M1 conformance suite observes `1 + 2`, `i32::MAX + 1`, and `i32::MIN - 1` through all three
  targets on Linux, compares fixed expected values and typed outcomes, and audits the manifest and
  exact artifact inventory.
- Linux and Windows both verify JavaScript/WebAssembly behavior, target-independent source
  rejection, Boolean scalar-carrier normalization, and repository portability. Windows native and
  `all` execution fail closed with `ZRYNA-N4002` and publish no bundle.

## Implemented M2 explicit profile

- The separate `zryna-syntax::v3` boundary defines exact provider-neutral DTOs for M2 syntax,
  verifies every graph, budget, spelling, and nested span against one authoritative source map,
  and exposes only opaque verified views. The pinned TypeScript 6 protocol-v3 worker and typed
  frontend transport provide the matching syntax-only implementation while retaining immutable
  protocol-v2 behavior.
- The isolated `zryna-ir::control_flow_v1` component verifies the frozen raw `ControlFlowV1`
  program model into opaque, source-map-bound views with bounded scalar operations, direct calls,
  explicit control-flow edges, dominance, reducibility, call-graph, ABI, and resource checks.
- The driver-owned [M2 module closure](M2_MODULE_CLOSURE.md) resolves only verified explicit
  relative `.zry` imports from retained no-follow workspace capabilities, authenticates exactly
  one final complete protocol-v3 source map, and seals canonical ordered modules, edges, source
  hashes, and a cross-platform graph identity under fixed resource limits.
- When invoked from the driver, the internal
  [M2 straight-line semantic boundary](M2_STRAIGHT_LINE_SEMANTICS.md) revalidates that exact final
  graph, owns modules, exports, scopes, exact scalar types, locals, assignment, and acyclic direct
  calls, preserves once-only left-to-right evaluation, and returns only mandatory-verifier-sealed
  `ControlFlowV1`. The internal [M2 control-flow boundary](M2_CONTROL_FLOW_SEMANTICS.md) adds
  canonical `if`/`while`, definite merge and loop state, reachability, and all-path return analysis.
  Independent callers must supply a complete source-map-bound verified snapshot.
- The internal [M2 deterministic JavaScript backend](M2_JAVASCRIPT_BACKEND.md) consumes only those
  opaque verified views. It exhaustively lowers exact scalar operations, private direct calls,
  returns, parallel jump edges, strict Boolean branches, and loops into bounded byte-deterministic
  ESM with typed `i32`/`bool` entry wrappers and sealed export aliases. Internal execution imports
  those public aliases through the existing pinned, deadline- and output-bounded Node capability.
- The internal [M2 direct core WebAssembly backend](M2_WEBASSEMBLY_BACKEND.md) consumes the same
  opaque verified views. It exhaustively lowers the exact scalar and CFG inventory into bounded,
  byte-deterministic WebAssembly 1.0 containing only type, function, export, and code sections,
  then validates and audits the complete bytes. Internal typed execution passes those same sealed
  bytes over standard input to an inline pinned Node host, with no staged-path reopen race.
- The internal [M2 verified native MIR profile](M2_NATIVE_MIR.md) lowers sealed `ControlFlowV1`
  one-for-one into deterministic target-specific modules, symbols, typed values, direct calls,
  blocks, parallel edges, and terminators, then independently verifies every raw claim before
  exposing opaque views and a rebuilt scalar ABI.
- The internal [M2 Linux x86-64 native backend](M2_NATIVE_BACKEND.md) consumes only that verified
  MIR, emits local typed bodies plus scalar-ABI wrappers, and closes the exact ELF section, symbol,
  and call-graph-bound relocation inventory before constructing an artifact. The driver prepares
  link/run only through the artifact-bound scalar ABI and retains the private staging identity.
- The public driver now composes those boundaries only when exact `--profile control-flow-v1` is
  present. It discovers and authenticates one complete module graph, lowers it once, dispatches the
  same opaque verified authority to selected targets, and commits one create-only atomic bundle
  with deterministic [`zryna-manifest-v2.json`](M2_MANIFEST_V2.md). Omitting `--profile` preserves
  protocol v2, `I32V1`, manifest v1, and every M1 command and bundle contract.
- Issue #55 makes individual explicit-profile M2 build/run requests available. Issue #56 adds the
  independent fixed-oracle three-target M2 conformance registry and aggregate required gate.
- The explicit `control-flow-v1` profile is implemented and covered by the required fixed-oracle
  Linux and Windows `m2` gate. Omitting `--profile` preserves the M1 syntax, semantics, CLI, and
  manifest-v1 contracts.
- Issue #57 records the separate authenticated website import, deployment, and live commit/digest
  evidence. This compiler status does not assert that an external website deployment has occurred.

## Specified M3 profile with internal syntax and layout authorities

Issue #75 specifies the separate future `DataOwnershipV1` profile and exact CLI spelling
`data-ownership-v1`. The normative data/ownership, aggregate-layout, and ownership-runtime-ABI
documents plus the digest-pinned `tests/m3-contract-v1.json` registry freeze Issues #75–#90 and
their dependency graph.

The compiler-owned `zryna-layout` component now verifies complete source-map-bound raw type graphs,
assigns canonical dense TypeIds independent of discovery order, rejects by-value recursion and
unstorable borrows, computes checked `Linear32V1` and `LinuxX8664V1` structs, enums, fixed arrays,
and handle layouts, and seals exact SHA-256 layout documents behind opaque immutable views. Shared
machine-readable fixtures pin every normative layout row and the exact five-type Pair fingerprints.
The authority is not reachable from the M1/M2 driver or CLI, allocates no runtime memory, and emits
no target artifact.

The separate `zryna-syntax::v4` boundary now decodes a closed, bounded M3 syntax contract and
authenticates it against one exact final `SourceMap`. Its module-flat type arena and function
arenas preserve source order and exact UTF-8 spans for nominal struct/enum declarations,
compiler-known containers and references, aggregate construction, projection, matching, and
weak-upgrade syntax. The pinned TypeScript 6 protocol-v4 worker is syntax-only and advertises no
module-resolution or semantic authority. A typed frontend process boundary requires the exact v4
capability tuple and fails closed before exposing an opaque source-bound snapshot. Protocol v2 and
v3 behavior remains unchanged.

The public compiler still does not accept M3 declarations or values, select syntax protocol v4,
expose DataOwnershipV1 IR, provide an allocator or ownership runtime, emit memory-bearing M3
JavaScript/WebAssembly/native artifacts, or accept `--profile data-ownership-v1`. Default M1 and
explicit `control-flow-v1` M2 remain the only public profiles.

The first planned executable slice is an internal scalarizable `Pair` struct observed through a
scalar ABI v1 result. Later dependency-ready issues add checked aggregates, owned String and Vec,
moves and deterministic drops, bounded lexical borrows, explicit shared/weak references, three
target implementations, an atomic manifest v3 CLI, fixed-oracle conformance, and authenticated
website publication. Tracing GC, public aggregate ABI, raw pointers, unsafe, FFI, threads, WASI,
Components, custom allocators, and freestanding targets remain outside M3.

## Runtime and toolchain boundary

- JavaScript and WebAssembly execution require an absolute direct Node.js `22.22.1` executable.
- Native object emission is fixed to `x86_64-unknown-linux-gnu` and uses pinned pure-Rust Cranelift.
- Native executable linking and execution require canonical `/usr/bin/gcc` and GNU ld versions
  documented in the CLI and native executable specifications.
- Successful build and run output is published only below `.zryna/out` in atomic, create-only
  bundles with a deterministic profile-specific manifest. Windows native and `all` run reject
  before publication with `ZRYNA-N4002`; there is no partial-target fallback.

## Deliberately unsupported

Public source-level Boolean execution requires explicit `control-flow-v1`; it remains rejected by
the default M1 path. The compiler-owned M2 gate checks three-target equivalence for one fixed
source control-flow and module oracle; it is not a claim of general language completeness. The
current public executable profiles do not claim heap values, an allocator, a tracing-GC profile,
browser execution, WASI, Windows or macOS native execution, static native executables, package
resolution, watch mode, incremental builds, or production readiness. The absence of heap or GC
capabilities in these scalar profiles is not a general zero-runtime or GC-free guarantee for future
data profiles.

## Evidence and reference

- [CLI contract](CLI.md)
- [Compiler architecture](ARCHITECTURE.md)
- [M1 conformance evidence](M1_CONFORMANCE.md)
- [M2 deterministic module closure](M2_MODULE_CLOSURE.md)
- [M2 three-target conformance](M2_CONFORMANCE.md)
- [M2 manifest and atomic bundle contract](M2_MANIFEST_V2.md)
- [M2 straight-line semantics](M2_STRAIGHT_LINE_SEMANTICS.md)
- [M2 control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md)
- [M2 deterministic JavaScript backend](M2_JAVASCRIPT_BACKEND.md)
- [M2 direct core WebAssembly backend](M2_WEBASSEMBLY_BACKEND.md)
- [M2 verified native MIR](M2_NATIVE_MIR.md)
- [Roadmap](ROADMAP.md)
- [Aggregate layout v1](../spec/memory-model/AGGREGATE_LAYOUT_V1.md)
- [Syntax protocol v4](SYNTAX_PROTOCOL_V4.md)
- [Scalar ABI v1](../spec/abi/SCALAR_V1.md)
- [Language overview](../spec/language/OVERVIEW.md)
