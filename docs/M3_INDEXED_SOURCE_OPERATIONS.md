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
| Copy alias read / exclusive write | `BorrowRead` / `BorrowWrite` |
| Owned alias observation / exclusive replacement | explicit structural clone / `BorrowReplace` |

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

Owned elements cannot be implicitly copied or moved out. An explicit clone retains the source,
produces one distinct owner and uses the same recursive initialized-prefix cleanup as ordinary
generic clone. Replacement retains the container's initialized range and creates no hole.

## Lexical identity and cleanup

Aliases are const source bindings, not owned locals. Their table records the actual issued
borrow identity after index preparation, exact referent type and access mode. Index preparation
may itself consume temporary borrow identities; the alias must not retain a guessed earlier ID.

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

Focused source fixtures are split into ordinary-array, explicit-indexed positive, rejection,
region and sibling-container modules. They authenticate syntax before semantic checks and
inspect sealed views, failure cleanup and deterministic diagnostics. Private resource controls
label their injected counters separately from successful authenticated source proofs.

Final acceptance still requires the complete declared issue matrix, applicable hostile IR and
resource proofs, documentation/contracts, preflight, M0/M2 and required Linux/Windows checks.
Passing focused examples alone does not close any of these issues. Shared/Weak source producers,
broader ownership CFG, backend execution and public activation retain their existing separate
requirements; this document does not waive their interaction obligations.

## Outstanding full-issue requirements

This batch is not yet a closure claim for #255, #256 or #274. The current source adapter accepts
the supported non-handle graph. Shared/Weak referents still require the separately verified
handle source stages; raw opaque-slot tests cannot discharge that source requirement.
Ordinary indexing of a fresh returned array or a dynamically selected nested container still
needs a reviewed base-value/addressable-container adapter. A hidden intermediate clone is not
equivalent because it changes allocation and failure effects. These limitations must remain
visible during acceptance reconciliation rather than be treated as completed generic support.
