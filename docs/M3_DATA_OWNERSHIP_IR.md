# M3 verified data and ownership IR

Status: implemented internal compiler trust boundary. This document describes the separately
verified `DataOwnershipV1` Universal IR added by issue #78 and the ownership-proof foundations
extended for issue #81. It does not activate a runtime, backend, driver route, CLI profile,
manifest, or public aggregate ABI.

## Authority and isolation

`zryna-ir::data_ownership_v1` is independent of the M1 `I32V1` and M2 `ControlFlowV1` raw and
verified programs. It does not add a type, operation, diagnostic, artifact field, or acceptance
case to either older profile.

The only constructor of its verified program has this authority shape:

```text
verify(
    raw Program,
    &SourceMap,
    expected entry FileId,
    owned Linear32V1 VerifiedLayouts,
    owned LinuxX8664V1 VerifiedLayouts,
) -> Result<VerifiedProgram, Vec<Diagnostic>>
```

The raw program carries these exact independent claims:

```text
AuthorityClaims {
    runtime: OwnershipRuntimeV1,
    type_universe: [u8; 32],
    linear32_fingerprint: [u8; 32],
    linux_x86_64_fingerprint: [u8; 32],
}
```

Verification requires both layout snapshots to have the same `SourceMapIdentity` as the supplied
source map and the same `TypeUniverseIdentity` as one another and the raw claim. Their targets must
be exactly `Linear32V1` and `LinuxX8664V1`, respectively, and their complete sealed fingerprints
must match the corresponding claims. Every IR `TypeId` is looked up through both snapshots; a
foreign universe, unknown index, target swap, or fingerprint mismatch fails closed. Success moves
both snapshots into the verified program, so later target work can consume the authority without
reconstructing layout.

The expected entry file is independent of the raw entry-module claim. Modules are dense and in
the final source-map order, each module is bound to one exact `FileId`, and every span must resolve
through that source map inside its containing module. The verifier derives CFG predecessor,
successor, reachability, dominance, loop, and exit facts from the exactly-one terminator in each
block; raw code cannot supply a competing CFG authority.

Only entry-module scalar exports enter scalar ABI v1. Aggregate, owned, shared, weak, and borrowed
values cannot appear in the public ABI. The verified program retains the sealed scalar ABI beside
the two layout snapshots.

## Closed raw vocabulary

The raw program owns dense arenas for modules, functions, blocks, values, places, borrows, cleanup
plans, and cleanup actions. Ownership transitions are re-derived from instructions and CFG edges;
there is no raw transition enum whose claims could substitute for that proof. IDs are local to
their documented owner. A numerically
equal ID from another program, module, function, block, type universe, or arena has no authority.
Every retained arena entry must be reachable and have exactly the required owner; sparse,
duplicate, cross-owner, shared, and orphan claims are rejected.

Each raw `TypeId(u32)` is only an untrusted canonical-index claim. Verification resolves it through
both branded `zryna-layout::TypeId` universes. Stored type structure, nominal identity, field and
variant order, array length, element or payload type, drop kind, and runtime kind come only from
those layout views. Borrow parameters and definitions separately pair one resolved referent type
with exact `Shared` or `Exclusive` access. The IR never treats the raw integer as authority or
recomputes a host layout.

The closed `PlaceKind` inventory is exactly:

- `Parameter(u32)`, `Local(u32)`, or `Temporary(ValueId)` roots;
- `StructField { base, ordinal }` selected by its sealed source ordinal;
- `EnumPayload { base, variant }` selected by its sealed variant ordinal and dominating variant
  proof; or
- `FixedArrayConstant { base, index }` selected by one in-range constant index.

Each projection records its claimed result type, but verification derives that type step by step
from the base place and layout authority. Projection through another category, an invalid ordinal
or index, an inactive enum payload, or a mismatched type is rejected. Parent places overlap all
children; distinct struct fields and distinct constant array indices are disjoint. Dynamic fixed
array indexing and vector indexing are value-producing instructions and conservatively borrow the
complete container rather than manufacturing independently movable projected places.

The exact closed `InstructionKind` inventory is:

