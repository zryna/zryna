# M3 Shared and Weak evidence matrix

Status: Issue #259 test/interface plan. **Planned tests below are not existing execution evidence.**
Read the [authority contract](M3_SHARED_WEAK_AUTHORITY.md) for SW1–SW5, complete payload domain,
operation semantics and exclusions. A scalar-only checkpoint cannot discharge #83.

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
