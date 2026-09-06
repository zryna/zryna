# M3 ownership composition interfaces

Status: Issue #277 planned implementation contract, not implemented generic source semantics.
Authority baseline: `8cc4eed8d522976ca557a27ea54993fb0d5ebf1c`. No runtime, backend, driver route,
public profile or target execution is enabled. The [evidence matrix](M3_OWNERSHIP_COMPOSITION_EVIDENCE.md)
separates existing declarations/tests from proposed interfaces and future proofs.

Later #278 candidate extensions are documented in [generic function operations](M3_GENERIC_FUNCTION_OPERATIONS.md),
[structural clone](M3_GENERIC_CLONE_CORE.md), and [generic Vec operations](M3_GENERIC_VEC_OPERATIONS.md).
The bounded constructor/root-replacement slices described below record their original scope;
their earlier exclusions are not a substitute for the later candidate's separate evidence.

The normative authorities are [data ownership](../spec/language/DATA_OWNERSHIP_V1.md), sections
2–12, and [ownership runtime ABI](../spec/abi/OWNERSHIP_RUNTIME_V1.md), sections 2–8.
The [Shared/Weak contract](M3_SHARED_WEAK_AUTHORITY.md) owns SW1–SW5 and handle meanings;
this stage adapts them without changing syntax, ABI operations/statuses, budgets or diagnostics.

## C1: Typed operands and complete payload identity

Input: authenticated snapshot/expression/span, sealed type universe and both target layouts,
exact value/place identity and available ownership/projection/refinement state. Output: one typed
operand role, distinguishing a Copy value, unique owned value, read-only place operand and exact
call-scoped borrow authority. Raw IDs, matching spellings or structural similarity grant no authority.
Wrong universe, nominal target, unavailable place or conflicting access rejects before dependent effects.

Full #83 requires `bool`, `i32`, String, every Struct field and Enum variant, zero/nonzero
FixedArray, empty/nonempty positive-stride Vec, nested containers, direct and nested `Shared<T>`/
`Weak<T>` leaves, and finite constructed values through legal nominal indirections. By-value cycles
and zero-stride Vec elements remain rejected. Opaque typed handle slots let #278/#279 compose
operations before #261 supplies real handle production; they do not prove source handle support.
Type mapping must resolve complete instantiated identities without assuming every referenced type
has an earlier mapped index. Borrow authorities are never storable payloads.

## C2: Ordered preparation and one-time commit

Input: exact operation, ordered children, operand effects and current owner state. Output: a
single-use prepared plan binding reserved resources, success result/effects and failure cleanup.
Planning validates structure/types and checked costs before dependent raw-arena or owner mutation.
Runtime preparation then evaluates children once in specified order: Struct declaration order,
array/Vec ascending order and other arguments/source expressions left to right.

Commit consumes only the prepared inputs and issues the exact result once. Failed preparation
publishes no result, retains the operation's uncommitted inputs and reverse-cleans completed
temporaries according to the original trap. This is not rollback of earlier successful expression
evaluation. A trap prevents later evaluation. Preserve diagnostic precedence, authenticated spans
and reservation-release order; a terminal resource diagnostic prevents later dependent work in
the selected preparation schedule.

Compile-time rejection and runtime failure are separate observations. The internal aggregate
child-preparation candidate leaves real compiler arenas, ownership state and type cache unchanged
when a child tree is rejected, while retaining earlier successful statements. Its runtime cleanup
instructions still follow the failure/commit contract above; planning is not target execution.
See the [candidate evidence and remaining scope](M3_OWNERSHIP_COMPOSITION_EVIDENCE.md) rather than
interpreting this internal checkpoint as completed generic C2 or public M3 support.

Issue #296 extends that shared authority to mixed non-handle constructor trees. Schedule selection
precedes evaluation and does not retry a failed operation through another lowerer. Previously
admitted complete aggregate trees retain interleaved semantic/resource checks. Newly admitted
mixed owned roots first prepare an ordered semantic/effect summary, then replay recorded resource
checks with actual ancestor credits. A semantic rejection in that summary precedes deferred
resource diagnostics. Legacy-shaped children inherit their mixed root's selected schedule;
standalone legacy Vec operations retain their existing route and precedence. Syntax/input limits,
layout checks, intrinsic byte bounds and checked arithmetic are not deferred by this policy.
The evidence matrix documents this deliberate boundary rather than asserting one global ordering.
For mixed-route local initialization, preparation also checks the destination local place and
initialization transition before consuming the initializer. Rejection preserves earlier statements,
bindings and ownership; this does not enable generic destination replacement.

