# M3 structured owned control-flow closure matrix

This matrix reconciles Issues #325, #326, and #327 as the compiler-only implementation evidence
for Issue #271. It covers authenticated protocol-v4 source lowering into mandatory
`DataOwnershipV1` verification and independent hostile raw-IR rejection. A verified instruction,
cleanup plan, branch, or fault trace is compiler evidence; it is not runtime execution, concrete
allocation/refcount mutation, backend lowering, or an observed target outcome.

## Frozen acceptance matrix

| Key | Owner | Checked boundary |
| --- | --- | --- |
| `S1` | #325 | One shared structured route covers lexical Block, If with explicit or omitted else, While, WeakUpgrade and admitted Match occupants across nested/repeated flow, early return, loop return and reachable post-loop continuation. |
| `S2` | #325 | Nearest lexical shadowing, mutable Copy assignment, source order, exact join/backedge state, unreachable/type/mutability rejection and deterministic replay remain explicit. |
| `B1` | #325 | Root-owned structured routing preserves established private String/Vec branch and loop boundaries; nested arena statements cannot steal a legacy route. Terminal owned `if` replaces the former join shape with exactly three entry/then/else blocks, direct Return in each arm, and no join parameter. |
| `I1` | #326 | Independent raw IR seals mixed owners, masks, enum variants, edge values, loop headers, returns, traps and reverse cleanup across nested/repeated CFG. |
| `I2` | #326 | Hostile state, mask, variant, owner, result, edge, cleanup and lexical-borrow forgeries reject with exact code/message/count/order/span traces and recover on replay. |
| `P1` | #327 | The #270 String/Struct/Enum/FixedArray/Vec, Shared/Weak-bearing and finite Vec-indirection payload matrix composes calls, WeakUpgrade and nested CFG through verified IR. |
| `F1` | #327 | Existing fallible Shared/Weak, Vec and call occupants retain source order and exact reverse-prefix fault/fallthrough/loop cleanup; the trace describes verified cleanup, not execution. |
| `R1` | #327 | An authenticated source-produced graph reaches exact block/edge ceilings and rejects first-extra; a separate synthetic held-resource case proves checked overflow. Both report stable `ZRYNA-M3201`, preserve failed state and recover on the same lowerer. |

## Exact executable evidence bindings

| Key | Exact Rust path and `#[test]` names |
| --- | --- |
| `S1` | `crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs` / `structured_owned_nested_repeated_branches_and_loops_verify_without_owner_repair`, `structured_owned_mixed_graphs_compose_nested_scopes_loops_and_returns`, `legacy_string_and_vec_signatures_route_from_shared_structured_shapes`, `one_arm_owned_match_with_a_block_continuation_uses_structured_cfg`, `omitted_else_asymmetric_return_loop_return_and_post_loop_are_reachable` |
| `S2` | `crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs` / `structured_owned_unequal_branches_and_loop_header_moves_reject_deterministically`, `lexical_shadowing_drops_the_inner_owner_and_restores_the_outer_binding`, `mutable_copy_statement_updates_a_loop_condition_exactly_once`, `structured_copy_mutability_type_and_unreachable_errors_replay_without_repair`, `lexical_shadowing_does_not_admit_a_same_block_duplicate`; `crates/zryna-semantics/src/data_ownership_v1/tests/weak_upgrade_source.rs` / `weak_upgrade_exact_shadow_is_scoped_and_case_fold_collision_still_rejects` |
| `B1` | `crates/zryna-semantics/src/data_ownership_v1/tests/private_loop_cleanup_budgets.rs` / `private_string_loop_rejects_incoming_owner_move_at_loop_join`; `crates/zryna-semantics/src/data_ownership_v1/tests/private_loop_core.rs` / `private_vec_mutation_loop_rejects_immutable_target_at_exact_operation`; `crates/zryna-semantics/src/data_ownership_v1/tests/private_string_if.rs` / `private_string_if_accepts_nested_owned_control_flow`; `crates/zryna-semantics/src/data_ownership_v1/tests/private_string_mutation_loop.rs` / `private_string_mutation_loop_resolves_nested_callees_but_allows_direct_reads`; `crates/zryna-semantics/src/data_ownership_v1/tests/terminal_owned_if.rs` / `terminal_string_if_returns_owned_results_directly_from_each_arm`, `terminal_vec_if_returns_exact_vec_results_directly_from_each_arm` |
| `I1` | `crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs` / `mixed_owner_nested_repeated_cfg_seals_transfers_variants_and_cleanup` |
| `I2` | `crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs` / `mixed_cfg_rejects_state_mask_variant_owner_edge_and_cleanup_forgeries`, `mixed_cfg_rejects_a_foreign_result_owner_identity`, `mixed_cfg_rejects_missing_extra_and_reordered_cleanup`, `mixed_cfg_rejects_lexical_borrow_escape_at_every_edge_and_exit`, `mixed_cfg_join_diagnostic_is_independent_of_branch_target_order` |
| `P1` | `crates/zryna-semantics/src/data_ownership_v1/tests/structured_payload_cfg.rs` / `payload_matrix_composes_handles_vec_calls_upgrade_and_nested_cfg` |
| `F1` | `crates/zryna-semantics/src/data_ownership_v1/tests/structured_payload_cfg.rs` / `payload_cfg_fallible_operations_keep_source_ordered_cleanup` |
| `R1` | `crates/zryna-semantics/src/data_ownership_v1/tests/structured_graph_resources.rs` / `authenticated_graph_hits_exact_and_first_extra_block_and_edge_limits`, `synthetic_held_block_and_edge_overflow_is_checked_and_recovers` |

The paths and names above are repository-relative and are resolved by
`tests/m3-issue-graph-cases.mjs`. Missing, renamed, reordered, duplicated, or invented evidence and
boundary-language drift fail closed.

## Closure boundary

The integrated compiler evidence makes #271 a compiler-only closure candidate after its required
full and ignored suites, preflight, M0/M2 regressions, independent review, and hosted Linux/Windows
CI pass. This document records exact executable bindings; it is not an execution receipt for any
gate not actually run.

#272 remains responsible for complete internal owned calls and inherited imported-function
resolution. #273 remains responsible for complete exhaustive enum matching and active-payload
composition. #275 remains responsible for non-indexed owned/static/active-payload lexical
borrowing, and #269 remains the normative source-completion parent. Existing call, Match and
WeakUpgrade operations appear here only as already admitted CFG occupants.

Still excluded are break, continue, exceptions, unstructured control flow, new borrow-carrying CFG
edges, implicit ownership repair, runtime allocation/refcount/drop, backend lowering, target
execution, driver or CLI routing, artifact publication, and public `data-ownership-v1` activation.
