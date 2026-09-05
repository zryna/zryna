# Typed opaque handle slots in the generic operation core

Issue #278 supplies non-handle composition before #261 supplies authenticated Shared/Weak source
operations. Its opaque slot boundary reuses existing typed IR, layout and ownership effects. No
new placeholder value, unchecked producer, handle opcode, reference-count model or alternate
verifier is needed or introduced.

## Existing integration interfaces

The sealed layout graph already gives Shared and Weak distinct exact type identities, payload
identities, non-Copy drop kinds and runtime kinds. Generic `StructConstruct`, `EnumConstruct`,
`FixedArrayConstruct` and `VecConstruct` match their operands against these exact child types,
including handle leaves nested under other composites. They do not require a child to have been
created by a particular source lowerer. A future handle producer passes an ordinary exact typed
owned result through the same constructor preparation and ownership-transfer boundary.

An opaque slot does not authorize aliasing one owner ID twice. Each non-Copy operand transfers one
unique ownership obligation. `InitializePlace`, `MoveFromPlace` and `ReplacePlace` retain the same
exact type and pending-owner rules for composites containing handles. Vec allocation failure owns
all prepared children and existing roots; commit consumes children only after preparation succeeds.
Replacement drops the old complete composite before installing the complete replacement.

Existing `SharedConstruct`, `SharedClone`, `WeakDowngrade` and `WeakClone` are the operation hooks,
not implementations of a second count authority. Their exact typed inputs/results and preparation
cleanup bind the issuing owned value. Count meanings, provenance, last-strong payload release and
implicit weak ownership remain the separate Shared/Weak contract and #260 authority. #261 must
produce these operations from authenticated source and integrate real handle-leaf clone/drop
effects. In particular, generic non-handle clone continues rejecting handle-containing graphs;
it must not pretend to clone handle payloads or silently substitute non-handle clone behavior.

## Evidence and limits

`opaque_handle_slots_fixture.rs` is independent raw-IR evidence, not source handle production.
It receives typed Shared, Weak and nested-container parameters; exercises the existing two clone
hooks; then moves distinct handle owners through Struct, active Enum, FixedArray and Vec
construction, whole-vector replacement and return. The failure plans retain the exact preceding
owners, including complete argument storage before Vec allocation. The final cleanup excludes
the transferred result and every already consumed child.

`opaque_handle_slots.rs` checks mandatory whole-program verification, opaque cleanup views,
non-Copy ownership and deterministic replay. Wrong Shared/Weak slot types, repeated owner use and
premature cleanup of an uncommitted container are independent hostile claims. These tests prove
symbolic compiler operation compatibility only: no address, count transition, runtime fault or
target drop receipt is executed or claimed.

The #261 adapter still must map authenticated handle type syntax and prepare its owned results
through the shared source transaction. Current non-handle source admission is not handle source
support. This boundary establishes reusable typed slots and effects for that adapter without
waiting for #270, and without treating an opaque declaration as completion of #261 or #83.
