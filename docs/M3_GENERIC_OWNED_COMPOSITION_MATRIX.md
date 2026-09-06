# M3 generic owned composition closure matrix

Status: Issue #320 checked planning authority for parent #270. This document freezes the bounded
type/operation matrix and distinguishes merged compiler evidence from three concrete implementation
gaps. It adds no source form, IR operation, runtime, backend, driver route or public profile.

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
- `gap`: no executable evidence may be claimed until its owning issue implements and verifies it.

The focused commands are `pnpm m3:data:quick` for semantic source/planner rows,
`cargo test --locked -p zryna-ir data_ownership_v1` for independent IR rows, and
`pnpm m3:runtime-abi:quick` only when the completed #83 handle transition authority is relevant.
Passing those commands does not convert compiler evidence into target execution.

## Frozen type/operation matrix

`Whole transfer` includes a complete local move and complete root replacement. `Static transfer`
means a Struct-field or constant FixedArray subobject move/replacement. `Ordinary Vec element`
means transient checked observation, exact replacement and prepared push; it is not the persistent
indexed-borrow producer from #255/#256. Every `G321`, `G322` or `G323` cell blocks #270.

| Payload graph | Construction | Whole transfer | Static transfer | Structural clone | Ordinary Vec element |
| --- | --- | --- | --- | --- | --- |
| Nested non-handle Struct | `E1` | `E2` | `E3` | `E4` | `E5` |
| Nested non-handle Enum, every active variant | `E1` | `E2` | `E3` | `E4` | `E5` |
| Zero/nonzero FixedArray with non-handle elements | `E1` | `E2` | `E3` | `E4` | `E5` |
| Empty/nonempty positive-stride Vec with non-handle elements | `E1` | `E2` | `E3` | `E4` | `E5` |
| Finite values through legal non-handle Vec indirection recursion | `E1` | `G323` | `G323` | `G323` | `G323` |
| Direct Shared/Weak | `H1` | `H1` | `G321` | `H2` | `G322` |
| Struct/Enum/FixedArray/Vec containing Shared/Weak leaves | `H1` | `H1` | `G321` | `H2` | `G322` |
| Finite values through legal Shared/Weak indirection recursion | `H1` | `H1` | `G321` | `H2` | `G322` |

This matrix does not broaden clone capability. `E4` is the generic non-handle structural clone;
`H2` is the distinct #83 handle-aware clone whose Shared/Weak leaves perform explicit count
operations. Transfer eligibility is independent from clone eligibility. `G321` must therefore
reuse generic transfer semantics without admitting handle graphs through `GenericClonePlace`.

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
| `H1` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs` / `shared_and_weak_structural_payload_categories_lower_through_verified_ir`, `handle_leaves_compose_through_struct_array_vec_projection_and_replacement`; `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_payload_closure.rs` / `recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay` | `crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs` / `shared_weak_instruction_types_reject_wrong_payload_and_handle_categories` | `crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs` / `direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup`; #83's checked ledger retains exact resource/diagnostic/replay mappings |
| `H2` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs` / `structural_handle_clone_seals_struct_enum_array_and_vec_count_recipes` | `crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs` / `seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles`, `rejects_non_handle_roots_and_inexact_prefix_cleanup_deterministically` | `crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs` / `handle_aware_structural_clone_has_exact_cleanup_frontier_and_pristine_retry`; `crates/zryna-semantics/src/data_ownership_v1/tests/handle_frontier_source.rs` / `handle_frontier_source_vec_enum_occurrences_retain_replacement_owners` |

The existing `H1` whole-transfer evidence is bounded to complete owners and roots. It is not proof
of the missing static-subobject or ordinary Vec-element rows. Likewise, #255/#256
`indexed_handle_source` evidence proves explicit transient/lexical authority and `BorrowReplace`;
it cannot be relabelled as #322 ordinary Vec source completion.

## Concrete blocking implementation gaps

