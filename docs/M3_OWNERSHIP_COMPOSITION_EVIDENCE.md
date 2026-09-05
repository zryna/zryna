# M3 ownership composition evidence

Status: Issue #277 planned generic evidence and integration matrix, with the internal #278
constructor-preparation and #296 mixed-construction work described below; not implemented generic
source capability.
Read the [eight interface contracts](M3_OWNERSHIP_COMPOSITION.md) and
[Shared/Weak evidence](M3_SHARED_WEAK_EVIDENCE.md). Normative language/ABI authority remains
unchanged. Existing tests below are located evidence, not newly executed by this document.

## Existing authority and limits

Paths below are relative to `crates/zryna-semantics/src/data_ownership_v1/` at
the merged authority baseline `8cc4eed8d522976ca557a27ea54993fb0d5ebf1c`, except the explicitly
named IR file. The declarations and named tests below are also checked against this candidate.

| Existing declaration | Reuse and missing interface |
| --- | --- |
| `type_model.rs::map_node_types` | Sealed identity mapping; does not map Shared/Weak or establish general recursive-indirection mapping |
| `owner_state.rs::OwnerState` and `OwnerDelta` | Pending order/value association/effects; not complete masks, variants, borrow or CFG provenance |
| `owned_lowering_resources.rs::OwnedCleanupAccounting` | Exact excluded owner and reserved cleanup costs; needs generic site/shape composition |
| `owned_cfg_state.rs::OwnedCfgState` | Dense arenas/reservations; `finish` requires upgrade-success adaptation, not a substitute for independent IR |
| `owned_vec_lowering/expressions_and_calls.rs::direct_call` | Ordered preparation and trap transfer; current producer/identity-call subset is not arbitrary owned calls |
| `owned_enum_payload_move.rs` | Narrow refined payload/local continuation, not arbitrary owned matches |
| `crates/zryna-ir/src/data_ownership_v1.rs::transfer_edge_owners` | Independent synthesized-success owner and ordinary argument transfer; preserve its exact authority |

This candidate includes the #233–#237 extractions and the #288 constructor-order correction.
`function_dispatch.rs` now exists in this candidate; proposed generic modules remain unimplemented.
This describes the candidate tree, not the merge status of its prerequisite pull requests.
Resolve final registration paths against the integrated parent before implementation.

## Staged constructor commit envelope (#278)

The private Struct, Enum and fixed-array constructor adapters now reserve their own final
commit capacity after complete outer type/shape/field mapping and before child materialization.
The checked precedence is aggregate operands, transition, result value, then result place
(zero places for a Copy result). Parent-capacity rejection intentionally precedes child
diagnostics at the constructor span. When capacity is available, declaration/index evaluation
order and existing semantic diagnostics are unchanged. Vec retains its separate existing
fallible construction reservation; source admission, IR, ABI and limits are unchanged.

The aggregate route's private ordered-expression decisions separate source/type validation from
materialization. Struct field types still resolve one child at a time under the constructor
reservation; array element and selected enum payload types resolve before it. The shared
classifier is used by the child-preparation candidate below without admitting additional source
forms or public capability.

Shared private operand decisions also retain the existing binding diagnostics, ancestry/mask
availability rules and clone resource-check order. Direct projection APIs still materialize
prefixes before contextual failures, and projected String clone budgets include those prefixes.
Complete child preparation applies those same decisions to scratch topology instead: a rejected
child tree publishes no new real prefix. These are distinct call boundaries, not changed
projection diagnostic precedence.

Shared aggregate resource decisions read committed and held counters without issuing credits or
emitting identities. Constructor and ordinary-emission wrappers retain their different check
orders, including the absence of a place check for Copy emission. The live constructor ticket
remains responsible for checked acquisition and release; these views are not an immutable C2 plan.
The ordered check chains now share those same decisions, while one borrowed credit ledger applies
the existing constructor and assignment arithmetic to live or isolated scratch counters. Storage,
panic semantics and final-commit order are unchanged. The child planner uses the same ledger on
isolated counters; it does not acquire a second whole-subtree ticket in addition to nested tickets.