## C3: Place replacement and initialized shape

Input: exact place topology, active enum variant, initialized field/element mask, borrow access and
replacement type. Output: a prepared replacement followed by one old-value cleanup and installation.
The old target remains owned until complete RHS preparation succeeds; failure preserves it for
trap cleanup. Partial-root transfers retain their exact masks where the verified authority admits
them, not merely a Boolean initialized flag. Reject overlapping or unavailable transfers.

Ordinary generic Vec observations and replacement belong to #278's core, with every required #83
handle combination integrated by #261 before #264 closes; #270 owns broader final integration.
Bounds and mutation access remain
mandatory; this does not invent Vec holes, element move-out or pop. Explicit borrowed Vec elements
remain #256; dynamically indexed elements conservatively belong to their complete container.

The bounded #278 mixed-root replacement slice uses this contract for mutable, fully initialized
non-Copy Struct/Enum/FixedArray/Vec locals whose target topology selects `MixedSummary`, inside
the existing private straight-line mixed-result route. A fresh supported constructor, distinct
whole move, or already admitted RHS operation prepares through the shared summary. Target validity
is checked first; after RHS semantic/effect preparation, the target must still be wholly owned and
the replacement must have a distinct exact-type owner. Destination consumption reports M3014 at
the complete RHS span before deferred resource replay. The final replacement transition is checked
after replay and before any real consumption. Commit emits one `ReplacePlace`, applies the proved
owner/fact change and checks the final preparation checkpoint. Rejected preparation preserves
earlier statements, the old destination, bindings, arenas, facts, type cache and surrounding credits.

Repeated assignments compose within that same straight-line route; the verifier derives each
old active payload and transfers pending completion order. This does not generalize partial or
projected replacement, indexed element mutation, clone capability, function signatures or CFG.
Legacy-shaped targets retain their prior route and diagnostic order, even in a mixed function.
The evidence matrix distinguishes verified cleanup obligations from runtime/storage execution.

## C4: Operation effects and cleanup

Input: operation/site, retained and consumed owners, exact type/layout/shape and cleanup role.
Output: success ownership and operation-bound failure cleanup, preserving status shape and original
trap identity. A single generic planner owns reverse completion order, active payload selection,
initialized-prefix cleanup and storage-last release. Return excludes exactly its transferred owner.
Structural clone retains its source; failure drops only completed destination leaves in reverse.
Handle clone is a checked count operation, never payload clone. Release does not allocate or call
source code. Reject forged role/site, duplicate drop, missing leaf, wrong variant or premature release.

SW1/SW2 bind control provenance and distinct handle owners, not counts alone. SW3 commits only a
fully initialized payload. SW4 finish requires completed exact recursive payload cleanup, never a
caller assertion that the payload is uninitialized. SW5 preserves construction/non-reentry
provenance: distinct cloned owners may share a target, but duplicate owner IDs and forged release
back into the same pending control fail. Do not introduce a blanket Weak-cycle ban or tracing.
The compiler seals symbolic obligations; concrete addresses/counts and one-time execution receipts
belong to the bounded #260 model and later target/runtime proof times specified by #259.

## C5: Owned calls and trap boundaries

Input: resolved exact same-program signature, acyclic call graph, ordered value/borrow arguments
and caller state. Prevalidate arity/types, then evaluate arguments left to right. Output on success:
one caller-owned result and precisely transferred callee inputs. During argument preparation the
caller owns completed argument temporaries. After transfer the callee owns its by-value inputs;
callee trap cleanup handles them, and caller trap cleanup excludes them. No double cleanup or
implicit clone repairs a mismatch. The callee's call-scoped borrow access ends on call completion,
on either success or trap. The caller's original lexical borrow authority is separately retained
and discharged under lexical rules; the call does not implicitly extend it or permit escape.
Neither authority may be stored or returned.

Mixed/nested payload arguments and results, including required handle combinations, are #278/#279
plus #261 integration obligations before #83 closes, not missing work deferred to #272.

## C6: Refined matches and continuations

Input: once-evaluated scrutinee, canonical exhaustive variant mapping and exact active-payload
refinement. Output per arm: typed result and ownership state, or an explicit terminated path.
Only the active payload is available; duplicate/missing arms, wrong refinement and repeated
consumption reject. A continuation transfers one result owner and reconciles complete remaining
definite state. No implicit drop/clone repairs unequal incoming states.

