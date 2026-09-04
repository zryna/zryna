# M3 Shared and Weak authority contract

Status: Issue #259 implementation/interface freeze, not implemented Shared/Weak source semantics.
Frozen baseline: `f1b88304e9ee918ba46808f60859097999785f1b`, after verified #82/#122 closure.
This document enables no runtime, backend, driver route, public profile, or target execution.
It does not close #83. The [evidence and integration matrix](M3_SHARED_WEAK_EVIDENCE.md)
distinguishes existing tests from required future tests.

The normative authorities remain [data ownership](../spec/language/DATA_OWNERSHIP_V1.md),
especially sections 2, 5–7, 10–12, and
[ownership runtime ABI](../spec/abi/OWNERSHIP_RUNTIME_V1.md), especially sections 2–3 and 7–8.
The following are implementation obligations, not changes to syntax, wire schemas, ABI signatures,
statuses, resource maxima, or the normative contract.

## Authority map

Paths name current implementation authorities, not a claim of a source producer.

| Layer | Existing authority | Missing integration owned by |
| --- | --- | --- |
| Source | `crates/zryna-syntax/src/v4.rs`: `RawTypeSyntaxKind::Shared/Weak`, `RawExpressionKind::Shared/Clone/Downgrade`, `RawStatementKind::WeakUpgrade`; source-map-bound verified snapshot | #261 exact type mapping/operations; #262 upgrade statement |
| Types/layout | `crates/zryna-layout/src/lib.rs`: sealed nominal identities, `TypeCategory`, `referenced_type`, finite by-value graph, target fingerprints | #261 must map every admitted instantiated handle type; current semantic `type_model.rs::map_node_types` does not map Shared/Weak |
| IR | `crates/zryna-ir/src/data_ownership_v1.rs`: `SharedConstruct`, `SharedClone`, `WeakDowngrade`, `WeakClone`, `WeakUpgradeBranch`, opaque instruction/terminator views | #260 independent complete operation/state/hostile proofs, then #261/#262 producers |
| Ownership/drop | IR `InitializePlace`, `MoveFromPlace`, `ReplacePlace`, `DropPlace`, sealed site/role cleanup and derived recursive drop actions | #277 interfaces, #278 non-handle core, #261 handle leaves, #279/#262 control-flow integration |
| ABI | `crates/zryna-ownership-runtime-abi/src/lib.rs`: `VerifiedControlLayout`, `ControlState`, `TransitionClaim::Control`, `validate_transition`, operation-bound failure claims | #260 bound control/handle authority and completion evidence below; #263 hostile integrated fixtures |

The IR already rejects wrong instruction payload/result types with `ZRYNA-I3005` and wrong upgrade
successor/result shape with `ZRYNA-I3014`. This is not complete source support or authenticated
runtime control ownership. The semantic structural Clone calculation admitting handles likewise
does not establish source lowering. No backend may fill these missing compiler proofs.

## Complete payload domain

Every row is required for full #83, not an optional post-#83 extension.

| Payload category | Required composition and exclusions |
| --- | --- |
| Scalars | `bool`, `i32`; no widening/coercion |
| String | empty/nonempty UTF-8 and nested ownership; constructing Shared moves String, not its bytes through a new clone |
| Struct | all fields, nested nominal identities, owned/container/handle fields, declared initialization/drop order |
| Enum | every variant, payloadless and owned active payloads, nested enums; inactive payload never read/dropped |
| FixedArray | zero and nonzero lengths, nested owned/handle elements; ascending preparation, descending cleanup |
| Vec | empty/nonempty, nested containers/aggregates/handles; only positive-stride element layouts, rejecting zero-sized elements as required by the ABI |
| Shared/Weak | direct handle payloads and handles within any admitted aggregate/container; clone increments count without requiring payload Clone |
| Recursive nominal types | recursion through admitted indirections with finite by-value layout; finite constructed values and valid control provenance |

Borrow authorities are never storable payloads. Shared/Weak are never Copy. There are no
user-defined generics, nullable handles, implicit clone, raw pointers, source count/identity
inspection, or direct Weak dereference. Zero-sized fixed-array payloads still require a nonzero
control allocation; this does not admit zero-stride Vec elements.

