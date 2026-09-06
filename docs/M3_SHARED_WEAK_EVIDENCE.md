# M3 Shared and Weak evidence matrix

Status: Issue #259 integration plan with #260 independent proof evidence and the internal
#261/#262 source checkpoints below. Planned target-runtime tests remain future execution evidence.
Read the [authority contract](M3_SHARED_WEAK_AUTHORITY.md) for SW1–SW5, complete payload domain,
operation semantics and exclusions. A scalar-only checkpoint cannot discharge #83.

## Independent #260 executable mapping

The following tests exercise compiler/ABI proof boundaries, not source lowering or target
execution. The broader named integration matrices later in this document remain obligations
of their listed owners; this mapping does not claim #83 completion.

| Executable family | Exact evidence |
| --- | --- |
| IR `data_ownership_v1::tests::shared_weak_authority` | Authenticated layout/source-backed raw programs for all payload categories, retained clone/downgrade operands, success-only synthesized ownership, typed outcomes, exact ordered cleanup, hostile types/modes/moves/borrows/edges/cleanup, rejection replay and valid recovery |
| IR `shared_weak_authority::payload_construction` | Fully initialized owned aggregate/container/handle payload construction, nested and recursive-indirection types; not only Copy parameters |
| IR `shared_weak_authority::upgrade_shape` and `WeakUpgradeShape` doctest | Branded reusable C8 edge schema, foreign/missing types, final mandatory verifier rejection, private-field opacity |
| ABI `control_model_clone_expiration_and_payload_before_implicit_weak_finish` | Both target layouts; live and expired Weak cloning, expiration produces no owner, exact last-strong payload-before-implicit-weak finish, premature/replayed release rejection |
| ABI `control_model_provenance_topology_modes_and_allocation_failures_are_atomic` | Wrong invocation/site/allocation geometry, dense identity, unissued handles, overlapping controls, wrong mode, non-success publication and deterministic valid recovery |
| ABI `control_model_immutable_handle_graph_and_recursive_release_are_exact` | Exact nested strong/Weak payload transfer, reverse recursive release cascade; future/self/duplicate edges and reordered cleanup fail |
| ABI `control_model_payload_shapes_derive_reverse_active_cleanup_and_vec_storage_last` | Both targets; scalars, String, both Enum variants, array0/2, Vec0/2; omitted drops fail and Vec storage is last |
| ABI `control_model_allocation_failure_retains_embedded_handle_until_successful_retry` | Failed publication retains source handle; successful retry transfers it exactly once; stale payload-handle access fails |
| ABI `control_model_requires_exact_independent_surviving_owner_contract` | Omitted release and wrong expected owner/control/mode fail; exact independently expected returned owner passes |
| ABI `control_model_internal_resource_boundaries_publish_nothing_on_rejection` | Private counter controls exercise actual allocation/node replay preflights at exact/first-extra/checked-overflow with no owner publication and valid fresh-invocation recovery; not million-allocation execution |
| ABI `control_model_count_boundaries_remain_indivisible_existing_abi_claims`, `control_model_synthetic_refcount_failure_cannot_issue_an_owner` | Explicitly synthetic saturated count/checked-budget proofs; existing transition authority distinguishes success, expiration and unchanged overflow, and no overflow owner can be issued; not billion-owner programs |
| ABI `VerifiedControlTrace` doctests | Caller construction/mutation of opaque proof fields fails to compile |

Source evaluation/span/fault injection and executed target count/drop behavior are not supplied
by these symbolic tests. Existing independent IR resource and ABI transition suites remain
required alongside this matrix. Full gate/CI receipts must be recorded separately when run;
listing an executable here is not a claim that Linux/Windows integration gates already passed.

## Issue #262 source upgrade checkpoint

`weak_upgrade_source` authenticates retained addressable and once-evaluated temporary Weak
operands, success-only Shared binding scope, exact wrong-type/missing-name diagnostics, rejection
replay and valid recovery. `weak_upgrade_fault_oracle` binds the verified terminator to the sealed
#260 success/expired/refcount-overflow claims without pretending to execute a target runtime.
`weak_upgrade_state` additionally freezes complete moved/reuse, live-borrow edge and unequal-join
diagnostics, with an authenticated equal-join control proving the original Weak root remains live
until later cleanup. Upgrade statements select structured lowering before legacy borrow-only gates;
this does not extend borrow lifetimes across edges. Temporary source tests pin producer-failure
cleanup without the uncommitted result, overflow cleanup with that completed temporary first, and
one release before each outcome body followed by exact survivor cleanup.
`weak_upgrade_exact_extra_overflow_resources_restore_pristine_state` covers exact, first-extra and
`usize::MAX` held credits for value, place, transition, cleanup-action and cleanup-plan limits with
unchanged rejected state and pristine recovery. Transition demand includes root-scope drop credits
derived from the authenticated owned declarations, not a fixed unexplained offset.
Independent IR `weak_upgrade_temporary_edges_reject_forgery_and_cleanup_corruption_then_recover`
starts from a complete accepted temporary-release graph and rejects a forged success type,
overflow cleanup omission/reordering and missing expired-path release, then re-verifies the
pristine graph. This is bounded compiler evidence, not full #262 closure or executed target cleanup.

