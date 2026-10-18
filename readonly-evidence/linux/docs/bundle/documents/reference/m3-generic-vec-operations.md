# Ordinary generic Vec operation authority

Issue #278 composes ordinary non-handle Vec observations and replacement from the existing
[indexed authority](M3_INDEXED_BORROW_AUTHORITY.md) and
[canonical structural clone core](M3_GENERIC_CLONE_CORE.md). This adds no element move-out,
container hole, independent dynamic `PlaceId`, runtime execution or public profile. Explicit
source `Borrow`/`BorrowMut` Vec elements are implemented by the shared #255/#256 lexical producer,
documented in the [indexed source contract](M3_INDEXED_SOURCE_OPERATIONS.md). Its authenticated
evidence is separate from the ordinary transient operation tests described here; neither enables
target execution or public activation.

## Exact read roles

Ordinary Copy observations retain `VecIndexCopy`, including its signed bounds and initialized
container checks. Owned observation requires explicit clone; it never silently moves or copies
an element. The producer evaluates the container and then its index exactly once, establishes
`BeginIndexedBorrow` shared access, emits `GenericCloneBorrow`, then ends the transient borrow.
The begin checks signed index against runtime logical length, not capacity, before element access
or clone allocation. Out-of-bounds failure stops later work and uses the pre-begin cleanup.

Raw `GenericCloneBorrow { borrow, cleanup, prefix_cleanup }` is an operand adapter to the same
canonical clone, not another recursive operation authority. Mandatory verification derives the
referent from an existing lexical or call-frame borrow definition, requires that authority to be
active, and requires the exact same-type non-Copy result with a distinct owner. Both shared and
exclusive read authority are usable; this does not create a reborrow or change access mode.
Inactive, ended, foreign, unavailable or wrong-type claims fail through existing diagnostics.
Copy referents still use `BorrowRead`; borrow authorities cannot escape into stored fields or returns.

All non-handle String/Struct/Enum/FixedArray/positive-stride Vec graphs supported by the canonical
classifier are supported here, including permitted finite values through container indirection.
The existing per-verification amortized classifier sees both clone operand forms. There is no
second graph traversal policy, budget, handle transition or borrowed-element prefix role.
The subsequent #261 handle-aware adapter also admits direct Shared/Weak and handle-containing
referents through `HandleAwareCloneBorrow`, preserving the count-only handle leaf semantics.
It does not widen `GenericCloneBorrow` to handles. Lexical alias and formal-borrow replacement
reuse `BorrowReplace`; the #255/#256 matrices cover exact source type, bounds, owner exclusion,
failure retention and restoration for both arrays and Vecs.

Issue #323 binds that recursive classification to an authenticated nonempty source value rather
than inferring it from nested `Vec<Vec<String>>` evidence. Its `Vec<Node>` contains a leaf of the
recursive Enum, and prepared push receives an explicit structural clone whose destination-prefix
failure first releases that partial result before the unchanged Vec, clone source and earlier root.
Exact, first-extra and checked-overflow cleanup controls preserve pre-operation state and replay the
same diagnostic, then a same-lowerer retry binds the recovered prepared owner and push-failure
order. A separate authenticated source proves indexed clone observation and exact replacement of
the recursive `Node` referent. No element move-out, persistent borrow or runtime recursion
guarantee is added.

## Clone cleanup and sealed views

The existing `PrepareFailure` and `GenericClonePrefixFailure` sites remain separate and mandatory.
The latter begins with `DropGenericCloneInitializedPrefix` of the exact new result owner, followed
by every pre-existing pending root in reverse completion order. The container remains among those
owners; a call-frame borrowed referent is not an owned callee input and is never added to callee
drop cleanup. Both failures discharge active call-frame and lexical borrows in the order exposed
by `failure_ended_borrows()`, then perform the appropriate cleanup without replacing the original
trap. Successful clone leaves the source unchanged and publishes only its distinct completed result.

