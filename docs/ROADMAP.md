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
verified M2 IR, the isolated IR verifier implemented, deterministic sealed M2 ECMAScript and direct
core WebAssembly emission with typed Node execution implemented, independently verified M2 native
MIR lowering implemented, and M2 native object and typed execution implemented. Issue #55 composes
those authorities into the explicit public `control-flow-v1` path and deterministic atomic manifest
v2 bundles. Issue #56 adds fixed-oracle aggregate three-target M2 conformance, exact invalid and
resource-boundary evidence, and a stable Linux/Windows aggregate gate. The compiler capability and
conformance surface is complete; Issue #57 records authenticated website import, deployment, and
live commit/digest evidence as a separate external closure gate.
Issue #45 freezes the normative
[scalar control-flow and modules v1](../spec/language/CONTROL_FLOW_MODULES_V1.md) contract and a
digest-pinned planning inventory. Issue #46 implements the separate exact protocol-v3 schema,
pinned TypeScript 6 syntax-only worker, opaque source-map-bound syntax verifier, and typed worker
transport without selecting it in the driver. Issue #48 implements the separate, source-map-bound
`ControlFlowV1` raw-to-verified boundary. Issue #47 implements retained-capability fixed-point
module discovery and final source-map authentication without selecting it in the public driver.
Issue #49 implements the internal [straight-line M2 semantic boundary](M2_STRAIGHT_LINE_SEMANTICS.md).
Issue #50 extends it with [canonical control-flow semantics](M2_CONTROL_FLOW_SEMANTICS.md), definite
state, reachability, and return analysis. Issue #51 implements the internal
[deterministic JavaScript backend](M2_JAVASCRIPT_BACKEND.md) over opaque verified views. Issue #52
implements the internal [direct core WebAssembly backend](M2_WEBASSEMBLY_BACKEND.md) over the same
authority, including typed execution of the exact validated bytes. Those component gates did not
independently enable a compiler profile or CLI command. Issue #53 implements the internal
[verified native MIR profile](M2_NATIVE_MIR.md), including deterministic lowering and an independent
raw-to-verified CFG, call, symbol, Boolean, ABI, dominance, and resource boundary. Issue #54 adds
the internal [M2 Linux x86-64 native backend](M2_NATIVE_BACKEND.md): deterministic Cranelift object
emission, exact call-graph-bound relocation and symbol audits, artifact-bound typed link/run, and
retained staging identity. Issue #55 adds exact public profile selection, single-analysis module
graph orchestration, typed multi-target dispatch, deterministic
[`zryna-manifest-v2.json`](M2_MANIFEST_V2.md), and one create-only atomic transaction. `I32V1`,
protocol v2, manifest v1, and all M0/M1 executable evidence remain unchanged.

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
| #52 | direct capability-minimal M2 core WebAssembly emission and execution | #50 | complete |
| #53 | independently verified native MIR control flow and calls | #50 | complete |
| #54 | audited Linux x86-64 native object, internal calls, link, and run | #53 | complete |
| #55 | explicit-profile atomic multi-file CLI and manifest v2 | #47, #51, #52, #54 | complete |
| #56 | fixed-oracle three-target conformance and required aggregate gate | #55 | complete |
| #57 | authenticated compiler documentation, website synchronization, deployment, and live closure | #56 | external closure |

The backend issues #51, #52, and #53 proceeded only after #50. The public CLI activated only after
every backend and the native execution path were available to Issue #55. Aggregate M2 conformance
is defined by the authenticated [M2 conformance gate](M2_CONFORMANCE.md) and required `m2` CI
context. Website synchronization and live provenance are external evidence tracked by #57; they do
not add compiler capabilities or certify later milestones.

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

Goal: add a separate explicit `DataOwnershipV1` profile without reinterpreting default M1 or
explicit M2. Issue #75 freezes the specification, exact non-goals, real issue graph, first internal
Pair slice, checked layout rules, ownership transitions, and non-Rust runtime ABI before any M3
implementation is activated. The canonical planning inventory is digest-pinned in
`tests/m3-contract-v1.json`.