The affine ticket releases its own credits in reverse order on child failure and immediately
before final commit on success, preserving ancestor and assignment credits. Ordinary emission,
whole/projected clone, projection topology materialization, compound assignment and direct
partial-transfer preflights all account for held capacity. Committed arena counts still determine
identities. Infallible aggregate commit reserves no fictional cleanup actions or plans.

The `aggregate_constructor_envelope.rs` and `aggregate_constructor_envelope_flows.rs` tests
separate injected-counter acquisition/frontier/unwind checks from authenticated source fixtures
and independent full-IR valid replay. Injected held counts are not claims that every independent
limit is reachable from dense source. These tests are source additions, not execution claims in
this document; the handoff must report actual commands and results.

## Internal child preparation and consumption candidate (#278)

The aggregate `value` entry now prepares the complete currently admitted child tree before
materializing any of its children. One iterative walker uses the shared classifier and keeps a
single scratch owner state, canonical projection map, moved/partial masks, checked resource
counters and constructor credits. Existing place metadata is borrowed; planned places are metadata,
not speculative raw instructions. A private value-type snapshot observes the pending instruction
suffix without advancing the real cache, then appends predicted result definitions. Instruction
and value cursors remain distinct when earlier instructions have no result.

Preparation rejection preserves the real instruction/place/cleanup arenas, owners and pending
order, projection topology and masks, constructor cache contents/cursor, counters and outstanding
credits. Earlier successful statements remain intact. In the legacy complete-aggregate schedule,
the diagnostic comes from the first ordered semantic or resource failure; a later invalid name
cannot displace an earlier cleanup capacity failure. The new mixed-root schedule below explicitly
differs. This intentionally replaces the previous retained **compile-time** child
artifacts. It neither runs an allocator nor rolls back effects of an executing source program.

The private single-use plan holds an exclusive lowerer borrow through consumption. Selected
operations materialize without classifying or resolving source again. Release-build checks bind
actual result identities/types, ordered immediate child operands, nested constructor contracts,
cleanup IDs/roles and destination-prefix identity, resource effects, ordered per-operation owner
deltas and final owner/topology
state. Real constructor commits still observe the actual emitted value types and use the existing
prepared-constructor authority. Mandatory independent full IR verification remains separate and
cannot be replaced by a successful plan check.

The prerequisite child vocabulary is Bool/i32/String literals, Copy and whole-value references,
static Copy/String projections, projected String clone, named whole aggregate clone, and nested
Struct/FixedArray/selected Enum constructors. Context-only projected aggregate transfers/clones,
Vec operations, handles, calls, borrowing and CFG were not admitted by that prerequisite alone.
The #296 extension below adds its stated constructor/read/call forms through the same authority.
This candidate
does not complete generic C2, #278, #83 or M3, and enables no runtime/backend/CLI/public profile.

Zero-length fixed arrays and payloadless Enum variants are distinct from zero-field Struct
declarations. The current protocol-v4 collection check rejects a zero-member data declaration
before source authentication and lowering. A raw empty-Struct rejection test documents that
boundary; it is not a successful prepared child or full-IR control, and this work does not change
the admission policy.

| Test source | Located coverage and limits |
| --- | --- |
| `constructor_child_preparation_red.rs` | Later-invalid child after a String literal or whole move; full real-state equality and exact diagnostics. The historical red cases must pass on the integrated implementation. |
| `constructor_child_preparation_matrix.rs` | Authenticated nested arrays and Enum-array payload; exact valid instruction shapes through independent full IR verification, deterministic replay, first-error precedence and later-error atomicity. |
| `constructor_preparation_types.rs` | Isolated cache observation, dense append/cursor checks and overflow rejection; helper tests, not target execution. |
| `constructor_preparation_consumption.rs` | Private malformed-plan result, cleanup, constructor contract, range and same-typed child-order rejection, with successful controls. Internal invariant panics are not a source-error rollback guarantee. |
| `constructor_preparation_controls.rs`, `constructor_preparation_control_fixtures.rs`, `constructor_preparation_copy_prefix.rs` | Source/IR positive controls for zero-length arrays, payloadless Enum variants, Copy parameters and bounded visits; fresh and prior-cached String-clone prefixes, plus cached Copy projection with a moved String sibling. Replays compare instruction-kind sequences, not complete IR structural equality. Empty-Struct protocol rejection is a separate negative control. |
| `constructor_preparation_cleanup_boundaries.rs` | Exact/first-extra cleanup-plan and cleanup-action limits, competing limits and overflow, with full rejection-state equality. Initial counters/plans are synthetic; these budget frontiers do not establish full-program IR validity or runtime allocation behavior. |
| `ordered_expression_decisions.rs`, `operand_projection_tests.rs` | Full rejected-child state equality while retaining earlier cleanup/name/field diagnostic precedence and unchanged direct projection behavior. |
| `aggregate_constructor_envelope.rs`, `aggregate_constructor_envelope_flows.rs` | Full failure-state equality with held parent/assignment credits, separate from authenticated successful shape checks and synthetic frontier limits. |

