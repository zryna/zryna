# M3 generic owned composition closure matrix

Status: Issue #320 froze the checked plan for parent #270. Issues #321, #322 and #323 now supply
the previously missing bounded implementation evidence, so this document reconciles the complete
internal #270 type/operation matrix. It adds no runtime, backend, driver route or public profile.

Read this with the [ownership composition contract](M3_OWNERSHIP_COMPOSITION.md), the
[composition evidence map](M3_OWNERSHIP_COMPOSITION_EVIDENCE.md), the
[generic clone core](M3_GENERIC_CLONE_CORE.md), the
[generic Vec authority](M3_GENERIC_VEC_OPERATIONS.md), and the completed
[Shared/Weak authority](M3_SHARED_WEAK_AUTHORITY.md). Paths are repository-relative and test names
are exact Rust `#[test]` functions, not proposed aliases.

## Evidence classes

- `source+IR`: authenticated protocol-v4 source reaches mandatory verified DataOwnershipV1.
- `hostile IR`: an independently valid raw program is mutated at the named verifier boundary.
- `planner control`: checked held-credit/exact/first-extra/overflow evidence; not a giant source.
- `symbolic fault`: compiler/ABI failure and cleanup traces; not target allocation or destruction.
- `integrated child`: authenticated source, hostile IR and bounded resource/replay evidence supplied
  by one of #321–#323 and checked here by exact path and test name.

A matrix label supplies no executable evidence by itself; no executable evidence may be claimed
unless its exact binding below resolves.

The focused commands are `pnpm m3:data:quick` for semantic source/planner rows,
`cargo test --locked -p zryna-ir data_ownership_v1` for independent IR rows, and
`pnpm m3:runtime-abi:quick` only when the completed #83 handle transition authority is relevant.
Passing those commands does not convert compiler evidence into target execution.

## Frozen type/operation matrix

`Whole transfer` includes a complete local move and complete root replacement. `Static transfer`
means a Struct-field or constant FixedArray subobject move/replacement. `Ordinary Vec element`
means transient checked observation, exact replacement and prepared push; it is not the persistent
indexed-borrow producer from #255/#256.

| Payload graph | Construction | Whole transfer | Static transfer | Structural clone | Ordinary Vec element |
| --- | --- | --- | --- | --- | --- |
| Nested non-handle Struct | `E1` | `E2` | `E3` | `E4` | `E5` |
| Nested non-handle Enum, every active variant | `E1` | `E2` | `E3` | `E4` | `E5` |
| Zero/nonzero FixedArray with non-handle elements | `E1` | `E2` | `E3` | `E4` | `E5` |
| Empty/nonempty positive-stride Vec with non-handle elements | `E1` | `E2` | `E3` | `E4` | `E5` |
| Finite values through legal non-handle Vec indirection recursion | `R1` | `R1` | `R1` | `R1` | `R1` |
| Direct Shared/Weak | `H1` | `H1` | `H3` | `H2` | `H4` |
| Struct/Enum/FixedArray/Vec containing Shared/Weak leaves | `H1` | `H1` | `H3` | `H2` | `H4` |
| Finite values through legal Shared/Weak indirection recursion | `H1` | `H1` | `H3` | `H2` | `H4` |

This matrix does not broaden clone capability. `E4` is the generic non-handle structural clone;
`H2` is the distinct #83 handle-aware clone whose Shared/Weak leaves perform explicit count
operations. Transfer eligibility is independent from clone eligibility. `H3` therefore reuses
generic transfer semantics without admitting handle graphs through `GenericClonePlace`.

## Exact existing evidence bindings

Each existing key has authenticated source, independent verifier evidence, and bounded
failure/resource/replay coverage. Multiple tests are named where those proof classes are separate.