Current status: the contract, internal verified aggregate-layout authority, separately versioned
protocol-v4 syntax boundary, isolated `DataOwnershipV1` raw-to-verified IR boundary, and internal
Copy-only struct/enum/fixed-array semantic lowerer, and sealed ownership-runtime ABI v1 declaration
authority are implemented. The semantic boundary resolves
nominal types and constant fixed-array projections, verifies both target layouts, and returns only
sealed IR. The ABI authority verifies exact declarations, authenticated layout-derived records,
checked header evidence, and pure transitions; it implements no allocator or helper. M3 is not
selected by the public driver and exposes no runtime, backend, CLI, public aggregate ABI, target
artifact, or host capability.

Issue #81 is complete at a bounded internal compiler checkpoint. Private functions cover String
literals, explicit clone, checked concatenation, moves, return cleanup, and root-local replacement,
plus Vec construction, explicit clone for exact `Vec<bool>`, `Vec<i32>`, and `Vec<String>`, moves,
return, push, checked Copy-element indexing, and supported exact
root-local replacement. Zero-argument producers and one-argument owned identity calls transfer
owners through independently verified direct-call boundaries. One canonical top-level no-phi
String/Vec branch restores its incoming owner state after reverse-dropping branch locals, and one
bounded terminal branch transfers either owned arm result through a canonical block-parameter
join. One bounded top-level no-carried-owner loop reevaluates its condition in a canonical header,
reverse-drops iteration locals before the backedge, and restores its exact incoming state on both
the backedge and false exit. Its stable-place subset replaces one mutable outer String after full
RHS preparation or pushes a Copy element into one mutable outer exact Vec without an owned header
phi; Vec replacement and owned-element Vec push remain unavailable future extensions. The same gate proves use-after-move diagnostics, one-plan/one-site cleanup roles, and the
cumulative 8 MiB String-literal limit while retaining verified IR and the exact runtime ABI
authority. Its internal fault/drop-trace oracle consumes authenticated runtime status dispositions
and exact trap identities for all admitted implemented String/Vec/aggregate-clone failures, keeps Vec bounds as a
separate verified trap, and proves pre-commit retention, result exclusion, reverse cleanup,
determinism, and bounded trace accounting without runtime execution. A bounded parameter-free
private straight-line route also constructs, moves, explicitly clones, returns, and drops owned
Struct, FixedArray, and root Enum graphs with Copy/String leaves. Structural clone retains its
source, derives its fallible String-leaf count and root-enum active variant from sealed authorities,
and uses prefix-safe failure cleanup. Mutable whole-root assignment for those graphs prepares a
distinct replacement before `ReplacePlace` commits the sealed recursive old-value drop. The private String and
aggregate/enum routes retain their distinct M3011 and M3014 moved-owner diagnostics, and unresolved
bindings use M3002. General structural Vec clone beyond String elements, nested aggregate clone
graphs containing Enum, Vec, Shared, or Weak values are future extensions. The verified IR prerequisite for
projected replacement now seals the exact old-subobject traversal and preserves replacement
subtree masks, enum refinement, and siblings. The semantic producer now resolves canonical static
StructField and FixedArrayConstant projections, retains Copy leaves, moves exact String leaves, and
moves one supported Struct/FixedArray subobject into an exact directly initialized same-type local.
The subobject path materializes all source descendants, masks that whole subtree in the enclosing
cleanup obligation, and preserves disjoint siblings. It also prepares and commits
replacement of mutable available String leaves without disturbing sibling masks, and explicitly
clones initialized available String leaves into distinct temporary owners while retaining the
source root and its partial-state masks. One initialized available non-Copy Struct or FixedArray
projection may now likewise be cloned into the immediately following exact same-type local, with
one source-retaining private straight-line site and layout-derived aggregate-prefix failure cleanup.
An exact-type direct local declaration can now transfer a
partially moved supported Struct or FixedArray root through its move-result temporary into one new
local; the producer materializes the complete static topology and migrates the exact root-relative
mask at both owner renames. One final exact-reference return also transfers that complete partial
state into an exact-topology temporary before survivor cleanup, with independent IR rejection of
forged or unsupported topology. One distinct mutable initialized same-type whole-root assignment
destination now accepts that partial Struct or FixedArray after complete source, temporary, and
destination topology plus resource preflight; replacement drops the old destination once and
installs the exact mask. One exact private single-variant enum `match` now moves its complete
supported Struct/FixedArray payload into a direct local, drops the emptied enum root, and reaches a
final local return through a zero-argument continuation with exact `D + 5` place amplification.
The combined projected-assignment checkpoint now additionally admits one complete static
Struct/FixedArray subobject move or explicit clone between distinct local roots. It requires an
immediate source operation -> sole-use typed temporary -> `ReplacePlace`. Move materializes and
masks the complete selected source subtree; clone retains its available source without descendant
places and seals layout-derived prepare/prefix failure cleanup. Both drop only the old target
subtree and preserve roots, sibling masks, and pending order. Move amplification is one value,
`S + D + T + 1` places, and two transitions. Clone amplification is one value, `S + T + 1` places,
two transitions, two cleanup plans, and `2P + 1` actions for pending-root count `P`.
One separate parameter-free private final-return checkpoint moves one complete available static
Struct/FixedArray subobject from a local root. It materializes and masks the exact source subtree,
returns the sole-use exact-type temporary, excludes that owner from survivor cleanup, and
preflights one value, `D + 1` places, one transition, one cleanup plan, and all pending cleanup
actions before mutation; a missing canonical source path adds its exact `M` places, for
`M + D + 1` total.
Enum partial-root transfer, aggregate-subobject moves outside that at-most-one direct-local or
parameter-free final-return form or
that one match-local payload extraction, and broader enum-payload moves,
transfer in call/CFG contexts, projected assignment outside the narrow site below, or
non-final/non-reference returns, dynamic projections,
direct projected-clone returns, public contexts, projected aggregate clone outside the exact
direct-local or distinct-root static-replacement forms, and projected aggregate assignment outside
one private straight-line complete static subobject move/clone between distinct local roots or
move/explicit clone from a distinct fully
initialized exact same-type whole Struct/FixedArray root into a mutable available static
`StructField`/`FixedArrayConstant` projection remain open, alongside
general owned joins, owned loop-carried joins, repeated or nested control flow, and general scope
exits are future child-issue work. Issue #82 is active through the checked dependency graph in
[`M3_BORROWING_SEMANTICS.md`](M3_BORROWING_SEMANTICS.md). Issue #113 freezes the contract and
existing verified-IR prerequisite. Issues #114 and #115 implement the internal private straight-line
`bool`/`i32` root shapes: shared and exclusive Copy access, const-alias write-through, the complete
root conflict matrix, one shared-from-shared reborrow, deterministic reverse lexical end, and
post-scope owner reuse. Issue #117 adds one private bool-root conditional with canonical
entry/then/else/join blocks, complete reverse arm discharge before each jump, dense source-ordered
borrow identities, exact summed arm costs, maximum-per-arm active capacity, and no borrow phi or
edge authority. Issue #120 adds canonical recursively Copy Struct-field and constant fixed-array
borrowing: static siblings are disjoint, same/ancestor/descendant paths overlap, projected
resources are preflighted exactly, and dynamic/Vec/enum/non-Copy projections fail closed. Issue
#121 adds one fixed preheader/header/body/exit bool-root loop whose body
reuses one static dense authority plan, reverse-ends it before every backedge, restores exact root
owner/initialization state, and carries no borrow authority, value block parameter, or edge
argument. The completed Issue #116 implementation adds one private parameter-free whole
owned root with one const shared alias in one lexical block. It admits only String clone/checked
concat, exact `Vec<bool>`/`Vec<i32>` Copy indexing, and supported whole
Struct/root-Enum/fixed-array clone; it retains the source, gives owned read results distinct owners,
reuses existing cleanup/fault authority, and leaves `BorrowRead` Copy-only. Projection, mutation,
move, runtime, backend, and public activation are excluded. Issue #116 passed independent
verification and required merge gates. Issue #119 completes the bounded private straight-line
whole-root call-only nonescape slice: exact recursively Copy signatures carry shared or exclusive
authority in source order, evaluate arguments left to right, permit same-authority forwarding, and
leave lexical `EndBorrow` with the caller. The mandatory verifier retains acyclic-call and exact
static-depth authority, while the authenticated registry freezes 36 source/snapshot files, 5
accepted cases, and 13 exclusions at merged-main provenance
`32e3f0607389dd1274c21770088456c765ee4fb7`. Protocol v4 and every runtime, backend, driver, CLI,
artifact, and public-profile boundary remain unchanged. The remaining dependency-ordered slices
retain nested/repeated control flow, runtime, backend, and public-profile work.