These are located source checks, not a substitute for a current execution receipt. Acceptance
still requires exact test inventory, independent review, resource/complexity and ignored-boundary
evidence, and the complete required Linux/Windows gates. Runtime failure-prefix cleanup and
three-target execution remain separate authorities; compile-time state preservation does not
prove them. The remaining generic operation, payload, call and CFG families below are not waived.

## Mixed non-handle construction (#296)

Private straight-line mixed owned results use the shared aggregate preparation authority for
Struct, selected Enum, fixed-array and Vec trees, including whole local moves. Each Vec seals its
own exact element identity; it does not inherit an unrelated function-wide element type. Type
mapping uses complete instantiated identities, including legal container indirections, rather
than relying on referenced types occurring earlier. Zero-member declarations, by-value cycles,
zero-stride Vec elements, source handles and inactive payload access remain rejected.

The selected mixed-root schedule prepares source/type/effect decisions before replaying deferred
resource checks. Struct children follow declaration order; array/Vec children follow ascending
index order. A later invalid name therefore precedes deferred capacity exhaustion in this new
schedule. Complete legacy aggregate roots retain their earlier interleaving, and standalone legacy
Vec operations retain their existing route. No failure triggers a fallback to a second evaluator.
Both schedules use the same decisions, checked credit ledger and affine consumption checks.

The contextual local-initializer entry is derived from the actual private mixed-result function;
it does not globally reclassify Vec roots or capture recursively Copy functions. Copy children
reuse the existing ten scalar operators and exact Bool/i32 semantics. Binary operands are prepared
left then right before operand-type validation. A mismatched scalar result in this new mixed
boundary reports M3007 at the complete expression, with `scalar result has a different exact
aggregate type` and `use a value with the exact declared type`. Existing Copy field diagnostics
are unchanged. A scalar result charges one value and one transition, not an owned place or cleanup.

Mixed-route local initialization extends that prepared result with one destination place and one
initialization transition, checked in that order after initializer resource replay and before real
consumption. Copy locals still require the destination place. Owned results retain their pending
slot while ownership and String byte facts move to the exact local identity. Capacity rejection
preserves arenas, bindings, local numbering, ownership, facts, cache and surrounding credits;
legacy local routes are unchanged. The separate C3 replacement slice is recorded below.

String clone/concat and supported private same-module String/Vec producer/identity calls use
their existing typed authorities. Nested scopes forward only their final immediate result.
Arguments transfer before caller CallTrap cleanup; original borrowed read owners remain retained.
Known zero bytes and Unknown bytes are distinct after actual availability/type checks; opaque
calls or aggregate projections cannot fabricate known lengths. This is not arbitrary mixed
signature calls, non-addressable aggregate cloning, general borrowed payloads or CFG composition.