## Source evaluation, spans and state

Use the existing authenticated protocol-v4 forms unchanged. Resolve exact module/declaration/type
identity before selecting operations; source spelling or structural similarity cannot substitute.
Evaluate each expression once, left to right; struct initializers retain normative declaration
order, array/Vec elements ascending order. A trap prevents later evaluation.

`shared(value)` consumes its fully initialized payload only at successful commit. Copy operands
supply a copied value; non-Copy operands transfer one unique owner. Clone and downgrade read their
handle operand without consuming it. Upgrade evaluates its Weak operand once and retains it on
success, expiration and count-overflow preparation failure. A temporary handle operand must be
materialized as a distinct pending owner until its exact lexical/expression cleanup point; no
implicit clone is inserted to manufacture a place.

Keep the existing source-map identity and expression span on the operation; operand-specific type,
moved-owner, or conflicting-borrow diagnostics use that operand's authenticated span. Upgrade
uses the whole statement span for its terminator and the binding token span for binding errors.
The success binding exists only in its success block, never the expired block or an outer scope.
Its body has ordinary move/return/drop semantics: a moved-out value transfers ownership; otherwise
the binding is dropped on exit. A success result is not an escaping borrow.

Authenticate the snapshot first, resolve declarations and exact types deterministically, then
derive source-ordered ownership/operation plans and checked resource requirements before raw-IR
materialization or ownership mutation. Stop dependent work when an authority is absent; a terminal
budget diagnostic prevents later planning. Preserve existing diagnostic families and ordering.
#260/#261/#262 must freeze
any new exact code/message/span and competing-error precedence in their executable tests before use;
#259 reserves no unreviewed numeric code. A later valid compilation must replay identically after
every rejection. Never route malformed handle operations through a fallback scalar lowerer.

## Operation and failure ledger

All statuses are ABI statuses, not source values. `ABI_VIOLATION` stops the trusted boundary and
must not replace a language trap. The table describes valid input; forged inputs fail independently.

| Operation | Success and retained ownership | Non-success |
| --- | --- | --- |
| Shared construction | Prepare fully initialized T and checked control layout; allocate nonzero control, initialize counts(1,1), move payload, publish one Shared<T> | Size/limit overflow: CAPACITY; valid unsatisfied allocation: ALLOCATION. No result; input still owned for original-trap cleanup |
| Shared clone | Retain source; checked strong increment; publish exactly one Shared of same T/control | MAX strong: REFCOUNT, unchanged input/counts, no result |
| Downgrade | Retain live Shared; checked weak increment; publish one explicit Weak of same T/control | MAX weak: REFCOUNT, unchanged input/counts, no result |
| Weak clone | Retain explicit live Weak; checked weak increment, including expired payload control | MAX weak: REFCOUNT, unchanged input/counts, no result |
| Weak upgrade | One indivisible observation plus checked increment; success edge receives exactly one new Shared<T> | strong=0: EXPIRED, no new value or trap cleanup; strong=MAX: REFCOUNT before either successor, original-trap cleanup |
| Non-last Shared release | Consume one Shared, decrement strong only, preserve payload and weak count | Infallible for verified input; corruption is ABI_VIOLATION |
| Last Shared release | Begin1→0, reserve control; complete recursive payload drop once; finish removes implicit weak, frees control iff no explicit Weak remains | No allocation or source callback; invalid phase/completion is ABI_VIOLATION |
| Explicit Weak release | Consume one explicit Weak; decrement weak; free iff no strong/implicit owner and last explicit Weak gone | Never consumes implicit weak; double/stale/wrong-control release is ABI_VIOLATION |

Shared construction composes existing allocation and compiler-owned initialization; it does not
add an eighteenth runtime operation or a new external ABI call. Non-OK output handles/pointers are
zero; boolean outcomes have the exact operation-specific zero shape. Generated code validates
status and complete result shape before ownership commit. REFCOUNT maps only to
`zryna.trap.refcount-v1`; construction ALLOCATION/CAPACITY preserve their respective exact traps.
Nested payload preparation may additionally produce its existing bounds/UTF8/other specified trap.
Cleanup retains that original identity and never allocates.

