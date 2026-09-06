# Internal owned-call closure matrix

This matrix binds Issue #272 to exact compiler-only source and Universal IR evidence. It does not
activate `data-ownership-v1` in a driver, CLI, backend, runtime, artifact, or public ABI. Indirect
calls, methods, callbacks, recursion, overloads, user generics, stored or returned borrows, the
additional match work in #273, and the lexical-access work in #275 remain excluded.

The `matrix_binding_is_complete_and_keeps_public_activation_excluded` test reads this file,
requires these exact ordered rows and exclusions, and resolves every binding to a real Rust
`#[test] fn` declaration. Renaming, moving, or removing evidence therefore fails the focused
`zryna-ir` suite instead of silently weakening the closure claim.

| Requirement | Source evidence after #329/#330 integration | Independent IR evidence |
| --- | --- | --- |
| Canonical same-module and named-import identity | `named_import_alias_retains_canonical_cross_module_identity_and_owned_cleanup`; `imported_producer_and_consumer_retain_one_foreign_nominal_identity` | `named_import_cross_module_function_ids_and_cleanup_are_verified_independently` |
| Mixed Copy and owned arguments evaluate once, left to right | `generic_calls_transfer_multiple_owned_and_copy_arguments_in_source_order`; `named_import_mixed_argument_producers_are_ordered_and_later_failure_retains_earlier_owners` | `cross_module_aggregate_container_and_handle_call_is_verified_independently` |
| Aggregate, container, and handle-bearing argument/result ownership | `imported_signature_accepts_the_complete_sealed_by_value_graph`; `payload_matrix_composes_handles_vec_calls_upgrade_and_nested_cfg` | `cross_module_aggregate_container_and_handle_call_is_verified_independently` |
| Foreign identity, signature, arity, order, result, owner, and partial-mask rejection | `imported_producer_and_consumer_retain_one_foreign_nominal_identity`; `generic_calls_reject_wrong_argument_type_and_repeated_owner_deterministically` | `cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup` |
| Exact caller trap, callee parameter, result, and structured cleanup | `named_import_alias_retains_canonical_cross_module_identity_and_owned_cleanup`; `payload_cfg_fallible_operations_keep_source_ordered_cleanup` | `cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup` |
| Cross-module cycle and static depth | `named_import_graph_rejects_a_cycle_at_the_closing_edge_exactly`; `named_import_target_still_obeys_the_direct_call_cycle_verifier` | `named_import_two_module_function_ids_do_not_bypass_call_cycle_verification`; `named_import_cross_module_static_depth_is_exact_and_first_extra_rejected` |
| Exact/first-extra call resources and checked overflow recovery | `named_import_preparation_resources_are_exact_atomic_overflow_checked_and_recoverable` | `cross_module_call_resource_preflight_and_checked_overflow_recover` |

Issue #272 closes only when #329, #330, and #331 are integrated together and the required
preflight, M0, M2, Linux, and Windows gates pass on that integrated revision.