```text
BoolLiteral, I32Literal,
I32Add, I32Sub, I32Mul, I32Neg,
Eq, Ne, I32LtS, I32LeS, I32GtS, I32GeS,
DirectCall,
StructConstruct, EnumConstruct, FixedArrayConstruct,
CopyFromPlace, MoveFromPlace, ClonePlace,
InitializePlace, ReplacePlace, DropPlace,
EnumDiscriminant, FixedArrayIndexCopy, VecIndexCopy,
StringFromUtf8, StringClone, StringConcat,
VecClone, VecConstruct, VecPush,
SharedConstruct, SharedClone,
WeakDowngrade, WeakClone,
BeginBorrow, BorrowRead, BorrowWrite, EndBorrow
```

`BeginBorrow` contains one dense `BorrowDefinition` with `Shared` or `Exclusive` access.
Potentially trapping calls, String construction, allocation-bearing constructions, clones,
indexing, concatenation, vector growth, shared construction, and reference-count increments name
the cleanup plan required by their exact raw variant. `StringFromUtf8` carries verifier-checked
immutable UTF-8 bytes and a prepare-failure cleanup identity; it is internal IR vocabulary, not a
public String runtime or compiler profile. `VecClone` currently admits only exact `Vec<bool>`,
`Vec<i32>`, and `Vec<String>` sources, preserves the source owner, and requires a distinct temporary
result owner. Every clone binds allocation failure to its exact prepare cleanup. `Vec<String>` also
requires a separately site-bound `VecCloneElementFailure` plan whose first typed action is
`VecInitializedPrefix`, followed by every pre-existing owner in reverse order. Executable consumers
must use `vec_clone_element_failure_drop_actions()` and its typed action kind for that role; the
root-only cleanup-plan compatibility view does not turn the prefix into an ordinary whole-place
drop. This does not claim general `Vec<T: Clone>` support. `ReplacePlace` is the infallible commit after its
replacement value has already been prepared, so it carries no cleanup plan. `DropPlace` supplies
the logical release operation for String, Vec, Shared, and Weak places; there are no separately
forgeable release-helper instructions.

`ClonePlace` additionally admits supported non-Copy Struct, FixedArray, and root Enum graphs with
String leaves. It preserves the source and produces one distinct result owner. Fallible String-leaf
clone binds a separate `AggregateCloneElementFailure` plan whose first typed action is
`AggregateInitializedPrefix`, followed by every pre-existing live root in reverse order. Verified
consumers use `aggregate_clone_element_failure_drop_actions()` for that role and obtain the exact
fallible-leaf count through `aggregate_clone_fallible_leaf_count()`. The verifier derives that count
from retained Linear32 layout and derives a root Enum's active variant from source ownership state;
neither is caller-controlled. Nested Enum, Vec, Shared, Weak, recursive, and cyclic graphs remain
excluded.

There is no generic target helper, raw address, source-selected runtime symbol, implicit clone,
implicit conversion, unchecked index, host exception, or arbitrary runtime call instruction.
Left-to-right operand order is part of every operation. Aggregate operands must exactly match the
verified field, payload, or element inventory and types.

Every block ends in exactly one terminator from this closed inventory:

```text
Return { value, cleanup }
Jump(Edge)
Branch { condition, when_true, when_false }
EnumMatch { place, arms }
WeakUpgradeBranch { weak, success, expired, cleanup }
Trap { identity, cleanup }
```

`EnumMatch` has exactly one arm for every sealed variant. `WeakUpgradeBranch` contains the
indivisible increment-and-branch contract. `TrapIdentity` is closed to `BoundsV1`, `AllocationV1`,
`CapacityV1`, `RefcountV1`, and `Utf8V1`. A cleanup plan is a dense ID, authoritative span, and
ordered list. Ordinary exits use `DropPlace(PlaceId)`. Only the dedicated clone-element roles may
start with `DropVecInitializedPrefix(PlaceId)` or `DropAggregateInitializedPrefix(PlaceId)`; these
typed prefix actions cannot be treated as ordinary whole-place drops.

Calls remain direct and acyclic. Block arguments, call arguments, returns, match payloads, and weak
upgrade results use sealed value identities with exact types. A borrow cannot cross a branch,
match, weak-upgrade, loop, return, or trap edge.