## Issue #261 source integration checkpoint

The semantic source route maps authenticated `Shared<T>`/`Weak<T>` types and lowers direct
straight-line `shared(value)`, explicit handle `clone`, and `downgrade` expressions through
mandatory IR verification. Named source tests cover bool, i32, String, nominal Struct/Enum,
zero/nonzero fixed arrays, positive-stride Vec, nested Shared payloads, payloadless/multi-variant
Enums, and finite recursive nominal values through Shared indirection. They also pin source
order, reverse failure cleanup, moved/wrong-type diagnostics, deterministic rejection replay, and
the exact/first-extra cleanup-action frontier with pristine recovery.

The source route also materializes non-addressable clone/downgrade operands, retains the exact
temporary in failure cleanup, and emits its successful `DropPlace` at the expression boundary.
Authenticated composition evidence constructs handle-containing Struct, Enum, fixed-array and Vec
values, clones static handle projections, replaces a static handle field, and transfers a nested
handle aggregate through one internal straight-line call. These paths use explicit
`SharedClone`/`WeakClone` count operations; they do not reinterpret handle leaves as Copy.

Source-authenticated symbolic fault-oracle evidence binds each emitted Shared allocation/count
instruction to the exact frozen ABI logical operation, admitted status and trap disposition.
Handle-aware structural clones additionally bind a canonical recipe-step ordinal to their exact
unpublished destination prefix, retained source root and reverse cleanup. Rejection replay and a
fresh valid lowering are deterministic. This reuses #260 transition authority and is compiler
evidence only: it does not claim an allocator, count mutation, fault injection or cleanup was
executed by a target runtime; those observations remain #263 work.

This is the compile-time #261 closure candidate, not #83 closure or target execution. The named
`recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay` test authenticates
the final #259 payload-domain cases through source lowering and mandatory IR verification. A
distinct verified handle-aware clone contract retains exact place/indexed-borrow source authority,
a distinct destination, and a finite canonical recipe graph. That recipe requires declaration-order Struct,
runtime-active Enum, ascending fixed-array/dynamic-Vec traversal and explicit Shared/Weak count
operations, with initialized-prefix cleanup before surviving roots. It does not relax the generic
non-handle clone contract and does not claim that a target backend executed the recipe. Full
count/allocation execution faults remain #263 work, while broader CFG upgrade/match composition
remains tracked by #262/#269. No target runtime or public profile is enabled.

## Existing evidence and its limits

All names below are existing tests in
`crates/zryna-ownership-runtime-abi/src/tests.rs` on the frozen post-#82 baseline.
They remain regression authorities; this planning change does not claim to rerun Rust tests.

| Existing test | What it establishes, not more |
| --- | --- |
| `pure_vec_and_count_transitions_are_exact` | Pure state/count claims; not executed allocation or source producers |
| `all_control_transitions_and_illegal_variants_are_checked` | Canonical operation/state cases and invalid variants; not issued handle provenance |
| `pending_last_strong_excludes_every_operation_except_finish` | Pending-phase operation exclusion; not authenticating completed recursive payload drop |
| `non_success_control_results_are_zero_shaped` | Operation-specific non-success boolean/result shape |
| `failure_atomicity_is_bound_to_the_exact_operation_status_set` | Exact operation/status and unchanged-input claims; not a bound control allocation |
| `layout_binding_rejects_target_and_fingerprint_mismatch` | Sealed layout identity; not ownership of a live control graph |
| `contextual_transition_claims_fail_closed_without_their_authority` | Context-sensitive claims require their authority; not complete SW1–SW5 implementation |

Existing IR raw/opaque operations and upgrade successor typing are authority to extend and test,
not evidence that source operations work. Existing #81/#82 cleanup, borrow, fault/drop-trace and
resource tests must remain unchanged and green. Their limited source shapes do not prove generic
nested handle cleanup or owned CFG composition.

