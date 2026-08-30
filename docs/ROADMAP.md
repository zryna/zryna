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
   source slice; public CLI integration remains step 10);
8. emit, validate, publish, and execute a direct import-free core WebAssembly module for the
   current `I32V1` scalar slice (implemented by Issue #16; a strict typed WebAssembly host wrapper
   and Boolean source/IR remain later gates);
9. lower native MIR to a real object, link, and run a Linux x86-64 executable;
10. expose explicit CLI build and run targets;
11. compare JavaScript, WebAssembly, and native results, including `i32` boundaries;
12. publish versioned status and reference documentation for website consumption.

Completion gates:

- one checked source fixture runs on all three targets;
- results and wrapping `i32` behavior match exactly;
- invalid source fails with stable diagnostics and no fallback;
- Linux end-to-end and Windows portability checks pass in CI;
- the website accurately distinguishes implemented and planned behavior.

## M2 — Control Flow and Modules

- exact arithmetic and comparisons with boundary tests;
- local bindings and function calls;
- `if` and `while`;
- deterministic module resolution and multi-file builds;
- JavaScript, WebAssembly, and native differential suites.

Completion gate: the control-flow and module conformance corpus passes on every supported target.

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
