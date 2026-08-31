# Delivery roadmap

This file is the authoritative delivery ledger. GitHub milestones mirror these gates; issues carry implementation detail and may not weaken the architecture contract. Milestones have no invented dates. A milestone closes only when its observable acceptance gates pass.

## Delivery workflow

```text
roadmap gate
    ↓
dependency-ready issue
    ↓
focused branch and implementation
    ↓
local checks and independent verification
    ↓
pull request and required CI
    ↓
merge to main
    ↓
compiler-owned documentation update
```

Every issue defines its problem, architecture boundary, scope, exclusions, dependencies, acceptance criteria, required tests, documentation impact, and security or runtime impact. Public changes use focused human-readable metadata and never bypass `main` through a direct implementation push.

## M0 — Architecture Foundation

Status: complete. The provider-neutral executable syntax protocol, bounded bootstrap adapter,
sealed Universal IR, and independently verified native MIR boundary are enforced by the canonical
[M0 conformance gate](M0_CONFORMANCE.md). The same fail-closed gate passed locally and in required
Linux and Windows checks, independent closure review found no unresolved P0 or P1 issue, and the
[public compiler status](https://zryna.com/reference/compiler-status/) matches the implemented
surface. At M0 closure, Zryna-owned semantic lowering was the first compiler step scheduled for
M1.

- strict repository contract and fail-closed architecture engine;
- stable diagnostics and source spans;
- frontend handshake and normalized snapshot contract;
- exact TypeScript 6 adapter pin;
- provider-neutral syntax DTOs owned below replaceable frontend providers;
- verified Universal IR;
- independent verified-only JavaScript and native backend boundaries; direct WebAssembly emission
  begins in M1;
- Linux and Windows CI;
- governed milestones, issue templates, and protected `main` gates.

Completion gates:

- every documented foundation check passes locally and in CI;
- scanner, manifest, dependency-graph, source, diagnostic, protocol, adapter, IR, MIR, and backend
  boundaries fail closed under registered negative tests;
- the component graph gives semantic lowering access to syntax DTOs without a compiler-to-frontend dependency;
- the public architecture and current-status claims match implemented behavior;
- the three-target roadmap and dependency order are published;
- `main` requires pull requests and successful CI without force pushes.

Closure evidence and the deliberately unsupported post-M0 surface are recorded in
[M0 conformance](M0_CONFORMANCE.md).

## M1 — First Three-Target Executable Slice

Goal: compile one `.zry` entrypoint to executable JavaScript, direct WebAssembly, and a Linux x86-64 native executable with matching specified behavior.

Current status: Issue #19 implements the explicit build/run CLI and atomic target bundles for the
`I32V1` slice. Issue #20 implements the checked three-target differential corpus, portable
JavaScript/WebAssembly matrix, invalid-source matrix, and scalar-ABI Boolean normalization proof.
M1 closure evidence includes versioned website-facing status and reference data published from the
authenticated compiler documentation bundle tracked in Issue #21.

Dependency order:

1. extend the lower-layer provider-neutral syntax contract with normalized function bodies,
   parameters, literals, returns, `bool`, and `i32` (implemented);
2. make the TypeScript 6 adapter produce that snapshot without owning Zryna semantics (implemented);
3. add Zryna-owned name resolution, strict semantic checking, and lowering to unverified IR
   (implemented by Issue #14);
4. reject `any`, implicit `any`, malformed provider data, and unsupported syntax with stable source
   diagnostics (implemented by Issue #14);
5. verify exact IR operations before any backend accepts the program (implemented by Issue #14);
6. freeze scalar ABI v1: logical export names, target symbol mapping, `i32` and `bool`
   representation, invocation, and host-result normalization (implemented by Issue #13);
7. emit and execute an ECMAScript module (implemented by Issue #15 for the current `I32V1`
   source slice; public CLI integration implemented by Issue #19);
8. emit, validate, publish, and execute a direct import-free core WebAssembly module for the
   current `I32V1` scalar slice (implemented by Issue #16; a strict typed WebAssembly host wrapper
   and Boolean source/IR remain later gates);
9. lower native MIR to a real audited Linux x86-64 object (object emission implemented by Issue
   #17; driver-owned sealed linking and typed execution implemented by Issue #18);
10. expose explicit CLI build and run targets (implemented by Issue #19 for the `I32V1` slice,
    including atomic multi-target bundles and ordered observations);
11. compare JavaScript, WebAssembly, and native results, including `i32` boundaries (implemented
    by Issue #20 through the repository-owned M1 conformance suite);
12. publish versioned status and reference documentation for website consumption (Issue #21).

Completion gates:

- one checked source fixture runs on all three targets;
- results and wrapping `i32` behavior match exactly;
- invalid source fails with stable diagnostics and no fallback;
- Linux end-to-end and Windows portability checks pass in CI;
- the website accurately distinguishes implemented and planned behavior.

## M2 — Control Flow and Modules

Goal: add a separately verified `ControlFlowV1` profile with exact scalar arithmetic, Boolean
comparisons, lexical locals, direct calls, structured branches and loops, deterministic modules,
and one multi-file program whose behavior matches on JavaScript, direct core WebAssembly, and
Linux x86-64 native output.

Current status: contract specified, exact syntax protocol v3 implemented, deterministic module
closure implemented, modules/scopes/types/calls and canonical `if`/`while` control flow lower to
verified M2 IR, the isolated IR verifier implemented, and deterministic sealed M2 ECMAScript
emission and typed Node execution implemented internally, but the public M2 profile and
cross-target execution remain unavailable. Issue #45 freezes the normative
[scalar control-flow and modules v1](../spec/language/CONTROL_FLOW_MODULES_V1.md) contract and a
digest-pinned planning inventory. Issue #46 implements the separate exact protocol-v3 schema,
pinned TypeScript 6 syntax-only worker, opaque source-map-bound syntax verifier, and typed worker
transport without selecting it in the driver. Issue #48 implements the separate, source-map-bound
`ControlFlowV1` raw-to-verified boundary. Issue #47 implements retained-capability fixed-point
module discovery and final source-map authentication without selecting it in the public driver.
Issue #49 implements the internal [straight-line M2 semantic boundary](M2_STRAIGHT_LINE_SEMANTICS.md).
Issue #50 extends it with [canonical control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md), definite
state, reachability, and return analysis. Issue #51 implements the internal
[deterministic JavaScript backend](M2_JAVASCRIPT_BACKEND.md) over opaque verified views. This does
not enable a compiler profile, a CLI command, or a public M2 support claim. `I32V1`, protocol v2,
manifest v1, and all M0/M1 executable
evidence remain unchanged while the remaining M2 gates are built on those verified boundaries.

Dependency ledger:

| Issue | Gate | Depends on | Ledger state |
| --- | --- | --- | --- |
| #45 | normative scalar, control-flow, module, IR, budget, and planned-conformance contract | M1 closure | complete |
| #46 | exact protocol v3 and pinned TypeScript 6 syntax adapter | #45 | complete |
| #47 | compiler-owned bounded deterministic module closure | #45, #46 | complete |
| #48 | independently verified `ControlFlowV1` Universal IR | #45 | complete |
| #49 | Zryna-owned modules, scopes, types, arithmetic, comparisons, locals, assignment, and direct calls | #46, #47, #48 | complete |
| #50 | canonical `if`/`while`, definite state, reachability, and return lowering | #49 | complete |
| #51 | deterministic M2 ECMAScript emission and execution | #50 | complete |
| #52 | direct capability-minimal M2 core WebAssembly emission and execution | #50 | planned |
| #53 | independently verified native MIR control flow and calls | #50 | planned |
| #54 | audited Linux x86-64 native object, internal calls, link, and run | #53 | planned |
| #55 | explicit-profile atomic multi-file CLI and manifest v2 | #47, #51, #52, #54 | planned |
| #56 | fixed-oracle three-target conformance and required aggregate gate | #55 | planned |
| #57 | authenticated compiler documentation, website synchronization, deployment, and live closure | #56 | planned |

The backend issues #51, #52, and #53 may proceed in parallel only after #50. Public CLI activation
waits for every backend and the native execution path. Public website status remains M1 until #56
passes required CI and #57 verifies the exact compiler documentation bundle live.

Completion gates:

- exact fixed-oracle arithmetic, comparison, Boolean, local, call, branch, loop, and module
  cases pass on JavaScript, direct core WebAssembly, and Linux x86-64 native output;
- invalid syntax, modules, names, types, CFG claims, paths, cycles, races, and `limit + 1` cases fail
  before backend work and publish no bundle;
- Windows runs the portable JavaScript/WebAssembly corpus and retains explicit native
  unavailability;
- one explicit `control-flow-v1` command analyzes one source graph once and atomically publishes a
  deterministic manifest v2 plus selected artifacts;
- required M0/M1 checks remain intact and a stable aggregate M2 check is protected;
- compiler-owned authenticated documentation, website CI, deployment, and live inspection all
  agree on the exact compiler commit and bundle digest.

## M3 — Data, Memory, and Ownership

- structs, enums, fixed arrays, and verified layouts;
- owned strings and vectors;
- move checking and deterministic drop insertion;
- borrowing;
- explicit shared and weak references;
- versioned native and WebAssembly runtime ABIs.

Completion gate: ownership and layout behavior is specified before implementation and remains equivalent across target-specific representations.

An optional tracing-GC profile requires a separate language and ABI proposal. It is not implied by the no-GC ownership profile.

## M4 — WebAssembly Components and WASI

- keep `wasm-web` as a capability-minimal browser profile;
- define pinned WIT worlds and canonical interfaces;
- add an explicitly versioned WASI profile;
- generate Component Model artifacts and host bindings;
- test denied and granted capabilities;
- publish browser, command, and server examples.

Completion gate: components expose self-described interfaces and receive no filesystem, network, clock, randomness, or environment capability unless the selected profile declares it.

## M5 — Packages and Reproducible Releases

- package manifest, deterministic dependency resolution, and lockfile;
- semantic versioning and compatibility policy;
- reproducible artifacts, checksums, SBOM, and provenance;
- npm, crates.io, Docker, and GitHub release publication;
- signed release notes and rollback procedure.

Completion gate: a clean environment reproduces and verifies every published artifact from a tagged source revision.

## M6 — Developer Tooling

- formatter and structured diagnostics;
- language server and thin editor integrations;
- source maps and debugging contracts;
- browser playground and runnable documentation;
- Open VSX and Visual Studio Marketplace release automation.

Completion gate: editor and playground behavior is driven by compiler contracts and does not duplicate language semantics.

## M7 — Independent Frontend and Stabilization

- native Zryna lexer, parser, resolver, and snapshot provider;
- provider conformance against the bootstrap TypeScript adapter;
- generics and monomorphization, `Option`, and `Result` after separate specifications;
- native-only FFI profile;
- compatibility, performance, and security gates;
- additional platforms after conformance gates exist;
- documented 1.0 stability criteria.

Completion gate: the native frontend can replace the bootstrap provider without changing verified IR or backend behavior, and every 1.0 compatibility gate is reproducible.