## Ownership and cleanup proof

The verifier derives state along reachable CFG paths rather than trusting the order of raw
transition records. Each non-`Copy` place is exactly one of `uninitialized`, `initialized`,
`shared-borrowed(k)`, `exclusive-borrowed`, `moved`, or `dropped`, with derived partial projection
metadata retained for aggregate cleanup. It rejects reads before initialization or after
move/drop, conflicting or escaping borrows, moves or drops while borrowed, contradictory
parent/child state, and double ownership effects.

Addressable storage and ownership are separate proofs. Copy parameters, locals, and temporaries
may have canonical places so projections and storage identity remain exact, but Copy values are
excluded from the non-Copy value-to-owner map and never create a pending-drop obligation. Every
non-Copy value has exactly one canonical owner place; duplicate roots and ownerless values fail
closed.

Every reachable join requires exact state equality for every live place. Loop backedges restore
the exact header state. Passing or returning a non-`Copy` value transfers its one pending drop
obligation. Renaming a partially initialized or partially moved aggregate owner requires exact,
bidirectional equality of the source and destination relative projection paths. Verification
rejects a missing or extra path before changing ownership state, so no partial mask or active
variant metadata can be silently discarded. This rule admits a whole-root `InitializePlace`
rename from a partial temporary into an exact-topology local; initializing a projection from a
partial owner remains rejected because that route has no subtree-mask transfer contract.
One final `Return` may also transfer an exact-topology partial temporary whose sealed root is a
Struct or FixedArray containing only acyclic Struct/FixedArray, Bool, i32, and String nodes. The
return removes that owner before cleanup, so every survivor still appears exactly once in reverse
order while the returned root does not. Missing, extra, or wrong projection paths, unsupported
Enum/Vec/Shared/Weak graphs, a prior drop, and cleanup that includes the returned owner fail closed.
The generic consumed-value path remains initialized-only, so `DirectCall` and other operands do not
inherit this return-specific admission.
Root `ReplacePlace` may likewise consume an exact-topology partial temporary only when the sealed
source and fully initialized destination are same-type acyclic Struct/FixedArray graphs containing
Bool, i32, or String leaves. Verification authenticates complete relative paths on both roots,
derives the old destination's recursive drop action from its pre-state, then migrates the exact mask
into the destination. Matching-but-incomplete, missing, extra, or wrong topology, a partial
destination, partial Enum/Vec/Shared/Weak roots, and prior-drop or cleanup misuse fail closed.
CFG block-parameter edges remain initialized-only: no partial owner can cross an edge until a later
profile defines and verifies edge mask transport and join equality.
Moving one canonical static Struct-field or fixed-array-constant subobject marks the selected
projection and every declared descendant moved beneath the still-pending enclosing root. The move
result receives one distinct initialized owner, so initializing one exact same-type direct local
renames that complete owner without transferring a partial mask. Derived cleanup therefore excludes
the entire moved subtree from the parent and drops the new local exactly once. Projection typing,
base/selector identity, duplicate or overlapping consumption, and cleanup claims are independently
verified. A separate exact exception admits one complete Struct/FixedArray `EnumPayload` move in a
private three-block single-variant `EnumMatch`: the refined arm immediately initializes a same-type
direct local, drops the Enum root, and jumps with no owner arguments to the sole final local return.
The source payload has complete static topology, the move result has one temporary owner and one
use, the return cleanup has zero actions, and a second site or alternate CFG fails closed. This does
not admit broader Enum-payload, dynamic, or Vec-element moves, nor broader projected
assignment/clone, call, direct-return, CFG-edge, or public transfer contexts. At most one such
aggregate-subobject move site is admitted per function.
Separately, at most one non-root `ClonePlace` may read an initialized non-Copy Struct or FixedArray
through a canonical `StructField`/`FixedArrayConstant` path in a private one-block function. It must
produce one unique temporary and be immediately followed by same-type `InitializePlace` into a
root local; the result has exactly one use. The source and its enclosing partial-state mask are
unchanged. Its prepare cleanup contains the pre-existing live roots, while its
`AggregateCloneElementFailure` cleanup begins with the temporary's initialized-prefix action and
then those roots in reverse order. Recursive fallible leaves come from sealed layout, so no
descendant place expansion is required. Enum-payload, public, CFG, alternate-use, direct-return,
and second projected-clone contexts fail closed.
At most one combined projected aggregate `ReplacePlace` site is separately admitted in a private
one-block function. Its target is a canonical `StructField` or `FixedArrayConstant` path rooted in
a local. The immediately preceding producer must either move or explicitly clone one distinct
fully initialized exact same-type supported non-Copy Struct or FixedArray whole local, produce one
unique temporary, and have the replacement as its only use. Move consumes the source. Root clone
retains it and carries independently verified prepare plus initialized-prefix cleanup that retain
both source and destination on failure. Ownership flow recursively drops the exact old target at
commit, leaves the destination root pending, and preserves all sibling masks. Fresh,
projected/partial sources, Enum/Vec/dynamic targets, overlap, alternate ordering/use, public or CFG
contexts, and a second move-or-clone site fail closed.
Replacement evaluation and allocation happen before
`ReplacePlace`; the instruction
itself commits by dropping the old destination and installing the already prepared value. Like an
explicit `DropPlace`, its verified view derives a planless recursive old-value action from the
pre-commit state. That action's traversal root may be a canonical owner or an exact static
projection. Projected replacement replay transfers only the prepared source subtree's state and
active enum variants, preserving the enclosing owner and every sibling mask. A trap during
preparation occurs at that producer's cleanup site and leaves the old destination live.

