# Complete enum matching closure matrix

## Closure status

- Issue: #273
- Integrated children: #333, #334, #335
- Status: compiler-only closure candidate
- Parent remaining: #269

The rows below bind the complete protocol-v4 enum-match boundary to authenticated source lowering
and independently constructed Universal IR. They are parsed exactly by
`complete_enum_matching_matrix_binds_exact_test_inventory_and_boundaries`; changing a row, test
name, child inventory, status, or retained boundary fails the focused semantics suite.

| Requirement | Source evidence | Independent IR evidence |
| --- | --- | --- |
| Exhaustive variants, canonical ordinals, one scrutinee evaluation, and active binding | `structured_match_exhausts_payloadless_and_mixed_payload_variants`; `structured_match_arm_order_is_canonical_and_scrutinee_is_consumed_once`; `structured_match_invalid_arm_sets_reject_exactly_replay_and_recover` | `nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup`; `nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload` |
| Owned aggregate and container results through nested continuations | `structured_match_owned_payloads_join_one_result_and_continue`; `structured_nested_matches_transfer_only_each_active_payload` | `generic_enum_payload_move_transfers_complete_owned_result_and_masks_only_active_subtree`; `generic_enum_payload_move_old_enum_drop_precedes_complete_result_edge_transfer` |
| Constructor, call, and Vec continuation composition and failure cleanup | `structured_match_constructor_retains_earlier_operand_across_continuation`; `structured_match_call_retains_arguments_until_complete_then_transfers_before_trap`; `structured_match_vec_growth_cleanup_retains_both_completed_operands` | `nested_multi_arm_match_seals_refinement_payload_transfer_join_and_cleanup` |
| Copy payload continuation restores one once-evaluated scrutinee | `structured_match_copy_payload_continuation_retains_surrounding_owned_parameter` | `copy_enum_join_restores_private_temporary_with_original_once_evaluated_ssa`; `copy_enum_join_omitting_restoration_rejects_unequal_arm_refinements`; `copy_enum_join_write_requires_its_active_exclusive_authority_and_exact_type` |
| Refinement dominance and inactive or foreign payload rejection | `structured_match_bad_arm_values_reject_and_replay_exact_diagnostics` | `nested_match_rejects_wrong_ordinal_absent_refinement_and_foreign_or_inactive_payload`; `generic_enum_payload_move_rejects_wrong_variant_and_absent_refinement` |
| Moved, partial, repeated, cross-arm, join, and cleanup forgery rejection | `structured_nested_matches_transfer_only_each_active_payload` | `nested_match_rejects_moved_partial_repeated_and_cross_arm_ownership`; `nested_match_rejects_incompatible_join_and_active_or_rest_cleanup_forgery`; `generic_enum_payload_move_rejects_forged_paths_types_and_cross_arm_result_use`; `generic_enum_payload_move_cleanup_is_exact_ordered_and_excludes_transferred_owner` |
| Exact and first-extra graph, value, place, transition, and cleanup resources with overflow recovery | `complete_enum_matches_bound_graph_resources_atomically_and_recover`; `complete_enum_match_value_place_transition_and_cleanup_resources_are_exact` | `generic_enum_payload_move_cleanup_preflight_exact_extra_and_checked_overflow_recover`; `copy_enum_join_restoration_transition_preflight_exact_extra_and_overflow` |

## Retained boundaries

- terminating match arms
- source-selected discriminants and wildcard arms
- implicit clone and inactive payload access
- stored or returned borrows and borrow-carrying CFG edges
- Issue #275 non-indexed active-payload borrowing
- runtime, backend, driver, CLI, artifacts, public ABI, and public profile activation