| Test module group | Located bounded evidence |
| --- | --- |
| `mixed_construction.rs`, `nested_mixed_construction.rs`, `mixed_positive_arrays.rs`, `mixed_zero_array_vec.rs`, `mixed_recursive_vec.rs` | Authenticated source/full-IR nesting in both directions, selected payload, zero/nonzero array, empty/nonempty Vec and one finite recursive nominal value. |
| `mixed_local_construction.rs`, `mixed_array_whole_moves.rs`, `mixed_enum_whole_moves.rs`, `mixed_struct_whole_moves.rs` | Actual local-to-constructor whole moves, exact owner/result/cleanup identities, duplicate-source rejection and deterministic replay. |
| `local_commit_fixture.rs`, `local_commit_controls.rs`, `local_tail_supplement.rs`, `local_tail_supplement_controls.rs` | Authenticated source/full-IR local controls; exact and first-extra destination capacity, competing place/transition limits, semantic precedence, Copy destination cost and known String fact renaming. Both late-capacity regressions failed before the prepared local tail and pass with it. |
| `mixed_copy_operators.rs`, `scalar_operator_matrix.rs`, `scalar_matrix_negatives.rs`, `scalar_owned_lhs.rs` | All ten scalar operations, Bool/i32 equality, ordered nested operands, exact result mismatch and competing operand diagnostics through the existing checker. Owned-left/missing-right cases test inference order, not acceptance of owned scalar operands. |
| `scalar_private_controls.rs`, `scalar_resource_controls.rs` | Malformed private scope/order/type/range rejection; immediate nested results; exact/extra real held credits; semantic rejection preserves full state/facts. Impossible internal-counter overflow is separately labeled. |
| `mixed_type_negatives.rs`, `mixed_type_negative_controls.rs` | Exact distinct nominal, nested Vec element, outer context and selected-payload rejection; valid full-IR controls, complete state/facts with surrounding credit, and fixed nested scalar/call visit/result-step counts. |
| `mixed_string_read_scopes.rs`, `mixed_unknown_projected.rs`, `mixed_disjoint_owned_sibling.rs` and their controls | Literal/local/projected reads, disjoint owned sibling availability, exact cleanup, fresh/cached projection and whole-state rejection. |
| `mixed_string_calls.rs`, `mixed_call_string_nesting.rs`, `mixed_vec_calls.rs`, `mixed_vec_siblings.rs`, `mixed_call_unknown_clone.rs` | Actual supported signatures, distinct callee/argument/result linkage, local ownership, Unknown facts and nested read/call full-IR replay. |
| `mixed_string_call_rejections.rs`, `mixed_call_consumption_misuse.rs` | Exact name/case/signature diagnostics and private catalog/type/transfer/site/result corruption rejection; no arbitrary call-profile activation or post-panic rollback promise. |
| `mixed_phase_controls.rs`, `mixed_cleanup_frontiers.rs`, `mixed_call_resource_controls.rs`, `mixed_call_resource_order.rs`, `mixed_byte_facts.rs` | Parent/child order, surrounding credits, checked coupled resource costs, byte-fact effects and rejection-state controls. Synthetic counter frontiers are not giant valid source programs. |
| IR `mixed_constructor_authority.rs`, `mixed_enum_authority.rs` | Independently built valid raw-IR controls followed by isolated layout/type/owner/variant/site/missing-or-duplicate-cleanup mutations. Raw spans alone do not authenticate source syntax. |
| `mixed_constructor_faults.rs`, `mixed_read_faults.rs`, `mixed_two_element_faults.rs` | Existing fault authority over actual verified operation sites/statuses, retained inputs and reverse root cleanup, including preceding locals and String reads. |
| `recursive_cleanup_witness.rs` and its tests | Bounded constructor-provenance replay derives reverse completed children before Vec storage-release events, rejects foreign/duplicate/unknown provenance and preserves the original fault. |

The recursive witness is deliberately not an interpreter or an allocator. It accepts only its
whitelisted single-block executed constructor/literal prefix and complete temporary roots.
Local/call/mutation prefixes and partial masks are rejected, not guessed. Empty Vec storage release
is a logical no-op event, not proof of an allocation or free. Separate read-fault tests prove root
cleanup for their supported prefixes without claiming recursive storage replay or target execution.

Private corrupted-plan panics test release-build invariants, not rollback after internal misuse.
Full-state comparisons apply to rejected source preparation. Source/full-IR replay, forged IR,
resource helpers and fault traces remain distinct evidence classes. Named tests must actually run;
inventory counts or these descriptions cannot replace ordinary/required ignored tests, ABI/static
contracts, preflight, M0/M2 and required Linux/Windows CI before merge.