Raw cleanup plans are claims, not authority. Each plan must be referenced by exactly one program
point. Verification binds that point to one closed `VerifiedCleanupRole`: `PrepareFailure`,
`VecCloneElementFailure`, `AggregateCloneElementFailure`, `CallTrap`, `Return`, or
`ControlledTrap`. Orphan plans, reused plans, and a plan attached to the
wrong operation or exit role fail closed. `VerifiedCleanupPlan::site()` exposes only the sealed
block, optional instruction index, and role for that one binding.

The verifier independently derives the exact actions required at every normal and controlled-trap
exit and compares place identity, action, and order.
Every live owned value is dropped exactly once in reverse successful-completion order. Struct
fields use reverse declaration order, only the active enum payload is dropped, arrays and vectors
drop the initialized prefix from its highest element to zero, and container storage is released
after its contents. Copy, uninitialized, moved, and already dropped places have no drop action.
Missing, duplicate, extra, reordered, unreachable, or wrong-exit cleanup fails verification.

Every verified instruction or terminator that names cleanup exposes `derived_drop_actions()` for
that exact program point. An explicit `DropPlace` and an infallible `ReplacePlace` commit also
expose their derived recursive old-value state from the instruction's pre-commit point even though
neither names a cleanup plan. Each returned
`VerifiedDropAction` seals:

- `root`, the exact recursive traversal root released by the action, which may be a canonical
  owner or a static owned subobject;
- `moved_projections()`, the descendants already moved or dropped and therefore excluded;
- `initialized_projections()`, the descendants whose initialization completed and remain live; and
- `active_variant()`, the statically refined active enum variant when one is required or known.

A fully initialized enum without static refinement remains self-describing through its stored
runtime tag. Partial enum cleanup is admitted only when the verifier seals an exact active variant.

These values are derived inside the verifier from the retained verified CFG and ownership state;
they are not copied from the raw cleanup list. A backend combines this sealed state with the
retained verified layout views to perform recursive field, element, payload, and container cleanup.
It never replays raw ownership transitions or invents a partial-initialization mask.

Controlled traps stop later evaluation, execute the verified cleanup plan, and retain the original
language-trap identity. Cleanup itself is an infallible logical effect. This boundary does not
implement target cleanup or a runtime.

## Runtime contract identity only

`RuntimeContractIdentity` is a closed enum whose only admitted value is `OwnershipRuntimeV1`.
It pins the declaration name `zryna-ownership-runtime-v1` and lets later authorities reject a
program claiming another contract. The separate issue #80 authority now seals the exact runtime
declarations, authenticated layout-derived records, header evidence, and pure transitions. This IR
does not define target helper symbols, status-number encodings, imports, allocator
capabilities, runtime object bytes, or an implementation of allocate, release, String, Vec,
Shared, or Weak operations. Those later authorities must validate their own raw claims and bind
back to this retained identity.

