# Indexed source operation integration

Issues #255, #256 and #274 share the existing
[indexed borrow authority](M3_INDEXED_BORROW_AUTHORITY.md) and
[generic owned preparation](M3_GENERIC_FUNCTION_OPERATIONS.md).
This work concerns authenticated source-to-verified-IR behavior, not target execution,
an allocator, public profile selection or completion of M3.

## Common operation matrix

| Source operation | Required authority |
| --- | --- |
| Ordinary checked array Copy read | checked indexed begin, `BorrowRead`, end |
| Explicit clone of an owned indexed element | checked begin, canonical `GenericCloneBorrow`, end |
| Ordinary checked array replacement | checked exclusive begin, complete RHS, `BorrowWrite`/`BorrowReplace`, end |
| Lexical `Borrow<T>` / `BorrowMut<T>` | exact referent and access, persistent scoped authority |
| Nested lexical indexed borrow | transient checked chain, then infallible `BindIndexedBorrow` into a scoped alias |
| Copy alias read / exclusive write | `BorrowRead` / `BorrowWrite` |
| Owned alias observation / exclusive replacement | explicit structural clone / `BorrowReplace` |

The [transient access and binding adapter](M3_TRANSIENT_INDEXED_ACCESS.md) admits fresh exact
FixedArray/Vec call/construction results for observation and chained FixedArray/Vec indexing from an
initialized binding-derived container. Transient ordinary access uses `BeginIndexedAccess`;
each subsequent checked child uses `ProjectIndexedBorrow` without a fabricated element place or an
intermediate allocating clone. Direct Copy reads from an addressable Vec retain the existing
`VecIndexCopy` operation. This does not make lexical or formal aliases projectable.

An in-range literal fixed-array index keeps its canonical static projection and can be
disjoint from another static sibling. Other array indices and every Vec index use the complete
selected container as their conflict region, irrespective of runtime index equality. Distinct
containers reached through disjoint static fields or array projections remain disjoint.
The container's type is never substituted for the selected element's referent type.

The base is resolved before the index is evaluated. Checked begin performs bounds checking
before replacement RHS preparation; RHS evaluation completes before old-value cleanup and
replacement commit. Negative, upper-bound and zero-length accesses keep the specified runtime
`BoundsV1` boundary rather than being rejected merely for their numeric value. A successfully
prepared RHS is not evidence that a borrowed owner can be moved, replaced, grown or dropped.

For a chain, each bounds check precedes the next index evaluation. Projection failure unwinds
the parent authority; success atomically retires it and issues the child with the original
conflict region. Only the final live authority is ended. Fresh Copy results receive real
`InitializePlace` storage with no owned cleanup root; fresh owned results stay pending through
index/bounds/clone failure and are dropped after the final end. Fresh expressions are not
mutable assignment targets.

Owned elements cannot be implicitly copied or moved out. An explicit clone retains the source,
produces one distinct owner and uses the same recursive initialized-prefix cleanup as ordinary
generic clone. Replacement retains the container's initialized range and creates no hole.

## Lexical identity and cleanup

Aliases are const source bindings, not owned locals. Their table records the actual issued
borrow identity after index preparation, exact referent type and access mode. Index preparation
may itself consume temporary borrow identities; the alias must not retain a guessed earlier ID.

For a named complete container with nested checked indexing, source preparation creates a
transient chain and then uses `BindIndexedBorrow` to issue the final lexical identity.
The infallible binding preserves the exact type, access and original conflict region, retires
the transient parent and clears projectability. It produces no value, owner or cleanup plan,
and has no active-count increase. Nested referents may be FixedArray or Vec; the exact
referent layout selects a fixed length or runtime Vec length for each bounds check.
The first container may be a Vec reached through a fully static prefix; disjoint
static prefixes retain their separate regions. Existing lexical/formal aliases cannot be
consumed as transient parents. Later alias reads, replacement and calls use only the bound child.

Nested lexical blocks preserve outer bindings and authorities. Scope exit ends its own aliases
in reverse issuance order, then drops still-owned local results in reverse completion order.
Copy locals require no drop. Outer pending owners retain their existing move and partial masks.
Aliases do not escape the block or cross a return/control-flow edge.

End and owned-local drop transitions are reserved before the associated begin or local
preparation is consumed. Moved locals release unused drop credit at scope exit. Per-expression
scratch preparation validates resources and ownership before real emission; this is not a claim
that a failed later statement rolls back every previously accepted statement in a function.
Rejected functions expose no verified program.

## Call integration contract

Internal calls must preserve source parameter order while distinguishing value arguments from
borrow arguments. An alias supplies exact live access, not a fabricated value or owner.
Only by-value owned arguments transfer ownership; the caller retains lexical end responsibility.
Formal borrow parameters have distinct call-frame authority and no fabricated local place.
Source argument preparation follows the declared parameter order; the final raw call encodes
the verifier's canonical value prefix followed by its borrow suffix. Canonical encoding does
not reorder source evaluation or change which authority belongs to each exact formal.
Owned referent clone and exclusive replacement reuse the existing verifier operations.
Caller/callee cleanup, exclusive argument reuse and call nonescape remain independently checked.

## Verification and boundaries

Focused source fixtures are split into ordinary-array composition, fresh-Vec, lexical-chain,
explicit-indexed positive, rejection, region and sibling-container modules. They authenticate syntax before semantic checks and
inspect sealed views, failure cleanup and deterministic diagnostics. Private resource controls
label their injected counters separately from successful authenticated source proofs.

Final acceptance still requires the complete declared issue matrix, applicable hostile IR and
resource proofs, documentation/contracts, preflight, M0/M2 and required Linux/Windows checks.
Passing focused examples alone does not close any of these issues. Shared/Weak source producers,
broader ownership CFG, backend execution and public activation retain their existing separate
requirements; this document does not waive their interaction obligations.

## Outstanding full-issue requirements

This batch is not by itself a closure claim for #255, #256 or #274. The current source adapter
accepts the supported non-handle graph. Shared/Weak referents still require the separately
verified #260/#261 handle stages; raw opaque-slot tests cannot discharge that source requirement.
Fresh and chained ordinary FixedArray/Vec access use the explicit transient adapter above, and
nested lexical access from named containers finalizes that transient chain with `BindIndexedBorrow`.
Fresh Vec observation reuses one real owned temporary, even for Copy elements; it ends access
before dropping that Vec. Checked descendants may alternate FixedArray and Vec without
independent element ownership or intermediate clones. Arbitrary expression-base shapes,
borrowing a fresh temporary and fresh
mutation are not implied. These boundaries and the complete required gates must remain explicit
during acceptance reconciliation rather than be treated as completed generic support.
