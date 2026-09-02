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

## Specified M3 profile with internal syntax, layout, IR, semantics, and runtime ABI declarations

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

The isolated `zryna-ir::data_ownership_v1` boundary now accepts an untrusted M3 program only with
the exact final source map, independently selected entry file, and verified `Linear32V1` and
`LinuxX8664V1` layout snapshots. It proves the complete source and CFG structure, branded layout
types and projections, ownership transitions, borrow state, and exact cleanup plans before
constructing opaque immutable views. The verified program retains both layout authorities, scalar
ABI v1 for entry-module scalar exports, and the closed `OwnershipRuntimeV1` contract identity.

The internal `zryna-semantics::data_ownership_v1` boundary now consumes an exact verified
protocol-v4 source authority, owns nominal and exact type resolution, verifies the semantic type
graph for both admitted layout targets, and lowers recursively Copy structs, enums, and fixed
arrays into that sealed IR. Fixed-array projection is constant-only and checked statically. The
Pair results are observed by a test-only scalar evaluator over opaque verified views; this is not
a production interpreter or target execution path.

The separate internal ownership-runtime ABI v1 authority now verifies the exact 17-operation
declaration set, target symbols and signatures, authenticated `Linear32V1` and `LinuxX8664V1`
layout-derived records, checked C-header evidence, operation-bound atomic-failure status, sealed
element-layout Vec stride and checked byte amplification, and pure logical transitions before
exposing opaque immutable views. It does not allocate, mutate runtime state, implement a helper,
compile or link an object, lower a backend, or activate a driver or CLI route.

Issue #81 is complete at its bounded internal private compiler boundary. That boundary
supports String literals, explicit clone, checked concatenation, moves, return cleanup, and
root-local replacement, plus Vec construction, explicit clone for exact `Vec<bool>`, `Vec<i32>`,
and `Vec<String>`, moves, return, push, checked Copy-element indexing,
and replacement of supported exact Vec roots. Private zero-argument producers and one-argument
owned identity calls are available internally. String/Vec functions also admit one canonical
top-level no-phi `if`/`else` from a bool literal or Copy bool parameter; branch-local owners drop in
reverse, incoming owners are restored exactly, and mutation of an incoming Vec fails before its
right-hand side. Private String and exact Vec result functions additionally admit one bounded
terminal `if`/`else`: each arm returns one owned-producing expression through a canonical
one-parameter owned join, and return cleanup excludes the joined value. One bounded top-level
no-carried-owner `while` evaluates its bool condition in a canonical header, reverse-drops
iteration-local owners before the backedge, restores incoming ownership state on the backedge and
false exit, and permits only the final return afterward. Its stable-place subset supports prepared
replacement of one mutable outer String and Copy-element push into one mutable outer exact Vec
without an owned header phi; Vec replacement and owned-element Vec push remain excluded. Vec construction, push,
and calls reserve parent resources before child ownership changes. The aggregate route constructs,
moves, explicitly clones, returns, and drops bounded parameter-free private straight-line owned
Struct, FixedArray, and root Enum graphs with Copy/String leaves. Structural clone retains its
source, creates a distinct owner, derives its fallible String-leaf count and root-enum active variant
from sealed authorities, and reverse-drops only the initialized result prefix on element failure.
Whole-root assignment for the same graphs is prepare-before-commit, rejects direct
self-consumption, and preserves sealed recursive cleanup for the old destination.
The verified IR now also seals projected replacement's old-subobject traversal and transfers the
prepared subtree's masks and enum refinement without disturbing siblings. The semantic producer
uses it for prepare-before-commit replacement of mutable available static String leaves and for at
most one combined private straight-line aggregate site. That aggregate site moves or explicitly
clones either a complete static Struct/FixedArray subobject rooted in a distinct local or a distinct
fully initialized exact same-type supported non-Copy whole root into a mutable available
`StructField`/`FixedArrayConstant` projection. Move consumes the selected source; clone retains it,
and both clone failure paths retain source and destination. Commit
recursively drops only the old target and retains the destination root and sibling masks. The producer also
resolves canonical static StructField and
FixedArrayConstant source places for Copy reads, exact String-leaf moves, and at most one supported
Struct/FixedArray subobject move into an exact directly initialized same-type local. It materializes
the selected subobject's complete descendants, preserves the enclosing root's masked cleanup
obligation and disjoint siblings, and rejects repeated, overlapping, or later whole-root consumption
outside the exact direct-local, final-return, and whole-root assignment transfers described below.
One complete available static Struct/FixedArray subobject may now also move directly into the final
exact-type return of a parameter-free private straight-line function. The producer materializes
and masks its complete subtree, returns its unique temporary owner, excludes that owner from
reverse cleanup, and preflights the return cleanup plan and all pending survivor actions before
source mutation.
Initialized available String leaves under those same paths now admit explicit clone into a distinct
temporary owner; failure cleanup retains the enclosing root's exact partial-state masks, and cloning
a moved or overlapping leaf fails closed. One initialized available non-Copy Struct or FixedArray
projection under the same static paths may also be cloned into the immediately following exact
same-type local. The source root and masks are retained, the result has a distinct temporary owner,
and the verifier seals one private straight-line site with layout-derived prefix failure cleanup.
An exact-type direct local declaration now transfers one
partially moved supported Struct or FixedArray root through its move-result temporary into the new
local, materializing the complete static topology and migrating exact masks at both owner renames.
One final exact-reference return now transfers the same partial root into an exact-topology
temporary before cleanup; the verifier excludes the returned owner and reverse-drops every
survivor, while missing, extra, wrong, or unsupported topology fails closed.
One distinct mutable fully initialized same-type whole-root destination now accepts that partial
Struct or FixedArray from an exact-reference source. Complete source, temporary, and destination
topology plus value/place/transition capacity are preflighted before mutation; `ReplacePlace` drops
the old destination once, installs the exact mask, and invalidates source and temporary.
One combined private straight-line projected-assignment site now also moves or explicitly clones a
complete static Struct/FixedArray subobject between distinct local roots. The immediate source
operation -> sole-use typed temporary -> `ReplacePlace` shape drops only the old target subtree and
preserves both pending roots and sibling masks. Move materializes and masks the source subtree and
preflights one value, `S + D + T + 1` places, and two transitions. Clone retains the source without
descendant places and preflights one value, `S + T + 1` places, two transitions, two cleanup plans,
and `2P + 1` cleanup actions before any mutation.
One canonical private one-parameter route additionally accepts a single-variant enum whose complete
non-Copy Struct/FixedArray payload is bound by an exhaustive one-arm `match`. The arm moves the
active payload into an exact direct local, drops the emptied enum root, and jumps without owner
arguments to the final local return. Its checked model is three blocks, two edges, three values,
four ownership transitions, one zero-action cleanup plan, and `D + 5` places for `D` payload
descendants.
The private String route reports moved uses as M3011, the aggregate/enum route reports them
as M3014, and unresolved
binding names report M3002. The gate enforces one-plan/one-site cleanup roles and the cumulative
8 MiB String-literal limit, and returns sealed semantics retaining verified IR plus the exact
runtime ABI authority. An internal bounded fault/drop-trace oracle now covers every ABI-admitted
failure of the implemented String, Vec, and aggregate-clone allocation-bearing operations plus the
separate checked Vec bounds trap; it authenticates status disposition/trap identity, pre-commit operand retention,
uncommitted-result exclusion, reverse cleanup, deterministic replay, and event limits without
executing an allocator or target runtime. General structural Vec clone beyond String elements,
nested aggregate clone graphs containing Enum, Vec, Shared, or Weak values,
aggregate-subobject moves outside that direct-local or parameter-free final-return exception, the one distinct-root static
projection replacement, or the single-variant match-local enum payload extraction, broader
enum-payload moves, dynamic or Vec-element projections, projected aggregate assignment outside the
exact static-subobject-move-or-clone-or-whole-root-move-or-clone-to-static-projection site, projected aggregate
clone outside the direct-local or distinct-root static-replacement exceptions, partial Enum
transfer or partial-root transfer in call/CFG contexts, direct projected-clone returns, public
contexts, or non-final/non-reference returns, general owned phi joins,
owned loop-carried phi joins, repeated or nested branches or loops, and general scope exits remain
deliberately unavailable future extensions; `break`, `continue`, loop-body return, and post-loop
effects remain excluded. Issue #82 is dependency-ready but remains planned until its child-issue
acceptance plan is published.