| Issue | Gate                                                                 | Depends on              | State       |
| ----: | -------------------------------------------------------------------- | ----------------------- | ----------- |
|   #75 | normative profile, layout, ownership, and runtime ABI contract       | M2 closure              | complete    |
|   #76 | syntax protocol v4 and TypeScript 6 syntax-only adapter              | #75                     | complete    |
|   #77 | verified aggregate layout authority                                  | #75                     | complete    |
|   #78 | separately verified DataOwnershipV1 Universal IR                     | #75, #77                | complete    |
|   #79 | struct, enum, and fixed-array semantic lowering                      | #76, #77, #78           | complete    |
|   #80 | versioned ownership runtime ABI authority                            | #75, #77                | complete    |
|   #81 | owned String/Vec, move checking, and deterministic drop              | #78, #79, #80           | complete    |
|   #82 | bounded nonescaping lexical borrowing                                | #81                     | in progress |
|   #83 | explicit shared and weak reference semantics                         | #80, #81, #82           | planned     |
|   #84 | deterministic JavaScript and sealed helpers                          | #79, #80, #81, #82, #83 | planned     |
|   #85 | audited memory-bearing core WebAssembly                              | #79, #80, #81, #82, #83 | planned     |
|   #86 | independently verified native MIR                                    | #78, #80, #81, #82, #83 | planned     |
|   #87 | audited Linux x86-64 object, runtime, link, and execution            | #77, #80, #86           | planned     |
|   #88 | candidate driver integration and atomic manifest v3 bundles          | #76, #84, #85, #87      | planned     |
|   #89 | fixed-oracle three-target conformance and resource gates             | #88                     | planned     |
|   #90 | public profile activation, authenticated docs, website, and provenance | #89                     | planned     |

