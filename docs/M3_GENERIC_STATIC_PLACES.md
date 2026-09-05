# Complete generic static subobjects

The #278 composition adapters `GenericMoveFromPlace { place }` and
`GenericReplacePlace { place, value }` operate on complete non-Copy subobjects.
They do not relax the reviewed context restrictions of `MoveFromPlace` or
`ReplacePlace`. Copy observations and writes retain their existing operations.

## Admission and state

The target is a non-root path containing only exact Struct fields and constant
FixedArray indices. Its sealed type must pass the shared non-handle generic
graph classifier. Enum values and Vec values may be complete referents; an
Enum payload path or dynamically indexed Vec element is not this authority.
Dynamic elements use the separate indexed-borrow operations.

Both adapters retain the existing exact type, function ownership, place shape,
active-borrow exclusion, and ownership-flow checks. They cannot transfer a
partial subobject or reinitialize a moved target. Classification is computed
once per program verification when a generic operation requires it, using the
same bounded graph classifier as generic clone preparation.

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