The public compiler still does not accept M3 declarations or values, select syntax protocol v4,
route DataOwnershipV1 IR, provide an allocator or ownership runtime, emit memory-bearing M3
JavaScript/WebAssembly/native artifacts, or accept `--profile data-ownership-v1`. Default M1 and
explicit `control-flow-v1` M2 remain the only public profiles.

The first planned executable slice remains an internal scalarizable `Pair` struct observed through
a scalar ABI v1 result. Its semantic oracle is implemented, but target execution is not. The
completed bounded owned String/Vec compiler boundary remains internal; later dependency-ready issues add
bounded lexical borrows, explicit shared/weak references, three target implementations, an atomic
manifest v3 CLI, fixed-oracle conformance, and authenticated website publication. Tracing GC,
public aggregate ABI,
raw pointers, unsafe, FFI, threads, WASI, Components, custom allocators, and freestanding targets
remain outside M3.

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
- [M3 Copy aggregate semantics](M3_COPY_AGGREGATE_SEMANTICS.md)
- [M3 verified data and ownership IR](M3_DATA_OWNERSHIP_IR.md)
- [M3 ownership runtime ABI authority](M3_OWNERSHIP_RUNTIME_ABI.md)
- [Roadmap](ROADMAP.md)
- [Aggregate layout v1](../spec/memory-model/AGGREGATE_LAYOUT_V1.md)
- [Syntax protocol v4](SYNTAX_PROTOCOL_V4.md)
- [Scalar ABI v1](../spec/abi/SCALAR_V1.md)
- [Language overview](../spec/language/OVERVIEW.md)
