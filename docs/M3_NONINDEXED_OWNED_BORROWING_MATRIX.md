# M3 non-indexed owned borrowing closure matrix

This matrix closes the compiler-only source and independently verified IR boundary tracked by
#275. It composes the existing bounded borrowing authority with owned roots, static projections,
active enum payload refinement, and direct lexical calls. It does not activate a public profile or
claim target execution.

| Requirement | Source evidence | Independent IR evidence |
| --- | --- | --- |
| Owned roots and static Struct/FixedArray places retain exact ownership, masks, overlap, clone, replacement, and recovery | `nonindexed_static_owned_borrow.rs::nonindexed_owned_struct_and_array_places_clone_replace_and_restore`; `nonindexed_static_owned_borrow.rs::nonindexed_owned_static_places_reject_partial_wrong_mode_and_wrong_type_then_recover`; `nonindexed_static_owned_borrow.rs::nonindexed_owned_parent_and_subobject_borrows_overlap_exactly` | `nonindexed_owned_borrow_proof.rs::nonindexed_owned_root_and_static_subobject_borrows_have_independent_ir_authority`; `nonindexed_owned_borrow_proof.rs::nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers`; `nonindexed_owned_borrow_proof.rs::nonindexed_owned_static_place_and_lifetime_forgery_replay_deterministically`; `tests.rs::direct_call_accepts_two_exclusive_disjoint_projected_authorities`; `tests.rs::projected_borrow_does_not_change_owner_masks_or_cleanup` |
| Refined active enum payloads preserve exact variant, referent, parent cleanup, replacement, and lexical restoration | `enum_match_payload_borrow_source.rs::exhaustive_match_arms_borrow_only_the_refined_active_owned_payload`; `enum_match_payload_borrow_source.rs::inactive_match_payload_borrow_rejects_deterministically_then_recovers`; `active_enum_payload_borrow_source.rs::refined_active_enum_payload_shared_and_exclusive_borrows_restore_parent`; `active_enum_payload_borrow_source.rs::refined_enum_payload_borrow_rejects_inactive_and_foreign_variants_then_recovers` | `enum_match_payload_borrow_ir.rs::enum_match_arm_borrow_is_bound_to_exact_variant_region_and_cleanup`; `enum_match_payload_borrow_ir.rs::enum_match_payload_borrow_rejects_variant_nominal_region_and_cleanup_forgeries`; `active_enum_payload_borrow.rs::active_enum_payload_borrow_seals_refinement_mode_replacement_and_parent_cleanup`; `active_enum_payload_borrow.rs::enum_payload_borrow_rejects_inactive_foreign_type_and_mode_forgeries_then_recovers`; `active_enum_payload_borrow.rs::enum_payload_borrow_rejects_moved_overlap_disjoint_and_ended_authority_then_recovers` |
| Nested lexical and direct-call use neither clones authority nor permits escape and restores the exact owner | `nonindexed_borrow_calls.rs::nested_nonindexed_lexical_call_preserves_authority_and_restores_owner`; `nonindexed_borrow_calls.rs::nested_nonindexed_call_does_not_clone_authority_or_fabricate_owned_results`; `nonindexed_static_owned_borrow/calls.rs::static_owned_borrows_pass_shared_and_exclusive_authority_to_direct_calls`; `active_enum_payload_borrow_fixture/calls.rs::refined_payload_borrows_pass_shared_and_exclusive_authority_to_direct_calls` | `borrow_nonindexed_call_scope.rs::nonindexed_lexical_call_is_verified_as_nonescaping_authority`; `borrow_nonindexed_call_scope.rs::nonindexed_call_rejects_inactive_wrong_region_and_escape_then_recovers`; `nonindexed_projection_call_scope.rs::static_struct_and_array_projection_calls_preserve_lexical_authority`; `nonindexed_projection_call_scope.rs::refined_enum_payload_calls_preserve_lexical_authority`; `nonindexed_projection_call_scope.rs::projection_calls_reject_wrong_region_and_repeated_exclusive_then_recover`; `nonindexed_projection_call_scope.rs::enum_payload_calls_reject_wrong_region_and_repeated_exclusive_then_recover` |
| Exact and first-extra resource dimensions, overflow, atomic rejection, and deterministic replay are checked | `nonindexed_borrow_resource_frontiers.rs::static_and_active_enum_borrow_lowering_hit_exact_transition_capacity_and_recover`; `nonindexed_borrow_resource_frontiers.rs::exhaustive_match_payload_borrow_clone_hits_exact_resource_costs_and_first_extra_recovers`; `nonindexed_borrow_resource_frontiers.rs::exhaustive_match_payload_borrow_clone_overflow_is_atomic_and_recovers`; `nonindexed_borrow_resources.rs::nonindexed_owned_borrow_resource_dimensions_accept_exact_and_reject_first_extra`; `nonindexed_borrow_resources.rs::nonindexed_owned_borrow_resource_overflow_is_checked_and_recovery_is_stable`; `lexical_borrow_calls.rs::borrow_call_resource_preflight_accepts_exact_limits_and_rejects_first_extra_in_order`; `lexical_borrow_calls.rs::borrow_call_resource_overflow_precedes_limit_selection_and_preserves_authority_cost`; `structured_graph_resources.rs::complete_enum_matches_bound_graph_resources_atomically_and_recover` | `nonindexed_owned_borrow_proof.rs::nonindexed_owned_borrow_places_reach_exact_limit_and_reject_first_extra`; `borrow_resource_boundaries.rs::dense_lexical_active_borrow_exact_and_first_extra_are_fully_verified`; `nonindexed_owned_borrow_proof.rs::nonindexed_owned_root_static_overlap_rejects_atomically_and_recovers`; `active_enum_payload_borrow.rs::enum_payload_borrow_rejects_moved_overlap_disjoint_and_ended_authority_then_recovers`; `borrow_nonindexed_call_scope.rs::nonindexed_call_rejects_inactive_wrong_region_and_escape_then_recovers` |

The proportional ignored gate
`borrow_resource_boundaries.rs::sequential_nonindexed_borrows_reach_the_exact_transition_limit_and_reject_first_extra`
constructs and verifies the complete 262,144-transition exact and first-extra programs. It is run
by the required include-ignored closure suite, but is not used as ordinary enabled matrix evidence.

## Retained boundaries

- Dynamic-index FixedArray and Vec borrowing remains owned by #254–#256 and #274, not this matrix.
- Stored, returned, or captured borrows and borrow-carrying branch or loop edges remain rejected.
- Implicit lifetime shortening, arbitrary reborrowing, moves through live borrows, interior mutability, and raw pointers remain rejected.
- Borrowed imports, nominal type-import grammar, indirect calls, callbacks, recursion, wildcard arms, and terminating match arms remain outside this boundary.
- Runtime no-alias checks, backends, driver and CLI routes, artifacts, public ABI/profile activation, and target execution remain unavailable.

## Closure status

- #337 supplies owned-root and static-projection authority.
- #338 supplies refined active-enum payload authority.
- #339 supplies lexical and direct-call composition.
- #340 supplies hostile IR, resource, documentation, and checked-matrix evidence.
- #275 is a compiler-only closure candidate; parent #269 remains open.