## Required named matrices

Names designate new test families; implementing owners may split a family into bounded modules,
but #264 must retain a name-to-executable-test mapping for every row. Each family requires exact
assertions, not only `is_err`. Run the complete payload cross-product from the authority contract
where an operation is meaningful. Rejected combinations must be deliberate normative exclusions,
not hidden implementation gaps.

| Planned test family | Semantic positive/negative proof | Independent IR / ABI / hostile proof | Owner |
| --- | --- | --- | --- |
| `shared_payload_category_matrix` | Every scalar, String, struct field, enum variant, array0/nonzero, positive-stride Vec and nested handle category; exact nominal identity, legal indirect recursion | Forge wrong payload/result/type-universe/target; zero-sized Shared nonnull versus zero-stride Vec rejection | #260/#261 |
| `shared_construct_failure_retains_input` | Evaluate payload once, allocate only after complete preparation, move only at commit; allocation and capacity traps preserve source until cleanup | SW1/SW3 stale allocation, wrong size/alignment, zero/alias result, failure with nonzero result or changed input | #260/#261/#263 |
| `shared_weak_clone_does_not_clone_payload` | Clone/downgrade retain operand; exactly one new owner, no payload copy/clone or implicit call/return clone | Exact strong/weak increment, expired Weak clone, wrong-control handle, unrelated state unchanged, non-Clone payload independence where admissible | #260/#261 |
| `weak_upgrade_success_only_owner_and_expired_no_value` | Evaluate Weak once; success-only local scope; expired block has no handle; normal branch cleanup remains | Correct first success parameter and owner; reject fabricated expired result, extra/missing result, mismatched T, reusable nullable/check token | #260/#262 |
| `weak_upgrade_overflow_takes_neither_successor` | Live strong MAX retains Weak, traps before either body, preserves original trap/drop order | Indivisible success increment; no check-then-upgrade gap, MAX failure state unchanged/zero result; exact status variants | #260/#262/#263 |
| `last_strong_payload_before_implicit_weak_finish` | Last versus non-last release; nested active payload fully dropped before control finish; no source callbacks | SW4 premature/replayed finish, skipped/duplicate/out-of-order drop, unrelated-control recursion, same-control pending exclusion | #260/#261/#263 |
| `implicit_weak_is_not_releasable_as_explicit` | Live/expired explicit Weak ownership and exactly-once release | SW2 distinguish explicit handles from implicit count; no strong-live deallocation, double release, stale handle or owner replay | #260/#263 |
| `handle_prefix_cleanup_is_reverse_and_exact_once` | Partial construction/structural clone/replacement: every fault ordinal, reverse completed destination prefix, old target/source retained | Sealed site/role/active-variant/moved-mask topology, no uninitialized leaf drop, container storage last, no cleanup allocation | #261/#263 |
| `control_identity_layout_replay_rejected` | Independent programs preserve nominal/control identity, no address/count API | SW1/SW2 cross-control/site/invocation/target replay, bad base/size/alignment/fingerprint, orphan/duplicate owner, bad status shape | #260/#263 |
| `forged_control_cycles_fail_closed` | Immutable complete construction and weak observer/back-link cases; reject partial publication/interior mutation | SW5 strong-edge graph plus construction/non-reentry witness; reject forged Weak self/backedge release of a pending control, cycles/dangling or duplicate owner IDs; accept distinct clones sharing a target and lawful Weak observers | #260/#263 |
| `handle_resource_exact_first_extra_and_overflow` | Reachable fully authenticated exact/first-extra source budgets; checked preparation before mutation | Independent IR/ABI maxima, count MAX-1→MAX then REFCOUNT, expired0, byte/control-size limits, bounded graph budget and exhaustion without partial authority | #260/#263 |
| `handle_diagnostics_replay_after_failure` | Exact code/message/span/order, competing source errors, no later effect after trap; valid compile before/after rejection identical | Forged spans/source-map, malformed state before materialization, deterministic terminal budget and fault replay | #260/#261/#262/#263 |
| `handle_cfg_calls_match_and_scope_cleanup` | Nested/repeated branches/loops, exact joins/backedges, argument order and owner transfer, returned handle, active match payload, lexical exit/trap cleanup | Dominance, synthetic upgrade owner, wrong edge counts/types, missed/duplicate cleanup, moved/borrowed edge escape and mismatched header state | #260/#261/#262 |

