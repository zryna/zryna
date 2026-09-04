# M3 ownership composition evidence

Status: Issue #277 planned evidence and integration matrix, not execution or implemented generic
source capability. Read the [eight interface contracts](M3_OWNERSHIP_COMPOSITION.md) and
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

This is only a prerequisite to C2, not completed #278 or generic composition. Complete read-only
child-cost/effect planning before any child mutation remains REQUIRED, including exact cleanup
plans/actions, pending-owner order, projection/mask effects, retention, failure prefixes and
all supported nested operations. Releasing a commit ticket does not roll back already prepared
children or prove their combined capacity. Existing per-child cleanup and failure behavior remains
in force until that separately reviewed planning/commit unit is implemented and verified.

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