#278 owns operation/extraction effects; #279 owns arm/continuation CFG; #261/#262 integrate real
handles. Full #83 includes exhaustive multi-arm matches and nonterminal composition, not just the
existing one-arm direct-local/final-return checkpoint; it cannot wait for #273 afterward.

## C7: Lexical scopes and structured control flow

Input: binding scope, pending completion order, masks/refinements, active borrows and typed edge
signature. Output: explicit fallthrough, return or trap, with exact successor state or cleanup.
Nested/repeated blocks, branches and loops compose these results; termination cannot disappear
inside an untyped optional value. Return transfers its result and reverse-cleans remaining owners.
Joins require equal definite state, backedges restore header state, and lexical borrows discharge before
edges/returns. Reject mismatched ownership, masks, active variants or escaping authority rather
than inserting implicit clone/conditional-drop repair. Divergence is not a controlled trap.

#279 supplies the reusable scope/CFG adapter; #262 integrates authenticated upgrade bodies. Required
#83 nested payload calls/matches/returns stay within that integration, not a scalar-only fallback.

The initial internal `owned_aggregate_lowering/structured_cfg.rs` adapter now routes branches and
loops inside already-selected generic owned functions through the existing preparation machinery.
It records dense instruction ranges, explicitly restores branch planning state, requires equal
fallthrough ownership and loop-header masks, inserts lexical cleanup, and replays the completed
graph through `OwnedCfgState`. Authenticated `structured_owned_` tests cover nested/repeated
String-owner branches, a nested branch in a loop, terminal branch returns, unequal-state rejection,
and deterministic replay. `structured_match_` adds exhaustive multi-arm owned payload transfer or
clone into one typed continuation parameter, both direct-return and local-initializer forms. Copy
scrutinees use a compiler-private initialized temporary: each arm restores its original once-evaluated
SSA value through exclusive `BeginBorrow`/`BorrowWrite`/`EndBorrow`, so ordinary verifier refinement
rules admit the join without owner effects. Mixed Struct/Enum/Array/Vec scope graphs and checked
value/place/transition/cleanup exact, first-extra, overflow and pristine recovery have focused tests.
Structured functions are planned once in independent compiler-owned scratch state, sharing immutable
syntax/layout/catalog authority and staging diagnostics. Rejection leaves the original untouched;
accepted private state is published once after graph/resource checks, with mandatory full-program
IR verification still required before producing `VerifiedProgram`.
`structured_constructor.rs` now carries the existing affine constructor commit reservation and
ordered SSA operands through matches nested in Struct/Enum/FixedArray/Vec constructors. Earlier owned
operands remain pending during arm failures and transfer only at the exact typed constructor commit.
`structured_call.rs` carries the same result reservation through ordered internal value arguments,
then derives exact CallTrap cleanup from reconciled post-argument owners in scratch, transferring
owned arguments before the call cleanup. Nested FixedArray and two-function call fixtures repeat
the checked-resource/recovery matrix. Vec growth failure retains every completed child in reverse
completion order; its result is not yet pending. This is not #279 completion: the remaining indexed
composition boundaries, the complete C7 interaction matrix and
independent hostile evidence remain; no source handle or runtime-execution support is claimed.

Existing formal borrow parameters retain their exact sealed identity/access across Match edges,
as the IR's existing formal-parameter lifetime permits. Structured calls forward them in source
argument order, then use the canonical value-prefix/borrow-suffix call encoding. Exact aliases are
preserved at joins, failures end the same formal identity, and no EndBorrow/reborrow gap is emitted.
`structured_formal_` tests pin shared/exclusive forwarding, wrong-access/missing-alias diagnostics
and lexical-carry rejection. The resource matrix includes a genuine catalog-backed formal parameter.
Non-formal lexical/indexed authorities still cannot cross CFG edges (`I3011`); this slice does not
extend their lifetime or supply the broader #271 edge-borrow contract.

`structured_string.rs` preserves named/static String read places and genuine expression-result
owners through nested matches, then uses existing StringClone/StringConcat opcodes. Compiler-only
retained-read exclusions are exact overlapping places, carried unchanged through joins; consuming
or mutating access rejects while read-only clones remain allowed. The tail revalidates initialized
state and byte facts before releasing its own exclusions. `structured_string_` tests pin no extra
move/clone for named reads, arm/tail cleanup, read-only overlap, forbidden moves, post-operation
release, deterministic diagnostics and replay. Clone/concat shapes join the checked-resource matrix.
These exclusions do not grant IR borrow authority or extend lexical/indexed borrow lifetimes.

