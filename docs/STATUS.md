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

## Implemented M2 compiler components

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
- These are internal compiler boundaries only. The public driver still selects protocol v2; no
  CLI command or manifest exposes M2, and the M2 native backend remains unavailable. The
  executable three-target M2
  profile and every dependent M2 issue therefore remain unsupported.

## Runtime and toolchain boundary

- JavaScript and WebAssembly execution require an absolute direct Node.js `22.22.1` executable.
- Native object emission is fixed to `x86_64-unknown-linux-gnu` and uses pinned pure-Rust Cranelift.
- Native executable linking and execution require canonical `/usr/bin/gcc` and GNU ld versions
  documented in the CLI and native executable specifications.
- Successful build and run output is published only below `.zryna/out` in atomic, create-only
  bundles with a deterministic manifest.

## Deliberately unsupported

Public source-level Boolean execution remains driver/profile-gated even though sealed internal M2
JavaScript tests execute strict Boolean carriers. The current public executable slice does not
claim source control flow, modules,
heap values, browser execution, WASI, Windows or macOS native execution, static native executables,
package resolution, watch mode, incremental builds, production readiness, or an executable M2
feature.

## Evidence and reference

- [CLI contract](CLI.md)
- [Compiler architecture](ARCHITECTURE.md)
- [M1 conformance evidence](M1_CONFORMANCE.md)
- [M2 deterministic module closure](M2_MODULE_CLOSURE.md)
- [M2 straight-line semantics](M2_STRAIGHT_LINE_SEMANTICS.md)
- [M2 control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md)
- [M2 deterministic JavaScript backend](M2_JAVASCRIPT_BACKEND.md)
- [M2 direct core WebAssembly backend](M2_WEBASSEMBLY_BACKEND.md)
- [Roadmap](ROADMAP.md)
- [Scalar ABI v1](../spec/abi/SCALAR_V1.md)
- [Language overview](../spec/language/OVERVIEW.md)
