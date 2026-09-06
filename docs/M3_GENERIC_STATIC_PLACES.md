# Complete generic static subobjects

The #278 composition adapters `GenericMoveFromPlace { place }` and
`GenericReplacePlace { place, value }` operate on complete non-Copy subobjects.
They do not relax the reviewed context restrictions of `MoveFromPlace` or
`ReplacePlace`. Copy observations and writes retain their existing operations.

## Admission and state

The target is a non-root path containing only exact Struct fields and constant
FixedArray indices. Its sealed type must pass complete-transfer eligibility:
either the cached non-handle graph classification or the cached handle-containing
graph classification, with their existing layout restrictions. Enum values and Vec values may be complete referents; an
Enum payload path or dynamically indexed Vec element is not this authority.
Dynamic elements use the separate indexed-borrow operations.

Both adapters retain the existing exact type, function ownership, place shape,
active-borrow exclusion, and ownership-flow checks. They cannot transfer a
partial subobject or reinitialize a moved target. Classification is computed
once per program verification when a generic operation requires it, using the
same bounded graph classifications as clone preparation. Transfer-only programs
also compute the handle-containing classification; no clone site is required.
`GenericClonePlace` and `GenericCloneBorrow` remain handle-excluding. Explicit
handle-containing clones use the separate handle-aware clone authority.

## Move and replacement

A move produces one distinct exact owned result. The source must be complete
and its enclosing root must remain pending. Its entire materialized subtree is
marked moved, including Enum descendants, and its ancestors become partial.
The existing sealed drop mask excludes that whole subtree from the retained
root's eventual cleanup. The result becomes pending once, after the retained
root; matching relative projection paths preserve known Enum refinements.
Unrepresented refinements remain unknown. Complete transfer does not require
unfolding every field of a sealed type into places.

A replacement requires an already complete target and an independently
prepared complete exact owned value. It reuses the existing projection
consumption transition: the enclosing root remains pending, the prepared
owner is consumed, and the target receives its state and matching refinements.
Same-root aliasing and consuming an owner with live descendant borrows remain
invalid. Replacement does not repair a partial ancestor or permit holes.

`VerifiedInstruction::derived_drop_actions()` exposes the old target's sealed
drop action before replacement. Old-value destruction is the existing
infallible commit operation; these adapters add no allocation, failure cleanup
role, runtime implementation, or handle transition. Normal verification and
sealed-state replay use the same transfer routines.

## Handle-containing static transfer evidence

Issue #321 admits authenticated private `Struct { value: Vec<Shared<String>> }`
field moves, replacement, repeated replacement, self-clone replacement and return
through these existing adapters. Source ordering and cleanup are unchanged:
the RHS finishes before the old target is dropped, including every fallible
handle-aware clone step. No handle count is changed by the transfer itself.

`generic_static_source` and `handle_static_source` cover exact parent masks,
old-target retention, wrong lowerable RHS type, repeated move, moved target,
self-consumption and deterministic diagnostics. Independent
`generic_static_transfer::handles` raw-IR tests cover Shared, Weak and recursive
handle graphs, exact owner/result types, borrow conflicts, consumed RHS reuse,
moved targets, cleanup replay and continued generic-clone exclusion.
`generic_static_resources` uses authenticated fixture envelopes with controlled
held cleanup/transition credits for exact, first-extra, checked-overflow and
same-lowerer recovery evidence; it does not claim a full maximal source program.

Dynamic element move-out, hole repair, new public entry shapes and target-runtime
execution remain outside this adapter. The existing handle-aware clone and
symbolic shared/weak conformance contracts are not runtime execution receipts.

Issue #323 additionally authenticates both a Struct wrapper and a two-element FixedArray whose
static subobject is the finite `Node` Enum recursively linked through `Vec<Node>`. Exact source and
verified-IR evidence binds projected move results and parent masks, prepared clone owners,
old-target replacement cleanup and returned owners. Recursive-specific hostile IR rejects wrong
paths, owner reuse and cleanup forgery without claiming by-value recursion or target traversal.