| Key | Exact source evidence | Independent/hostile evidence | Failure, resource, diagnostic and recovery evidence |
| --- | --- | --- | --- |
| `E1` | `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_construction.rs` / `mixed_construction_vec_of_owned_struct_reaches_verified_ir`, `mixed_construction_owned_struct_containing_vec_reaches_verified_ir`; `crates/zryna-semantics/src/data_ownership_v1/tests/nested_mixed_construction.rs` / `mixed_nested_selected_enum_vec_payload_reaches_verified_ir`; `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_recursive_vec.rs` / `mixed_recursive_vec_indirection_constructs_finite_empty_child_through_full_ir` | `crates/zryna-ir/src/data_ownership_v1/tests/mixed_constructor_authority.rs` / `mixed_raw_constructor_mutations_fail_at_exact_authority_phase` | `crates/zryna-semantics/src/data_ownership_v1/tests/constructor_child_preparation_matrix.rs` / `constructor_child_matrix_nested_array_later_child_is_atomic`, `constructor_child_matrix_valid_nested_sources_replay_through_full_verifier`; `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_constructor_faults.rs` / `mixed_nested_vec_and_selected_enum_faults_preserve_completed_children` |
| `E2` | `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement.rs` / `mixed_root_replacement_constructors_moves_and_repeated_commits_reach_verified_ir`; `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_struct_whole_moves.rs` / `mixed_owned_struct_local_moves_once_into_outer_vec_with_exact_cleanup` | `crates/zryna-ir/src/data_ownership_v1/tests/mixed_replacement_authority.rs` / `mixed_replacement_hostile_mutations_reject_deterministically_after_valid_control` | `crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement_controls.rs` / `mixed_root_replacement_final_transition_exact_and_first_extra_preserve_state`, `mixed_root_replacement_invalid_targets_and_self_moves_preserve_prior_statements` |
| `E3` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs` / `generic_static_source_subtree_clone_and_move_keep_exact_parent_masks`, `generic_static_source_repeated_and_self_clone_replacements_retain_target_until_commit` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer.rs` / `generic_static_transfer_rejects_wrong_referent_and_unavailable_move`, `generic_static_transfer_rejects_reused_rhs_and_a_hole_at_the_commit_target` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs` / `generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional`, `generic_static_move_resources_exact_extra_overflow_and_recovery_do_not_leak_masks`; `crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs` / `generic_static_source_partial_root_and_self_consuming_rhs_are_stable_rejections` |
| `E4` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_source.rs` / `generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_clone_hostile.rs` / `generic_clone_forbidden_descendants_propagate_through_nested_and_recursive_graphs`, `generic_clone_rejects_forged_recursive_prefix_owner_shape_order_and_site` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_resources.rs` / `generic_clone_resource_exact_first_extra_preserve_source_state_credits_and_recovery`, `generic_clone_missing_source_precedes_deferred_cleanup_capacity` |
| `E5` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source.rs` / `generic_vec_source_owned_observations_clone_exact_elements_and_retain_container_cleanup`, `generic_vec_source_owned_replacement_checks_bounds_before_rhs_and_keeps_old_container_on_failure`; `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_push_source.rs` / `generic_vec_push_source_prepares_owned_elements_before_growth_and_retains_exact_failure_owners` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_hostile.rs` / `generic_vec_observation_rejects_inactive_foreign_and_wrong_referent_authority`, `generic_vec_observation_rejects_wrong_prefix_source_alias_and_premature_result_cleanup` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs` / `generic_vec_replacement_resource_exact_first_extra_reserves_bounds_rhs_commit_and_end`; `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source_rejections.rs` / `generic_vec_source_owned_bare_index_never_moves_an_element_or_leaves_a_hole`, `generic_vec_source_replacement_blocks_same_container_clone_before_nested_index_evaluation` |
| `R1` | `crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_composition.rs` / `finite_recursive_composition_reaches_verified_ir`; `crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_projections.rs` / `finite_recursive_static_move_and_replacement_bind_exact_owners_masks_and_commit`, `finite_recursive_vec_observation_and_replacement_bind_borrows_and_exact_owners` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs` / `recursive_construction_rejects_forged_variant_element_and_cleanup_then_recovers`, `recursive_whole_move_and_root_replacement_reject_identity_and_cleanup_forgery`, `recursive_static_move_and_replacement_reject_path_owner_and_cleanup_forgery`, `recursive_vec_observation_replacement_and_push_reject_forged_authority`, `recursive_clone_rejects_wrong_identity_prefix_and_cleanup_order_then_recovers` | `crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_resources.rs` / `finite_recursive_clone_and_push_resources_are_exact_atomic_and_recoverable`; `crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_resources.rs` / `recursive_clone_resource_preflight_is_exact_checked_and_replay_stable` |
| `H1` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs` / `shared_and_weak_structural_payload_categories_lower_through_verified_ir`, `handle_leaves_compose_through_struct_array_vec_projection_and_replacement`; `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_payload_closure.rs` / `recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay` | `crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs` / `shared_weak_instruction_types_reject_wrong_payload_and_handle_categories` | `crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs` / `direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup`; #83's checked ledger retains exact resource/diagnostic/replay mappings |
| `H2` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs` / `structural_handle_clone_seals_struct_enum_array_and_vec_count_recipes` | `crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs` / `seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles`, `rejects_non_handle_roots_and_inexact_prefix_cleanup_deterministically` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs` / `handle_aware_structural_clone_has_exact_cleanup_frontier_and_pristine_retry`; `crates/zryna-semantics/src/data_ownership_v1/tests/handle_frontier_source.rs` / `handle_frontier_source_vec_enum_occurrences_retain_replacement_owners` |
| `H3` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs` / `generic_static_source_subtree_clone_and_move_keep_exact_parent_masks`, `generic_static_source_repeated_and_self_clone_replacements_retain_target_until_commit`; `crates/zryna-semantics/src/data_ownership_v1/tests/handle_static_source.rs` / `handle_static_source_rejects_wrong_rhs_repeated_move_and_moved_target` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs` / `handle_static_transfer_without_clone_preserves_recursive_masks_and_owner_identity`, `handle_static_transfer_replacement_retains_old_target_through_count_failure`, `handle_static_transfer_rejects_exact_type_borrow_and_consumption_forgery_then_recovers`, `handle_static_transfer_rejects_repeated_move_forged_path_and_omitted_parent_cleanup`, `handle_static_transfer_rejects_partial_target_and_wrong_move_result` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs` / `handle_static_transfer_resource_preflight_exact_extra_overflow_and_recovery`; `crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs` / `generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional`, `generic_static_move_resources_exact_extra_overflow_and_recovery_do_not_leak_masks` |
| `H4` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_source.rs` / `generic_vec_handle_observation_clones_counts_and_retains_the_complete_container`, `generic_vec_handle_replacement_checks_bounds_then_commits_one_exact_owner`, `generic_vec_handle_push_prepares_once_and_transfers_only_after_success`, `generic_vec_handle_source_replays_identically` | `crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_handle_hostile.rs` / `generic_vec_handle_ir_rejects_stale_or_foreign_borrow_then_recovers`, `generic_vec_handle_ir_rejects_wrong_result_and_cleanup_identity_then_recovers` | `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_rejections.rs` / `generic_vec_handle_bare_observation_rejects_implicit_move_and_recovers`, `generic_vec_handle_replacement_rejects_overlap_before_nested_index_effects`, `generic_vec_handle_replacement_rejects_wrong_exact_owned_type_and_recovers`; `crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs` / `generic_vec_handle_push_resource_exact_first_extra_overflow_and_recovery`, `generic_vec_place_handle_push_clone_has_exact_direct_and_structural_resource_frontiers`, `generic_vec_place_handle_push_clone_overflow_replays_and_recovers` |