`VerifiedGenericClone::source()` now returns the explicit
`VerifiedGenericCloneSource::{Place, Borrow}` operand role. `generic_clone()` and every destination,
result, type, cleanup and frontier accessor are otherwise shared by both raw forms. A borrowed
referent never masquerades as its container place. Borrow-sourced Enum results start with unknown
static active variant: a container's variant or some constant-index sibling cannot authenticate
the selected runtime payload. The common frontier uses runtime tags and lengths as already
specified; its graph is a static symbolic obligation, not a progress or execution receipt.

## Replacement ordering and exclusions

For ordinary replacement the producer resolves the container, evaluates the index once, and
establishes exclusive `BeginIndexedBorrow` before preparing the RHS. Bounds failure therefore
precedes RHS evaluation. Complete RHS preparation precedes `BorrowWrite` for Copy or
`BorrowReplace` for non-Copy, then `EndBorrow`. Preparation failure retains the old element and
container plus previously completed RHS owners for reverse cleanup. Replacement commits one exact
old-referent drop and installs the prepared value without changing the container's ownership.

While exclusive access is active, overlapping owner reads, mutation, movement and consumption of
the container or borrowed descendants are excluded, including accesses hidden in RHS calls and
constructors. Distinct containers may still prepare values. RHS ownership must be complete and
unborrowed at commit; no implicit clone repairs self-consumption. Static enum refinements affected
by replacement or exclusive calls are invalidated by the existing indexed authority, including
callee mutation before trap cleanup.

## Resources and boundaries

Push resolves a mutable complete Vec, prepares its exact element once, then rechecks that the
RHS did not consume or invalidate the target. Its growth cleanup retains both the unchanged
container and the completed argument; only successful `VecPush` consumes the owned argument.
Copy elements add no owned cleanup. A same-container indexed clone may complete and end its
shared access before growth; an overlapping still-active borrow prevents mutation. Existing
capacity, layout and overflow authority is reused rather than replaced by a new growth policy.

Transient begins/ends consume existing borrow and transition resources. Bounds, clone prepare and
clone prefix each have one unique cleanup site; prefix accounts its additional destination action.
Replacement has no fallible commit site. Planning reserves end/commit resources before dependent
source mutation, preserves source evaluation and diagnostic ordering, and retains the existing
limits and mandatory full verifier. No element-count-sized place expansion is introduced.

Issue #322 authenticates the ordinary handle-containing Vec integration on top of this reusable
authority. Direct Shared/Weak elements and nested handle-bearing Struct, Enum, FixedArray and Vec
elements support explicit clone observation, exact replacement and prepared push. Indexed
observation uses `HandleAwareCloneBorrow`; clone operands prepared from ordinary places use either
the exact direct Shared/Weak count operation or `HandleAwareClonePlace`. Both routes preserve the
complete container, transfer only a successfully prepared replacement/push owner, and retain the
original bounds, growth and prefix-cleanup ordering.

The named `generic_vec_handle_` source tests bind all six element categories to mandatory verified
IR, the exact prepared RHS identity, bounds-before-preparation order, complete retained cleanup,
and byte-for-byte stable verified-program debug evidence when replayed against one authenticated
source authority. The enum case also binds the sealed runtime-active payload recipe to both of its
ordered handle-count leaves. Exact source diagnostics cover implicit move, moved replacement/push
operands, overlap and wrong types. Separate raw-IR hostile tests reject stale/foreign borrows,
wrong result types and missing or duplicate cleanup.

The generic Vec resource controls include direct Shared/Weak and structural handle clone routes
for replacement and ordinary-place push, plus indexed-clone push. They exercise exact,
first-extra and checked-overflow cleanup frontiers, preserve rejected state, and prove a pristine
same-lowerer retry. These remain compiler proofs: no element move-out, hole, pop, persistent source
borrow, allocator/runtime execution, backend, CLI or public profile is enabled.