Every successful semantic case must authenticate real source through syntax, layout, semantic
lowering and mandatory IR verification. No helper-only boolean oracle substitutes for source
acceptance. IR negatives start from independently authenticated valid programs and mutate the
exact authority under test; ABI negatives exercise raw claims plus independently bound layouts.
Fault tests must check the original trap identity, all surviving owners and exact ordered release
trace. Valid replay follows every hostile case. Test-only traces are not runtime implementation.

Exercise expressions nested in constructor operands, call inputs/results, active match payloads,
branch arms and loops, not only direct scalar locals. Cross independent control allocations and
multiple explicit Weak handles; include strong count0/1/MAX-1/MAX, weak explicit0/1/MAX, non-last
release, pending initialized/finished payload, live and dead controls. Count-only synthetic boundary
tests are labeled ABI proofs, never fabricated billion-handle source executions.

## Missing-interface acceptance

- SW1/SW2: opaque constructors remain inside the verifier/ABI authority; compile-fail opacity tests
  reject caller-created or mutable receipts. Bind exact payload/type/target/control/invocation/site
  and complete owner set at the appropriate proof time. Static compiler obligations are not actual
  runtime allocations: concrete invocation-local execution binding belongs to future target/runtime
  implementation. Test models supply hostile bounded claims, not compiler attestation of addresses.
  Reject replay even when counts happen to match; distinct owner IDs may legally share one control.
- SW3: prove checked layout/size/allocation and initialized payload before commit; validate all
  output/status fields. No new runtime symbol or schema is needed merely to describe preparation.
- SW4: a raw `payload_initialized=false` flag is insufficient. Receipt production must independently
  replay the complete sealed recursive drop obligations and issue one finish authority.
- SW5: authenticate a complete bounded graph/witness at the hostile-input boundary. A bare
  `acyclic=true` or a source-selected control ID is insufficient. Normal execution must not gain
  tracing, cycle discovery, callbacks or reentrancy. Strong-acyclic-only acceptance must not admit
  a forged Weak edge whose recursive drop reenters the same pending control. Prove construction
  provenance and release compatibility without banning lawful observers or inventing source forms.
- Exact Rust API representations and numeric diagnostics are reviewed in #260 before executable
  use. Any required new wire/schema/ABI change needs a separate explicit review, not quiet reuse
  of a field with a new meaning.

## Dependency and ownership ledger

| Child | Required handoff and independent acceptance |
| --- | --- |
| #259 | Freeze this complete contract after #80/#81/#82; independent normative/source review and documentation checks, no reverse #277 dependency |
| #277 | Adapt #259 contracts into generic composition interfaces, including required call/match hooks; no implementation wait on #83 or full #270/#271 |
| #260 | Verify SW1–SW5, complete transitions/upgrade authority and hostile boundaries after #259/#277; independent ABI and IR reviewers |
| #278 | Non-handle owned-operation core after #277, including generic container composition needed by #261; independently closeable |
| #279 | Ownership CFG core after #277/#278/#260, using verified edges; never waits for #262 |
| #261 | One semantic integration owner for type mapping, all payload producers, clone/downgrade and recursive cleanup after #259/#260/#278 |
| #262 | Same coordinated semantic state owner integrates upgrade, full required CFG/call/match behavior after #259/#260/#261/#279 |
| #263 | Independently constructed integrated negative/fault/count/cycle/resource matrix after #259–#262 |
| #264 | Reviewer distinct from implementation reconciles every row, exact commits, public claims and full regressions before #83 closes |

ABI and IR implementation may proceed on separately owned files after the interface prerequisites
are frozen; semantics and shared cleanup cannot have competing writers. Independent test modules
may proceed once their input interfaces are immutable. No stage can cite a later full #270–#273
feature as missing support for an already-required #83 case. Broader #269 and normative indexed
borrow #254–#256 chains remain separate and unchanged.

## Verification and publication gate

For #259 run the repository's Node contract and documentation checks, exact bundle inventory and
independent review. This document supplies no executed source example and no Rust/runtime proof.
Any executable authority change in downstream children requires focused semantic/IR/ABI tests,
opacity doctests, full preflight, M0/M2 regressions and required Linux/Windows CI on the final
integrated commit. Preserve all existing ignored-boundary execution in the full gates.

A final #264 evidence ledger records each named test's exact path, case/category coverage, command,
commit/tree and observed result, distinguishes helper/preflight/full-verifier/runtime strength,
and lists any unimplemented case as a blocker. Staged partial checkpoints may merge under their own
scope but cannot close #83. No target runtime, public profile, three-target equivalence or website
deployment is implied; public activation still requires #89/#90.