| Key | Owning issue | Missing executable boundary required before #270 closes |
| --- | --- | --- |
| `G321` | #321 | Authenticated static Struct-field/constant FixedArray subobject move, replacement, repeated replacement and return for handle-containing graphs; mandatory IR must separate transfer eligibility from handle-excluding generic clone, retain exact owners/masks/source/old target, and reject self-use, repeated move, partial/moved/wrong-type/borrowed targets with resource and replay proof. No such complete source-to-verified-IR matrix exists at this base. |
| `G322` | #322 | Authenticated ordinary Vec explicit handle-aware observation, exact replacement and prepared push for admitted owned/handle-containing element graphs; independent bounds-before-RHS, growth retention, active payload, cleanup, zero-stride/hostile authority and exact/first-extra/overflow recovery proof. Indexed-borrow and direct handle-clone tests are prerequisites, not this missing ordinary operation matrix. |
| `G323` | #323 | Authenticated finite values of a recursive nominal non-handle graph whose recursion crosses Vec indirection, covering complete-root move/replacement, static-subobject move/replacement, explicit structural clone, and ordinary Vec observation/replacement/prepared push; each operation still needs independent hostile, resource, exact diagnostic and deterministic recovery proof. Existing nested `Vec<Vec<String>>`, recursive-construction and verifier-only recursive-prefix tests are prerequisites, not this missing source-to-verified-IR operation matrix. |

#321 and #322 depend on completed #83 and this frozen matrix. They reuse #278 operations and #83
handle authority; neither may add a competing count, clone, borrow, cleanup or runtime model. #323
retains the existing non-handle operation meanings and supplies only the missing recursive source
matrix. Completion of any one gap leaves the others blocking #270.

## Preserved sibling and downstream boundaries

- #271 retains full structured owned control flow and lexical cleanup. This matrix does not claim
  unrestricted repeated/nested CFG, early-return or post-loop composition.
- #272 retains complete internal owned calls and inherited named-import resolution. Existing
  bounded generic/handle calls do not close its multi-argument/result/module matrix.
- #273 retains exhaustive enum match and active-payload composition beyond existing bounded cases.
- #274 remains the separately completed ordinary dynamic FixedArray authority; this document does
  not duplicate or weaken #254/#255 indexed ownership.
- #275 retains non-indexed owned lexical borrowing. #255/#256 retain dynamic array/Vec borrowing;
  no persistent alias, borrowed move-out or borrow-carrying CFG edge is added here.
- #269 remains the parent integration gate after #270–#275. #320 freezes a plan and cannot close
  #270, #269 or any target issue by itself.

Still excluded are user generics, by-value recursive layout, zero-sized Vec elements, implicit
clone, Vec pop, element move-out, holes, raw pointers, tracing GC, runtime allocation/refcount/drop,
backend lowering, target execution, driver/CLI/manifest changes, public aggregate ABI, public
`data-ownership-v1` activation, three-target equivalence and website publication. Required full
suites, ignored proofs, preflight, M0/M2 and Linux/Windows CI remain merge gates for later
implementation; listing existing tests here is not a new execution receipt.

## Issue #320 acceptance reconciliation

| Acceptance | Checked result |
| --- | --- |
| Existing evidence versus gaps | `E1`–`E5` and `H1`–`H2` bind actual tests; `G321`, `G322` and `G323` are explicit gaps, and all block #270 |
| Handle delegation | `H1`/`H2` and both gap contracts delegate Shared/Weak count/clone/release meaning to completed #83 |
| Machine-checked paths/names | `tests/m3-issue-graph-cases.mjs` resolves every exact Rust path and `#[test]` name and rejects matrix/key drift |
| Exclusions | The sibling/downstream section retains #271–#275, #269, runtime, targets and public activation |
| Focused gates | Run `node --test tests/m3-issue-graph-cases.mjs`, `pnpm m3:contract`, `pnpm docs:check` and `pnpm structure:check`; report only observed results |

Issue #320 is closure-ready when this checked matrix and its focused gates pass. That closes the
planning child only. Parent #270 remains blocked on #321, #322 and #323 plus its own final integrated
semantic/IR/ABI regressions, required ignored proofs, preflight, M0/M2 and hosted Linux/Windows CI.