## Resource limits

Existing `ControlFlowV1` module, function, parameter, block, value, edge, call-depth, and loop-depth
limits continue to bound the corresponding independent M3 arenas. Fully instantiated types,
fixed-array length, layout depth, fields and variants, and layout dependency edges retain the
limits already enforced by the supplied sealed `zryna-layout` authorities; Data IR does not assign
or recount those layout-owned resources.

The additional IR-owned limits are:

| Resource | Maximum |
| --- | ---: |
| Nominal declarations per program | 4,096 |
| Aggregate-construction operands per program | 262,144 |
| Ownership places per function | 65,536 |
| Ownership state transitions per function | 262,144 |
| Simultaneously active borrows per function | 16,384 |
| Cleanup plans per function | 65,536 |
| Inserted cleanup/drop actions per function | 262,144 |
| Retained diagnostics, including the terminal diagnostic | 256 |

Resource preflight uses checked aggregate counters and runs before proportional graph, layout,
ownership, or cleanup work. Exact-limit and first-extra tests freeze every new limit. One resource
failure is terminal and prevents later verification phases from constructing partial authority.

## Stable diagnostics

`ZRYNA-I3xxx` is reserved for this verifier. The exact implemented allocation is:

| Code | Rejected claim |
| --- | --- |
| `ZRYNA-I3001` | authority tuple, source, entry, or module mismatch |
| `ZRYNA-I3002` | malformed, sparse, duplicate, foreign, or orphan structural identity |
| `ZRYNA-I3003` | layout target, universe, fingerprint, `TypeId`, or stored-type mismatch |
| `ZRYNA-I3004` | invalid, foreign, or cross-module source span |
| `ZRYNA-I3005` | invalid value, operation, operand, or result type |
| `ZRYNA-I3006` | invalid place, projection, overlap, ordinal, or index |
| `ZRYNA-I3007` | invalid CFG edge, reachability, dominance, loop, or exit claim |
| `ZRYNA-I3008` | orphan ownership claim or invalid ownership dominance/owner relation |
| `ZRYNA-I3009` | scalar ABI, export, direct-call, argument, or result mismatch |
| `ZRYNA-I3010` | invalid ownership-state transition, move, initialization, or replacement |
| `ZRYNA-I3011` | invalid, conflicting, or escaping borrow |
| `ZRYNA-I3012` | missing, duplicate, extra, or incorrectly ordered cleanup/drop action |
| `ZRYNA-I3013` | invalid partial aggregate initialization or initialized-prefix cleanup |
| `ZRYNA-I3014` | runtime identity, trap, or weak-upgrade contract mismatch |
| `ZRYNA-I3201` | deterministic resource budget exhausted |
| `ZRYNA-I3202` | diagnostic budget exhausted or impossible bounded construction failed |

Diagnostics are deterministic and source-located when an authoritative span exists. At most 256
are retained, including one final `ZRYNA-I3202` when additional details are omitted. Any diagnostic
prevents construction of the verified program.

## Opaque verified views and non-goals

Verified program, module, function, block, value, place, projection, instruction, terminator,
borrow, transition, cleanup, and identity fields are private. Public methods expose immutable
views, opaque identities, exact-size iterators, both sealed layout snapshots, and the sealed scalar
ABI. Role-preserving payload views include function and block signatures, complete place-root or
projection kinds, literal values, direct callees and typed value-or-borrow call arguments, enum
construction variants, borrow identities and access, exhaustive enum arm-to-edge mappings, exact
trap identities, ordered true/false branch edges, ordered weak-upgrade success/expired edges, and
the per-site derived drop metadata described above. They do not expose the retained raw program or
offer a raw-to-verified identity conversion. Compile-fail tests protect construction, raw recovery,
and mutation boundaries.

This component performs no syntax lowering, semantic analysis, target lowering, optimization,
allocation, runtime execution, artifact emission, linking, publication, manifest production, or
CLI selection. Pair is not executable here. M1 and M2 remain the only public compiler profiles.
