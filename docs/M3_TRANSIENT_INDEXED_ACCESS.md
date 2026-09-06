# Transient indexed access and lexical finalization

This dependent verifier/source adapter extends the checked element authority in
`M3_INDEXED_BORROW_AUTHORITY.md`. It does not introduce an element move, an array
hole, handle transitions, or a second clone/drop engine.

`BeginIndexedAccess` uses the same exact container layout, signed i32 bounds
check, prepare-failure cleanup, access mode, and whole-container conflict region
as `BeginIndexedBorrow`. The latter remains the lexical operation. Only the
transient operation and its projected descendants may parent
`ProjectIndexedBorrow`; formal and lexical aliases cannot be consumed this way.

Projection requires an exact FixedArray or Vec referent and a fresh dense child
borrow identity. FixedArray bounds use the sealed array length; Vec bounds use
the selected referent's runtime length. Its bounds failure retains the parent and discharges that parent during
the ordinary failure unwind. Success atomically retires the parent and creates
the child, preserving access mode and the original conflict region. The active
borrow count therefore does not increase at projection. A retired parent cannot
be read, passed, projected again, or ended. The sealed projection view identifies
the parent, child, original region, exact element, optional fixed-array length, index,
cleanup and BoundsV1 trap identity without inventing a dynamic PlaceId.

Ordinary chained source access evaluates the base once, then each index once in
order, with each bounds check preceding evaluation of the next index. Replacement
checks the complete target chain before preparing the RHS once. Copy reads use
BorrowRead; explicit owned clones use the existing generic or handle-aware clone frontier;
replacement uses BorrowWrite or BorrowReplace. The final authority ends before
subsequent use of the containing owner. Exclusive calls and mutation invalidate
refinements through the retained original conflict region.

Fresh complete FixedArray or Vec call/construction/Match results used for observation are
evaluated into genuine temporary storage. Copy FixedArray storage has an explicit InitializePlace
effect, charged as one place and one transition, and is not registered as an owned
cleanup root. Non-Copy storage remains pending throughout index/bounds/clone
preparation and is dropped after the final access ends, while an independently
cloned result remains pending. A fresh expression is not granted mutation rights;
assignment still requires a mutable initialized binding-derived target.
An explicit clone of an available whole or indexed FixedArray may also supply
the fresh observation base: `clone(items)[i]` and `clone(items[j])[i]` evaluate
the clone source (including `j`) before `i`, once each. Copy array clones use
real initialized Copy storage; owned array clones retain both the original
source and distinct cloned temporary through the final bounds check. This reuses
the existing clone and transient-access authorities, not a new element move.
Checked descendants may alternate FixedArray and Vec referents without moving or
cloning intermediate containers. A Vec of Copy elements is still an owned allocation container;
it remains pending during index/bounds failure and is dropped after the final end.

These are verified static plans and opaque execution descriptors, not a claim of
runtime execution coverage. Source and independent raw tests must authenticate
their own syntax/layout authorities and pass the mandatory verifier. Shared/Weak
source construction and count transitions reuse the separate #260/#261 authorities;
the adapter does not define alternative handle semantics.
No target execution, backend implementation or public profile is activated here.

## Acyclic internal continuations

The independent IR authority permits an unfinished transient indexed operation
to cross acyclic expression continuations without ending and reborrowing. The
per-function dense borrow index caches incoming identities once. Every predecessor
must supply exactly the same live identities in issuance order; each keeps its
original region, exact referent and access mode. Branches may perform arm-local
operations but must restore that exact state before joining. Borrowed owners
cannot be renamed through by-value edge arguments. No borrowed edge argument or
phi-like authority is introduced.

`VerifiedTerminator::continued_indexed_accesses` seals these identities on every
ordinary success edge, including both success and expired Weak upgrade outcomes.
Instruction failure views include incoming identities; terminator
`failure_ended_borrows` describes reverse-issued unwind on controlled trap or Weak
upgrade count failure. Bounds failure retains the current parent until this
unwind; successful projection retires it exactly once. Owner cleanup follows
authority discharge. Return and loop backedges reject unfinished operations with
I3011. Lexical begins and `BindIndexedBorrow` results still cannot cross edges.

