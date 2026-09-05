# Generic non-handle structural clone core

This bounded Issue #278 operation implements the reusable clone part of
[C1–C4](M3_OWNERSHIP_COMPOSITION.md). It does not close #278, provide #255/#256 indexed source
borrows, implement handle operations, or activate a runtime, backend or public profile.
Existing `ClonePlace`, String and exact Vec clone operations and views remain unchanged.
Generic function preparation uses the canonical non-handle aggregate clone, including legacy-shaped
subtrees inside a mixed context. Standalone legacy function routes remain separate.

## Exact operation and ownership

Raw `GenericClonePlace { place, cleanup, prefix_cleanup }` claims one complete initialized
source root or exact static subobject and one exact same-type non-Copy result. Mandatory IR verification requires a distinct
temporary owner for that result. The source is read without consumption; shared lexical reads are
compatible, overlapping exclusive authority is not. Partial, moved, foreign, wrong-type
or aliased source/result claims fail closed through the existing place/type/ownership diagnostics.
For a static subobject the enclosing owner remains live; a disjoint moved sibling does not make
the complete selected subobject unavailable. No dynamic index is converted into a static place.

`GenericCloneBorrow { borrow, cleanup, prefix_cleanup }` adapts an existing exact active lexical
or call-frame referent to the same frontier. Ordinary owned Vec observation uses this adapter
after its signed bounds check; see [generic Vec operations](M3_GENERIC_VEC_OPERATIONS.md).

The accepted graph contains bool/i32 Copy leaves, String, arbitrary nested Struct/Enum/FixedArray
and positive-stride Vec nodes. Every Enum variant participates in capability validation, even if
not currently active. Zero-length fixed arrays are allowed; zero-size Vec elements are not.
Shared/Weak anywhere in the graph are rejected pending the separately owned handle integration.
No checked count transition or opaque handle placeholder is treated as implemented clone.

Capability classification is computed once per verification, only when the program contains this
operation. Reverse adjacency propagates forbidden descendants to their ancestors; every type is
enqueued at most once and every edge examined once. Instruction checks then use constant-time
classification lookup. Its work is bounded by the existing verified type/member/edge arenas,
without charging a new budget or repeating a graph walk per clone. The sealed consumer's optional
reachable-type enumeration is iterative and visits each identity once per request.
Legal Vec indirection cycles are retained as graph
references, never recursively expanded. The layout authority remains responsible for rejecting
by-value cycles. Runtime traversal of a finite constructed value is distinct from this static
graph traversal.

## Preparation, frontier and failure

Both cleanup identities are mandatory and unique to their exact instruction and role.
`cleanup` has the existing `PrepareFailure` role: before any destination resource is acquired,
reverse-clean every pre-existing pending owner, retaining the source among those owners.
`prefix_cleanup` has `GenericClonePrefixFailure` and must contain exactly:

1. `DropGenericCloneInitializedPrefix` of this instruction's distinct result owner;
2. every pre-existing pending root in reverse completion order, unchanged from preparation.

No failed result is published as a fully initialized owner. A failure after acquiring destination
storage uses the prefix role even if no child has completed. The original allocation/capacity
trap is preserved; cleanup does not allocate or invoke source code.

The recursive frontier is one operation-bound symbolic protocol. Struct children complete in
declaration order, FixedArray and Vec children in ascending index order. Enum traversal selects
only the runtime-active payload; no statically known root variant is required. Every active frame
tracks its acquired storage, completed children and at most one partially prepared child.
Unwinding first cleans the partial child recursively, then completed children in reverse, then
releases that frame's acquired Vec storage. A failing String clone contributes no completed String.
Completed nested values use their runtime active tags and initialized lengths for their ordinary
recursive drop. A frame does not release storage before any owned descendant. No source leaf or
uninitialized destination leaf belongs to the destination frontier.

The result becomes a distinct fully initialized owner only after the complete traversal succeeds.
Exact source-place enum refinement, when known, propagates to the successful result; borrow-sourced
results start with unknown static variant. Failure cleanup does
not fabricate destination projection masks or claim that a partial destination is wholly active.

## Sealed consumers and accounting

`VerifiedInstruction::generic_clone()` supplies an opaque `VerifiedGenericClone` with exact source,
destination, result, type and both cleanup identities.
`source()` explicitly distinguishes `VerifiedGenericCloneSource::Place` from `Borrow`; a borrowed
element never masquerades as its enclosing place. `frontier()` supplies the operation-bound
`VerifiedGenericCloneFrontier`; `types()` exposes only the reachable sealed layout records in
canonical identity order. Their field/type/variant identities and fixed lengths define traversal.
`generic_clone_prefix_failure_drop_actions()` exposes the typed
`GenericCloneInitializedPrefix` action followed by the verified pending-root actions. The legacy
root-only cleanup compatibility iterator must not be interpreted as a whole-value destination drop.

These views prove a static symbolic obligation, not runtime enum tags, allocation addresses,
frontier progress, fault injection or one-time execution receipts. Target execution and dynamic
frontier validation remain later consumers' obligations; no raw progress claim bypasses IR
verification. The generic operation is not a second ownership verifier or a borrow-specific clone
authority.

The operation uses the existing value, temporary-place and ownership-transition accounting.
Preparation and prefix each reserve one site; prefix accounts one additional typed action before
the retained roots. Existing plan uniqueness, drop-action limits, span authentication, diagnostic
families, authority checks and full ownership replay remain mandatory. No resource limit or
diagnostic number changes in this slice.