The existing `H1` whole-transfer evidence is bounded to complete owners and roots. `H3` and `H4`
bind the distinct static-subobject and ordinary Vec-element implementations without widening `H1`
or the handle-count contract. Likewise, #255/#256
`indexed_handle_source` evidence proves explicit transient/lexical authority and `BorrowReplace`;
it is not used as #322 ordinary Vec source evidence.

## Implemented child integration

| Key | Owning issue | Integrated executable boundary |
| --- | --- | --- |
| `H3` | #321 | Static Struct-field/constant FixedArray subobject move, replacement and return for handle-containing graphs reuse generic transfer semantics while explicit handle-aware clone preparation retains count and cleanup authority. |
| `H4` | #322 | Ordinary Vec observation, replacement and prepared push cover direct and nested handle-bearing elements with authenticated ordering, retained-container cleanup, hostile IR and bounded resource/replay evidence. |
| `R1` | #323 | Finite nonempty recursive values through legal Vec indirection cover construction, whole/static transfer, clone and ordinary Vec operations without permitting by-value recursion. |

#321 and #322 reuse #278 operations and completed #83 handle authority; neither adds a competing
count, clone, borrow, cleanup or runtime model. #323 reuses the existing operation meanings and
completes only the finite recursive source/IR matrix under `R1`.

## Preserved sibling and downstream boundaries

- #271's separate checked structured-control-flow matrix now makes it a compiler-only closure
  candidate; this #270 matrix does not independently claim that closure.
- #272 retains complete internal owned calls and inherited named-import resolution. Existing
  bounded generic/handle calls do not close its multi-argument/result/module matrix.
- #273 retains exhaustive enum match and active-payload composition beyond existing bounded cases.
- #274 remains the separately completed ordinary dynamic FixedArray authority; this document does
  not duplicate or weaken #254/#255 indexed ownership.
- #275 non-indexed owned lexical borrowing is outside this matrix and completed separately. #255/#256 retain dynamic array/Vec borrowing;
  no persistent alias, borrowed move-out or borrow-carrying CFG edge is added here.
- #269 remains the parent integration gate after #270–#275. #320 remains planning provenance and
  does not by itself close #270, #269 or any target issue.

Still excluded are user generics, by-value recursive layout, zero-sized Vec elements, implicit
clone, Vec pop, element move-out, holes, raw pointers, tracing GC, runtime allocation/refcount/drop,
backend lowering, target execution, driver/CLI/manifest changes, public aggregate ABI, public
`data-ownership-v1` activation, three-target equivalence and website publication. Required full
suites, ignored proofs, preflight, M0/M2 and Linux/Windows CI remain merge gates for later
implementation; listing existing tests here is not a new execution receipt.

## Issue #270 final reconciliation

| Acceptance | Checked result |
| --- | --- |
| Complete normative matrix | `E1`–`E5`, `H1`–`H4` and `R1` bind every admitted type/operation cell to exact executable tests |
| Handle delegation | `H1`–`H4` delegate Shared/Weak count/clone/release meaning to completed #83 |
| Machine-checked paths/names | `tests/m3-issue-graph-cases.mjs` resolves every exact Rust path and `#[test]` name and rejects matrix/key drift |
| Exclusions | The sibling/downstream section retains #271–#275, #269, runtime, targets and public activation |
| Focused gates | Run `node --test tests/m3-issue-graph-cases.mjs`, `pnpm m3:contract`, `pnpm docs:check` and `pnpm structure:check`; report only observed results |

The integrated compiler evidence is a #270 closure candidate when this checked matrix and its
focused gates pass. Required ignored proofs, full semantic/IR/ABI regressions, preflight, M0/M2,
independent review and hosted Linux/Windows CI remain final merge/closure gates; this document is
not an execution receipt for any unobserved gate.