`structured_indexed.rs` admits a Match in the first checked index of a named/static Array or Vec
observation, including explicit owned element clone. An exact compiler-only container exclusion
survives the arms; a typed Copy handoff reuses the once-evaluated joined index in the existing
indexed preparation plan. Bounds and transient authority begin only after the join. The
`structured_indexed_` tests pin Array/Vec Copy/owned paths, unchanged SSA index identity, one bounds
site, hostile handoff rejection and pristine recovery; the resource matrix includes indexed clone.
Later-index and post-bounds RHS Match now use the separately verified internal transient
continuation contract in [M3_TRANSIENT_INDEXED_ACCESS.md](M3_TRANSIENT_INDEXED_ACCESS.md).
Staged preparation preserves each completed bounds check, the exact live authority and original
container while Match arms run; the final typed SSA handoff performs one read, clone or replacement
and one end. Owned RHS ownership remains pending until replacement. Lexical authority still obeys
I3011 edge prohibition. `continued_indexed_source` and the expanded structured resource matrix pin
Array/Vec Copy/owned ordering, failure cleanup, exact/first-extra/overflow rollback and recovery.
Fresh container Match operands are not admitted by this slice; no end/reborrow or target execution
is claimed.

## C8: Upgrade-success edge signature

Producer input: the exact `Weak<T>` type from the sealed type universe and the ordinary typed
successor-argument schemas. #260 supplies a producer-facing edge-shape descriptor before a complete
CFG exists. It describes an ordinary edge or an upgrade-success edge: success requires a first
`Shared<T>` parameter of the exact referent type, followed by parameters matching its explicit
arguments; expired requires only parameters matching its explicit arguments. Declaring that
synthetic raw parameter does not issue an owned value or prove an outcome.

The descriptor proves type/signature shape only. It does not require an already-verified upgraded
program and proves no operand liveness, site ownership, borrow discharge, cleanup completion,
final CFG validity or concrete refcount outcome. #279 uses it while assembling untrusted raw IR.
Mandatory full IR verification subsequently proves the complete operation, ownership flow,
dominance, edge signatures and site-bound cleanup; only the resulting opaque verified views carry
final authority for verified consumers. No producer helper can replace or bypass that boundary.

The symbolic operation contract remains indivisible upgrade: success alone issues the new owner,
expired issues none, and REFCOUNT takes neither successor while preserving original-trap cleanup.
Expiration skips upgrade's trap cleanup, not normal cleanup in the expired body. A compile-time
descriptor does not decide which outcome will execute. Bounded #260 transition-model evidence is
separate from real target/runtime execution. No reusable Boolean ticket, nullable handle or
preliminary count test is introduced.

`OwnedCfgState::finish_with_layouts` now derives the exact #260 `WeakUpgradeShape` from the
operand place and sealed layout. It matches success arguments after the synthetic first Shared
parameter and matches expired arguments without a prefix. Ordinary finalization remains unchanged;
an upgrade without layout context fails closed. The named `owned_cfg_upgrade_success_prefix_is_sealed_and_ordinary_arguments_remain_exact`
test covers both schemas, malformed prefixes and operands, missing context, and pristine recovery.
This is producer schema evidence only, not source upgrade support or a full-program ownership
proof. Every completed program must still pass mandatory full IR verification. The remaining
#279 C6/C7 structured ownership composition is not completed by this adapter.

The #262 source route now consumes that schema for an authenticated `upgradeWeak` statement.
It retains addressable Weak operands, materializes non-addressable operands exactly once, gives
only the success scope its synthesized Shared owner, and preserves the exact overflow cleanup.
This is compile-time ownership evidence; target outcome execution remains the #263 boundary.

## Integration and closure

#277 requires #77/#78/#80/#82/#259, not parent #269 closure. #278 follows #277;
#260 follows #259/#277; #279 follows #277/#278/#260; #261 follows #259/#260/#278;
#262 follows #259/#260/#261/#279. Preserve additional existing registry prerequisites.
Neither core waits for full #83 or #270–#273. Full #83 still requires all payload/control-flow
combinations, hostile/resource/replay evidence and #264 reconciliation. Broader #269 completion
and #254–#256 indexed borrowing remain mandatory separate work.

This stage is separately closeable after independent finite-interface review and prerequisite
verification, not after inventing placeholder hooks. Missing sound input/output/failure contracts
keep #277 open; absent producers remain explicit #278/#279/#261/#262 work. Implementers must freeze
exact diagnostics, coupled resource reservations and final module ownership before code changes.
No public `data-ownership-v1` selection precedes the final #89/#90 gates.