The bounded #82/#120 checkpoint rejects dynamic-index and Vec-element source borrows. This does
not implement or waive the normative complete-container overlap rule. The remaining M3 capability
is explicitly tracked before complete target support and public activation:

| Issue | Normative completion work | Dependencies |
| ---: | --- | --- |
| #254 | exact indexed element access with independently verified conservative container authority | #82, retaining #77/#78/#80/#81 |
| #255 | dynamic fixed-array borrowing producer and failure/resource evidence | #254, retaining #76/#79/#81/#82 |
| #256 | Vec-element borrowing producer and failure/resource evidence | #254, retaining #76/#80/#81/#82 |

Issues #84, #85, and #86 retain their existing dependencies and also require this chain before
claiming complete M3 target support. #89/#90 remain conformance and activation gates, not owners
of missing source semantics. Copy-only staged evidence cannot close generic owned-element
requirements; any staged remainder needs explicit blocking work. These tracked gaps do not
activate new syntax or a public profile, and do not certify all other normative M3 requirements
complete. Bounded #122 closure must preserve the distinction and its actual verification gates.

The checked M3 registry records this dependency order rather than assuming that an earlier GitHub
issue number cannot depend on later-discovered work. Its current SHA-256 is
`0c144ad82ec1f601f5c2057578df001487e0bebeead002467bedcaa197ef9e51`.
The original #119 provenance and unchanged borrow-call fixture digest remain historical evidence;
updating the graph does not implement any of its planned capabilities.

`Pair` is the smallest mandatory fixed-oracle case, remains internal, and preserves scalar ABI v1
exports. Issue #79 proves nominal identity, construction, source field order and access, sealed
logical layout, verified IR, and the fixed scalar results through a test-only evaluator over opaque
verified views, without heap allocation or a production execution path. Pair becomes executable
across targets only inside the complete dependency-ordered backend, integration, and conformance
work in #84 through #89; it is not an earlier partial-profile checkpoint.

#88 produces an internally testable candidate route and manifest, not a supported public selector.
Exact `--profile data-ownership-v1` activation and support claims wait for #89 conformance and the
#90 authenticated publication gate.

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