This child does not finish #278, #83 or public M3. Generic structural clone, generalized
initialization/replacement, generic Vec observations/replacement, mixed calls/CFG and Shared/Weak
production remain their following issues. Constructor commitment does not activate arbitrary
assignment RHS by itself; the separate bounded mixed-destination C3 slice below extends it.
No runtime/backend/CLI/profile is enabled, and
website publication must consume the authenticated successful main documentation artifact.

## Bounded C3 mixed-root replacement (#278)

`preparation_replacement_commit.rs` prepares a complete RHS through the shared mixed summary,
proves destination retention and a distinct replacement owner, checks the final transition, then
commits once. `mixed_shape.rs` selects only supported mixed target topologies in existing private
straight-line mixed functions. Legacy destinations and partial/projected routes are unchanged.

- `mixed_root_replacement.rs` authenticates actual Struct/Enum/FixedArray/Vec source snapshots,
  constructor/distinct-move/repeated replacement and mandatory verified IR. It checks exact old
  target cleanup, active enum changes, returned-owner exclusion and deterministic replay.
- `mixed_root_replacement_controls.rs` checks the final commit transition at exact/first-extra
  held-capacity frontiers and later semantic rejection ahead of deferred capacity checks. Full
  prior compiler state and preparation facts remain unchanged on rejection. These are checked
  external credit controls around real statements, not huge exact-limit source programs.
- IR `mixed_replacement_authority.rs` independently builds a valid Enum/Vec payload replacement
  before isolated type, owner, moved-destination, reused-RHS and cleanup corruptions. Its sealed
  observations prove old active-payload retention, one old-root commit drop, installed variant,
  pending completion order and exact replay. They are not allocator or recursive storage execution.

The constructor-only recursive witness remains unchanged and does not accept mutation prefixes.
General partial/projected mixed replacement, generic Vec element replacement/observation,
structural clone, handles and CFG integration remain open under #278 and its dependent issues.
This slice does not close #278 or activate public M3.

## Located tests, not complete composition proofs

Names below are actual `#[test]` functions under the same directory's `tests/`.

| File | Existing test | Proof limit |
| --- | --- | --- |
| `cfg_owner_state.rs` | `owner_state_rejects_duplicate_alias_and_self_rehome_without_mutation` | Helper atomicity, not authenticated generic source |
| `cfg_owner_state.rs` | `owned_cfg_value_ledger_is_atomic_for_parameters_blocks_and_results` | Arena/value ledger, not full ownership joins |
| `cfg_validation.rs` | `owned_cfg_finish_rejects_disconnected_cycles_and_edge_signature_mismatch` | Shape rejection, not source upgrade |
| `private_aggregate_lowering.rs` | `nested_owned_structs_consume_inner_owner_once_and_preserve_failure_cleanup` | Current nested aggregate slice |
| `enum_payloads.rs` | `owned_enum_struct_payload_moves_through_a_direct_local_continuation` | Current narrow continuation |
| `copy_calls.rs` | `source_faithful_copy_call_cycles_fail_closed_in_ir_authority` | Acyclic authority, not arbitrary owned calls |
| `lexical_borrow_calls.rs` | `lexical_borrow_call_evaluates_values_in_left_to_right_source_order` | Existing argument order |
| `lexical_borrow_calls.rs` | `lexical_borrow_call_owns_one_unique_call_trap_cleanup` | Existing call cleanup, not every payload |
| `partial_struct_transfers.rs` | `partial_struct_owner_returns_with_exact_topology_mask_and_survivor_cleanup` | Existing exact mask transfer, not arbitrary CFG masks |

## Proposed evidence families

Every name in this table is proposed, not an existing test or API. Each family requires positive,
hostile, failure/resource and deterministic valid-replay cases at its implementing owner.

