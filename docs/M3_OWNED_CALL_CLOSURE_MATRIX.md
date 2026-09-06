# Internal owned-call closure matrix

This matrix binds Issue #272 to exact compiler-only source and Universal IR evidence. It does not
activate `data-ownership-v1` in a driver, CLI, backend, runtime, artifact, or public ABI. Indirect
calls, methods, callbacks, recursion, overloads, user generics, stored or returned borrows, the
additional match work in #273, and the lexical-access work in #275 remain excluded.

The `matrix_binding_is_complete_and_keeps_public_activation_excluded` test reads this file and
requires every row and exclusion token below. A renamed or removed evidence binding therefore
fails the focused `zryna-ir` suite instead of silently weakening the closure claim.

| Requirement | Source evidence after #329/#330 integration | Independent IR evidence |
| --- | --- | --- |
| Canonical same-module and named-import identity | `imported_owned_signatures_preserve_canonical_function_and_type_identity`; `owned_calls_resolve_same_module_and_named_import_targets` | `named_import_cross_module_function_ids_and_cleanup_are_verified_independently` |
| Mixed Copy and owned arguments evaluate once, left to right | `mixed_owned_call_arguments_evaluate_left_to_right_once`; `later_argument_failure_preserves_prepared_owners` | `cross_module_aggregate_container_and_handle_call_is_verified_independently` |
| Aggregate, container, and handle-bearing argument/result ownership | `imported_aggregate_container_and_handle_signatures_are_admitted`; `structured_owned_call_result_has_one_owner` | `cross_module_aggregate_container_and_handle_call_is_verified_independently` |
| Foreign identity, signature, arity, order, result, owner, and partial-mask rejection | `imported_owned_signature_forgeries_are_rejected`; `owned_call_transfer_forgeries_are_rejected` | `cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup` |
| Exact caller trap, callee parameter, result, and structured cleanup | `owned_call_cleanup_is_exact_across_straight_line_branch_and_loop`; `owned_call_failure_replay_recovers` | `cross_module_owned_call_forgeries_reject_identity_signature_order_owner_mask_and_cleanup` |
| Cross-module cycle and static depth | `import_cycle_and_call_cycle_diagnostics_are_stable` | `named_import_two_module_function_ids_do_not_bypass_call_cycle_verification`; `named_import_cross_module_static_depth_is_exact_and_first_extra_rejected` |
| Exact/first-extra call resources and checked overflow recovery | `owned_call_resource_boundaries_and_recovery_are_exact` | `cross_module_call_resource_preflight_and_checked_overflow_recover` |

Issue #272 closes only when #329, #330, and #331 are integrated together and the required
preflight, M0, M2, Linux, and Windows gates pass on that integrated revision.
