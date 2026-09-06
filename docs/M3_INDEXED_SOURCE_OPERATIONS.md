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
| Explicit clone of an owned indexed element | checked begin, canonical `GenericCloneBorrow` or `HandleAwareCloneBorrow`, end |
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
Handle-containing referents use the distinct handle-aware recipe: Shared and Weak leaves clone
their counts, never their payloads. Direct handle referents and Struct/Enum/FixedArray/Vec graphs
containing handles use the same exact lexical authority and `BorrowReplace` commit. A formal
borrow retains caller-owned authority rather than inventing a callee-local source root.

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
Passing focused examples alone does not close any of these issues. Shared/Weak transition authority,
broader ownership CFG, backend execution and public activation retain their existing separate
requirements; this document does not waive their interaction obligations.

## Outstanding full-issue requirements

The source adapter consumes the #260/#261 Shared/Weak authorities as well as the non-handle
operation core. The #255/#256 reconciliation below includes authenticated handle source evidence;
raw opaque-slot tests alone would not discharge that obligation. Final closure still requires the
complete integrated verification receipts, not only this focused evidence. These obligations do
not expand #274's ordinary fixed-array acceptance criteria.
Fresh and chained ordinary FixedArray/Vec access use the explicit transient adapter above, and
nested lexical access from named containers finalizes that transient chain with `BindIndexedBorrow`.
Fresh Vec observation reuses one real owned temporary, even for Copy elements; it ends access
before dropping that Vec. Checked descendants may alternate FixedArray and Vec without
independent element ownership or intermediate clones. Arbitrary expression-base shapes,
borrowing a fresh temporary and fresh
mutation are not implied. These boundaries and the complete required gates must remain explicit
during acceptance reconciliation rather than be treated as completed generic support.

### Indexed borrowing acceptance reconciliation (#255/#256)

Both producers use one exact referent/whole-container-conflict implementation. The named evidence
is split by responsibility rather than by duplicating array and Vec lowering:

| Acceptance | Executable evidence |
| --- | --- |
| Copy and owned reads, writes and lexical restoration | `explicit_indexed_source`, `explicit_indexed_additional`, `indexed_handle_source` |
| Direct Shared/Weak and nested handle-containing Struct/Enum/FixedArray/Vec elements | `indexed_handle_source_shared_and_exclusive_clones_preserve_exact_authority`, `indexed_handle_source_replacement_prepares_rhs_before_exact_referent_drop` |
| Exact caller/formal access and cleanup | `explicit_indexed_calls`, `indexed_handle_source_calls_preserve_caller_owned_formal_clone_and_replacement` |
| Bounds, empty Vec/zero-length array, signed indices, effect ordering and chained containers | `explicit_indexed_additional`, `checked_chain_source`, `indexed_handle_source` |
| Static/dynamic siblings, compatible shared roots, owner exclusion and Vec growth restoration | `explicit_indexed_regions`, `explicit_indexed_siblings`, `indexed_handle_regions` |
| Exact diagnostic spans/messages, deterministic rejection and valid recovery | `indexed_handle_rejections`, `explicit_indexed_call_rejections` |
| Independent forged authority, type, ownership, initialized-mask and cleanup rejection | IR `indexed_borrow_hostile`, `indexed_borrow_owned`, `indexed_borrow_refinement`, `indexed_access`, `handle_aware_clone` |
| Exact/first-extra resource admission, checked overflow and pristine retry | `lexical_indexed_resources`, `indexed_handle_resources`, IR `indexed_borrow_resources`, `indexed_access_resources` |

The handle source matrix authenticates v4 syntax before lowering, then inspects mandatory verified
IR. Clone-failure cleanup retains the complete source container and unwinds the unpublished
destination prefix first. Replacement tests check the dedicated old-referent drop authority,
never a whole-container `DropPlace`; a failed RHS retains both the container and its separate
source. Vec reserve failure after lexical end likewise retains the original Vec and prepared
element, with no moved-element mask.

Resource tests inject explicitly labelled private accounting counters after an independently
successful authenticated control. They prove arithmetic/frontier rejection and unchanged
preparation state, not that an enormous source program or target allocator executed. Both reverse
cleanup accumulation and indexed-clone prefix accumulation diagnose checked arithmetic overflow
before consuming preparation. Executed count/allocation faults remain #263 and downstream target
work; no public profile, returned borrow, pointer or runtime no-alias optimization is enabled.

### Ordinary fixed-array acceptance reconciliation (#274)

The ordinary adapter owns Copy observations, explicit owned observations and element
replacement, not independent owned-element moves or persistent lexical aliases.
Fresh call/construction and explicit array-clone observation bases use genuine temporary
storage; checked descendants retain the maximal static conflict prefix. Fresh mutation
remains excluded. No handle producer, ownership CFG extension or public activation is implied.

The five acceptance obligations map to the operation/exclusion matrix above; authenticated
`ordinary_array_source`, `ordinary_array_composition_source`, `ordinary_array_clone_base_source`
and `ordinary_static_prefix_source` fixtures; independent `indexed_access` and
`indexed_access_copy_storage` malformed-input/initialization checks; and the separate
`ordinary_array_composition_resources` and `indexed_access_resources` exact/first-extra,
overflow and replay controls. The composition evidence document locates these tests and
distinguishes source-to-verified-IR observations from injected resource controls.
The final obligation still requires execution receipts for the complete applicable suites,
ignored boundaries, preflight, M0/M2, Linux/Windows checks and independent review on the
integrated tree. Focused clone-base tests alone are not those receipts or runtime fault execution.
