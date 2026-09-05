# Indexed borrow authority

Status: implementation contract for issue #254. This is an internal Universal IR boundary,
not source admission, target execution, or public `DataOwnershipV1` activation.

## Access and conflict identity

`BeginIndexedBorrow { definition, index, cleanup }` takes one exact initialized FixedArray or Vec
place and one already evaluated `i32` SSA index. The definition's place is the complete container
conflict region. Its element referent comes exclusively from retained verified layout, not a raw
referent claim. A dynamic element never becomes a `PlaceId`, independently movable owner, or
statically disjoint projection. Array length zero remains a valid layout.

The opaque indexed view retains the borrow identity, container identity, index value, access mode,
exact element type, array length where applicable, and bounds-failure cleanup. A consumer uses the
container's runtime logical length for Vec, never capacity. Signed negative indices and indices at
or above length produce `BoundsV1`; every index into a zero-length array fails. Verification admits
such runtime indices while rejecting invalid types, identities, dominance and ownership claims.

## Evaluation and failure

The producer evaluates the container expression before the index, once each, and emits the begin
after those values and storage are available. The verifier authenticates SSA dominance and the
container's initialized state; source-faithful evaluation remains the source producer's obligation.
Bounds checking happens at begin, before an active element authority exists. Failure prevents all
later operations, discharges lexical access, and executes exactly the pre-begin pending-owner
cleanup in reverse completion order. No target may invent a bounds rule or use an ambient trap.

Shared/shared overlap is permitted. Any overlapping exclusive authority rejects, including two
different index values, static-index/dynamic-index pairs and container-root/element pairs. Distinct
containers remain distinct, including containers reached through disjoint static sibling fields.
Enum ancestors require their exact active refinement. A partially initialized or moved container
cannot authenticate an unknown selected element; a complete container in a disjoint live part of
an otherwise partial owner retains the existing static projection rules.

## Reads, replacement and calls

`BorrowRead` and `BorrowWrite` keep their Copy-only contracts. An owned element is never read by
implicitly copying or moving it. Generic owned begin/end authority is tested with non-Copy element
layouts and is not advertised as generic clone or source-operation support.

`BorrowReplace { borrow, value }` supplies the owned mutation boundary. It requires active exclusive
access to an exact non-Copy referent and a completely prepared same-type owned RHS. Preparation
precedes the infallible replacement: a preparation failure retains both the old element and every
successfully completed RHS owner until that producer's cleanup. Commit logically drops the old
complete referent value, consumes the prepared RHS owner, and installs that value without creating
a container hole or moving the container's pending drop obligation.

The replacement view identifies the borrow, exact referent and prepared value. Its old-value drop
is a complete referent traversal using retained layout and the value's runtime enum tags/container
lengths. It is not a `DropPlace` of the conflict container. Backends must preserve that distinction.
An indexed mutation invalidates potentially affected static descendant enum refinements; differing
runtime indices never preserve a guessed unaffected element. Copy writes and exclusive calls must
perform the same conservative invalidation when they can replace an enum-bearing value.

An internal borrow parameter can receive this exact referent/access pair and consume a prepared
replacement through `BorrowReplace`, providing a real non-Copy call sink rather than an endlessly
forwarded signature. Caller overlap and callee lexical nonescape remain independently verified.
No borrow is returned, stored, captured, converted to an address, or carried across a CFG edge.

## Resource and trust boundaries

Indexed begins consume the existing instruction, active-borrow and per-site cleanup budgets.
They do not expand one dynamic access into element-count-sized place topology. Existing checked
aggregate counters, deterministic terminal resource diagnostics and rejection-before-authority
rules remain in force. Every cleanup ID belongs to exactly one site; malformed, reused, missing or
misordered plans reject. A rejected program yields no verified program and does not contaminate a
later verification call.

## Dependent work and non-goals

#255 owns dynamic fixed-array borrowing source production; #256 owns Vec-element borrowing.
#274 owns ordinary array indexing outside explicit borrow creation. These producers must consume
this exact index/bounds/conflict contract; none may substitute a container-typed element referent.
#275 retains the broader non-indexed source operation matrix. Generic clone-through-borrow,
new source syntax, raw pointers, no-alias runtime checks, allocation/runtime implementations,
backends and public activation are not supplied by this authority.

The bounded #82 source checkpoint remains distinct from #254. Verification tests of raw IR and
opaque views prove compiler authority, not execution of JavaScript, WebAssembly or native code.
