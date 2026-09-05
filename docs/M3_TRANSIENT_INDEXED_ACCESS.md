# Transient ordinary indexed access

This dependent verifier/source adapter extends the checked element authority in
`M3_INDEXED_BORROW_AUTHORITY.md`. It does not introduce an element move, an array
hole, handle transitions, or a second clone/drop engine.

`BeginIndexedAccess` uses the same exact container layout, signed i32 bounds
check, prepare-failure cleanup, access mode, and whole-container conflict region
as `BeginIndexedBorrow`. The latter remains the lexical operation. Only the
transient operation and its projected descendants may parent
`ProjectIndexedBorrow`; formal and lexical aliases cannot be consumed this way.

Projection requires an exact FixedArray referent and a fresh dense child borrow
identity. Its bounds failure retains the parent and discharges that parent during
the ordinary failure unwind. Success atomically retires the parent and creates
the child, preserving access mode and the original conflict region. The active
borrow count therefore does not increase at projection. A retired parent cannot
be read, passed, projected again, or ended. The sealed projection view identifies
the parent, child, original region, exact element, current array length, index,
cleanup and BoundsV1 trap identity without inventing a dynamic PlaceId.

Ordinary chained source access evaluates the base once, then each index once in
order, with each bounds check preceding evaluation of the next index. Replacement
checks the complete target chain before preparing the RHS once. Copy reads use
BorrowRead; explicit owned clones use the existing generic clone frontier;
replacement uses BorrowWrite or BorrowReplace. The final authority ends before
subsequent use of the containing owner. Exclusive calls and mutation invalidate
refinements through the retained original conflict region.

Fresh complete array call/construction results used for observation are evaluated
into genuine temporary storage. Copy storage has an explicit InitializePlace
effect, charged as one place and one transition, and is not registered as an owned
cleanup root. Non-Copy storage remains pending throughout index/bounds/clone
preparation and is dropped after the final access ends, while an independently
cloned result remains pending. A fresh expression is not granted mutation rights;
assignment still requires a mutable initialized binding-derived target.

These are verified static plans and opaque execution descriptors, not a claim of
runtime execution coverage. Source and independent raw tests must authenticate
their own syntax/layout authorities and pass the mandatory verifier. Shared/Weak
source construction and transition work remains separate.