| Proposed family | Owner and required observations |
| --- | --- |
| `generic_payload_identity_and_shape_matrix` | #278/#261: full C1 domain, nested containers/real handle leaves, legal finite indirections; wrong universe/nominal/layout and zero-stride Vec rejection |
| `generic_prepare_commit_failure_and_replay` | #278: every preparation/clone/replacement fault position; retained sources, reverse completed prefix/storage cleanup, ordered diagnostics and repeatable valid replay |
| `generic_vec_observation_replacement_no_holes` | #278/#261: ordinary observations/replacement, full required payloads, bounds and allocation failure; no holes or borrowed-element #256 claim |
| `generic_owned_call_arguments_results_and_traps` | #278/#279/#261: mixed owned arguments/results, left-to-right effects, preparation versus callee-trap transfer, borrow exclusions, acyclic calls |
| `generic_refined_match_continuation_state` | #278/#279/#261: exhaustive multi-arm nested payloads and nonterminal continuation; wrong variant, repeated consumption and unequal state rejection |
| `generic_nested_branch_loop_scope_cleanup` | #279/#262: nested/repeated branches/loops, body calls/matches/returns/traps, exact header restoration and lexical borrow discharge; no repair of mismatched state |
| `generic_upgrade_synthetic_success_signature` | #260/#279 then #262: producer-facing type/signature shape before CFG construction, not final ownership or runtime outcome; mandatory full IR verification seals +1 success parameter only, expired zero synthetic, exact type/dense identity and cleanup; separate bounded transition-model proof covers overflow neither edge and forged outcome/owner/replay rejection, not target execution |
| `generic_composition_resource_boundaries` | #278/#279/#263: checked values/places/blocks/edges/transitions/operands/drop actions/sites and program totals, exact/first-extra/overflow before dependent mutation and valid replay |

Source positives require authenticated syntax through mandatory independent IR verification.
Forged raw IR/ABI claims are separate adversarial evidence; helper arithmetic is not source proof.
Coupled limits may make a source maximum unreachable: derive that frontier honestly and identify
the independent verifier/ABI proof instead of claiming a synthetic helper is an authenticated
producer. No budget changes or new diagnostic numbers are authorized here. Target execution,
runtime fault/drop receipts and three-target equivalence remain separate later gates.

## Proposed file ownership and integration protocol

These filenames reserve review boundaries, not existing APIs or a demand to create every file.
Around 500 lines triggers a split/cohesion review, not a compulsory limit. Split where cohesive;
a larger cohesive module is allowed with a documented rationale. Minimize actual registration edits.

| Owner | Proposed scope | Exclusion |
| --- | --- | --- |
| #278 | `owned_operation_planning.rs`, `owned_operation_lowering/` children for constructors/projections/clone/replacement/calls | No handle count semantics, upgrade syntax or second verifier |
| #279 | `owned_cfg_state.rs` adapter, `owned_block_lowering/` scope/branch/loop/match/continuation children | No source handle operations or dependence on #262 |
| #260 | Separate bounded IR/ABI operation/control verification and hostile model fixtures | No generic source producer or executed runtime |
| #261 | `shared_weak_lowering/` expressions/type adapter and real handle leaves in #278 | No duplicate aggregate/Vec/drop core |
| #262 | `shared_weak_lowering/upgrade.rs`, narrow adapter to #279/#260 continuation/outcomes | No second CFG verifier or reusable upgrade ticket |

One designated integrator owns shared type mapping, `owner_state.rs`, `owned_cfg_state.rs`, cleanup
accounting and actual dispatcher/registration files at a time. Independent workers may own distinct
leaf modules/tests only after interfaces are frozen. Each handoff names exact base/commit, files,
inputs/outputs/failure invariants, prerequisite authorities, diagnostics/resources, actual test
commands/results and missing integration. No concurrent shared-file edits or guessed visibility.

#277 documentation drift checks bind real existing declarations, located tests and eight contract
sections, with missing-symbol/section mutations. They do not verify future APIs or execute Rust.
The frozen historical #277 checkpoint had 351 test names, including 2 ignored proportional tests.
Re-freeze the actual semantic inventory for every current executable change before handoff;
that historical count and earlier 345/346 refactor evidence are not current execution proof. Preserve
existing44borrowing evidence guards and required ignored execution through preflight. Every later
executable change requires independent source/IR review, relevant focused tests and complete
preflight/M0 on Linux and Windows; contract-only narrow checks cannot replace those merge gates.