## Upgrade CFG contract

Reuse `WeakUpgradeBranch { weak, success, expired, cleanup }`, not a preliminary liveness test,
a reusable Boolean ticket, Option, nullable result, or source-visible count.

The first success block parameter is the synthesized Shared<T> for the exact Weak<T> payload;
remaining success parameters correspond to ordinary explicit edge arguments. The expired edge
has only its ordinary arguments and no synthesized handle. Expiration skips only upgrade's trap
cleanup, not normal cleanup later in the expired body. Overflow takes neither successor.

Every edge still proves complete definite owner/initialization state, dominance, exact argument
types and unique owner transfer. Branch locals reverse-drop before exits; joins require equal
incoming state. Loop backedges restore exact header state; borrows do not cross edges. Returned
owners are excluded from local cleanup and transferred once. Required #83 contexts include nested
and repeated branches/loops, call operands/results and exhaustive match payloads wherever the
normative language admits them. The current bounded #81/#82 source subsets do not discharge these
cases; #277/#279 infrastructure and #261/#262 integration must implement them before #264 closes.

Single-threaded indivisibility means no observable gap between deciding success and incrementing.
It does not authorize threads, atomics, synchronization, host reentrancy or memory-ordering claims.

## Required sealed integration interfaces

These are named obligations for review/implementation in #260, not public Rust APIs supplied by
this document. Raw claims remain untrusted; no Boolean assertion or caller-selected identifier
alone can create a receipt. #277 adapts these contracts without becoming a prerequisite of #259.

There are two distinct proof times. The compiler seals symbolic type, owner, site, control-flow,
layout and cleanup obligations; it cannot attest a future concrete allocation address, dynamic
count or runtime invocation. A test transition model may instantiate those obligations with bounded
invocation-local claims. Future target/runtime implementations must bind actual allocations and
one-time execution receipts to the compiled obligations at execution, then independently audit
them at their trusted boundary. That later work belongs to #84–#87, not a runtime in #259/#260.
The table describes both sides of that interface, not static materialization of all runtime graphs.

| Decision | Required input/binding | Verified output and rejection boundary |
| --- | --- | --- |
| SW1 control provenance | Exact verified ABI/layout target, payload TypeId/type-universe/fingerprint; unique live allocation identity, base/size/alignment; invocation and transition site | Opaque bound control authority, rejecting wrong type/target, non-base/overlapping/stale allocation, cross-control or cross-invocation replay |
| SW2 handle ownership | Issued control identity plus distinct strong/explicit-weak owner identity and live/moved/dropped state | One checked transition consumes or retains precisely those owners; implicit weak is not an issuable source handle; counts alone cannot authenticate ownership |
| SW3 construction preparation | Fully initialized payload owner and layout-derived checked offset/size; operation-bound allocation result and unchanged-input failure proof | Unpublished prepared control; successful one-time commit transfers payload and issues first Shared. Failure issues no control/handle and retains input |
| SW4 last-release completion | One last-strong begin receipt bound to control/site; exact sealed payload drop topology, active variants and initialized masks; completed release trace | One non-replayable finish permission, only after payload cleanup; rejects omitted/duplicate/out-of-order leaves, premature finish and forged payload_initialized=false |
| SW5 control graph admission | Complete bounded claimed control/owner graph or authenticated construction witness, bound to payload topology and release obligations | Strong-edge acyclicity plus construction/non-reentry provenance; reject dangling/duplicate owner IDs, foreign ownership and forged cycles; repeated target edges from distinct cloned owners are legal |

SW1/SW2 extend proof context, not source types or target ABI bytes. Existing `ControlState` contains
only counts/phase/initialized/allocation flags; pure transition acceptance alone supplies none of
those identity or one-time-use bindings. Existing layout fingerprints do not authenticate a live
allocation. #260 must independently verify hostile raw claims and supply opaque outputs before
#261 relies on them.