The cache admits at most 262,144 total incoming identities per function, with
checked accounting before allocation. Existing active-borrow, transition, place,
edge and cleanup budgets still apply, including inherited active identities.
`transient_indexed_resources_cache_exact_first_extra_and_recovery` authenticates
the exact cache boundary and first extra identity; a separate accounting test
covers machine-integer overflow without pretending it is source execution.
`transient_indexed_edges` tests exact joins, owned replacement, failure unwind,
lexical exclusion, return/backedge rejection and owner-transfer exclusion.
`transient_indexed_upgrade` tests both ordinary upgrade outcomes and count failure.

The #279 source adapter stages named/static/fresh-container access through begin/project,
expression continuation and finish. Later-index Match evaluates after preceding
bounds; replacement RHS Match evaluates after the complete bounds chain. Exact
Copy and owned SSA handoffs are consumed once without cloning or moving a container
to resume it. Owned RHS results retain their pending owner until BorrowReplace.
The existing named/static first-checked-index route stays unchanged. A fresh FixedArray/Vec Match
base is evaluated once before its index; Copy arrays receive genuine initialized temporary storage,
while owned arrays and Vecs retain the exact joined owner through bounds and observation and drop it
after the final EndBorrow. Replacement is intentionally unavailable because a Match result is not a
mutable initialized source place; callers must first bind it to one. Backend execution is not
claimed. `continued_indexed_source` authenticates named/static and fresh Array/Vec Copy/owned cases,
a fallible index call before bounds, exact continuation identity, fresh-base and RHS failure cleanup,
and deterministic rejection/recovery. Fresh Match storage starts traversal at index zero even
when a FixedArray index is a statically valid literal. It cannot reuse the static-place prefix
optimization for named containers. `fresh_indexed_literal_source` authenticates Copy reads and
owned clones for `[0]`, `[0][0]`, and `[0][indexValue(...)]`, checking joined-base storage,
exact index/authority order, failure retention and final end before the owned temporary's release.
Fresh assignment remains rejected by canonical syntax as a non-place, with deterministic rejection
and valid observation recovery. The structured scratch resource matrix includes all eighteen indexed
shapes across values, places, transitions, cleanup actions and plans, with exact, first-extra and
checked-overflow cases and complete state rollback. Together these form the bounded compile-time
#279 infrastructure closure candidate without extending lexical authority or target execution.

## Lexical finalization

`BindIndexedBorrow` is an infallible, effect-only transfer from one active
transient access to a fresh dense lexical borrow identity. It copies the exact
referent, access mode and original conflict region, but clears transient
projectability. It creates no owner, cleanup plan or bounds check and does not
increase the active count. An existing lexical/formal authority, a retired
parent, or an already bound child cannot parent this transfer. Subsequent
projection or rebinding of the lexical child is rejected.
The per-function borrow index retains the child's exact definition and region,
but no transient origin. The sealed binding view identifies this transfer without
claiming a bounds check, allocation or ownership result.

Nested lexical borrowing from a named complete FixedArray or Vec root evaluates
each dynamic index once in source order. Further nesting traverses exact FixedArray
or Vec referents. A Vec reached through an entirely static prefix can itself be
the first checked container. The maximal in-range
static prefix remains the conflict region;
dynamic descendants conservatively retain that region. After the complete
transient chain succeeds, binding creates the persistent alias. The source
planner reserves the final EndBorrow before preparing the initializer and charges
every bounds/projection/binding effect through its ordinary resource replay.
Bounds failures before binding unwind the active transient parent; later clone,
replacement-RHS or call failures unwind the lexical child. Calls and mutations
use the existing exact type/access, owner exclusion and region-refinement rules.
This does not admit borrowing from a fresh temporary or consuming another alias.

The [indexed source contract](M3_INDEXED_SOURCE_OPERATIONS.md) records the operation
matrix and call integration. Named source, raw-hostile and resource evidence is
located separately in the [composition evidence](M3_OWNERSHIP_COMPOSITION_EVIDENCE.md#ordinary-and-lexical-indexed-source-integration-255256274).