The weak count includes explicit Weak owners plus one implicit owner while strong>0 **or a
last-strong release is pending**. Pending begin makes strong=0 while payload cleanup may still be
in progress; this reserved state is not ordinary expiration. Existing preflight admits no other
transition on that same control until finish. Recursive payload cleanup may release different
controls; it must not reenter the pending control. Finish consumes the receipt once, removes the
implicit owner, and deallocates only at weak=0. No payload read occurs at strong=0 except the
privileged compiler-owned destruction represented by that pending receipt.

## Cycles, cleanup and resources

Fully initialized immutable construction can only embed already valid handles; it exposes no
self handle during preparation. There is no partial Shared publication, interior mutation,
cycle constructor or runtime tracing/cycle collection. Weak observers/back links can be assembled
while payloads are uniquely owned; they never grant mutation or make an impossible strong cycle
constructible. Layout by-value-cycle rejection and control-graph validation are different proofs.

SW5 is a bounded hostile-claim admission/provenance check owned by #260/#263, not runtime tracing
or periodic cycle discovery. Reject cycles in claimed strong ownership; do not treat a non-owning
Weak edge as a strong edge or reject every harmless weak observer. Multiple distinct cloned owners
may name the same control. A duplicate owner identity is different from those legal repeated edges.

Strong-edge acyclicity alone is insufficient: a forged Weak self/backedge could cause recursive
payload destruction to release the same pending control. The admission proof must also authenticate
construction provenance and prove compatibility with the existing non-reentry rule. Newly published
payloads can contain only already issued handles; immutable existing Shared payloads cannot be
patched to refer to a future control. This explains why an unconstructible forged self/backedge is
not justified merely by labeling it Weak. Ordinary observers and lawful acyclic parent/back links
remain admitted. Do not invent a blanket source Weak-cycle prohibition or a cycle-construction API.
#260 must independently prove this implication for its chosen symbolic/witness representation and
reject forged same-pending-control release sequences; current count-only validation does not prove
it. If the representation cannot establish it, that is a blocking authority gap, not permission
to loosen the ABI's pending-phase rule. Generated verified transitions preserve admitted graph
invariants; concrete target implementation/audit obligations stay with #84–#87.

Recursive drop/clone plans include handle leaves under every payload row. Aggregate/Vec clone
prepares in order, then on failure drops only completed destination leaves in reverse, storage last;
source obligations remain intact. Successful handle clones already in that prefix must be released
once. Replacement retains the old target until the entire replacement succeeds. Call trap, return,
scope exit, branch and loop cleanup all use the same exact ownership authority, not duplicated
ad-hoc cleanup paths.

Preflight checked counts before allocating raw arenas or mutating source ownership. Preserve all
language/IR/layout/ABI ceilings: places65,536, transitions/drop-actions262,144, active borrows16,384,
strong/weak counts u32::MAX, dynamic allocation2,147,483,647bytes, live allocations and allocation
operations1,048,576, runtime status transitions4,194,304, and retained diagnostics256.
Use sealed target control size/alignment, not host usize or Rust layout; even empty payloads allocate.
Exact/first-extra, checked byte amplification and overflow must be proven without changing budgets.
A coupled unreachable source maximum needs an honest verifier/ABI proof classification, not an
unreachable source test or silent skip.

## Integration gate

#259 depends on completed #80/#81/#82, never on #277/#278/#279 or #83 completion. #277 adapts
SW1–SW5 and required composition hooks; #260 verifies handle/upgrade authority; #278 supplies
non-handle owned operations; #279 supplies CFG using verified #260 edges. #261 integrates handle
producers after #278; #262 integrates upgrade after #279; #263 adds adversarial/failure/resource
proofs; #264 independently reconciles every evidence row before #83 closes.

No required #83 payload, call, match, cleanup or control-flow case is deferred to later
#270–#273 completion. #269's broader source closure and #254–#256 indexed borrowing remain
separate tracked obligations, not permissions to narrow #83. The existing checked registry is
unchanged by this document; live child integration dependencies are recorded in the evidence matrix.
