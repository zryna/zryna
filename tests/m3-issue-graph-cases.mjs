import assert from "node:assert/strict";
import test from "node:test";
import { readFileSync } from "node:fs";
import { m3IssueGraph, validateM3IssueOrder } from "../scripts/lib/m3-issue-graph.mjs";
import { loadAndValidateM3Contract, validateM3Contract } from "../scripts/check-m3-contract.mjs";

test("M3 completion dependencies use topological rather than issue-number order", () => {
  validateM3IssueOrder(m3IssueGraph);
  validateM3IssueOrder([{ number: 254, dependsOn: [] }, { number: 84, dependsOn: [254] }]);
  const graph = loadAndValidateM3Contract().issues;
  for (const target of [84, 85, 86]) {
    const dependencies = graph.find(issue => issue.number === target).dependsOn;
    for (const prerequisite of [254, 255, 256, 269]) assert(dependencies.includes(prerequisite));
  }
  assert.deepEqual(graph.find(issue => issue.number === 83).dependsOn, [80, 81, 82, 264]);
});

test("M3 dependency order rejects missing, cyclic, duplicate and malformed authority", () => {
  const rejected = [
    [], null,
    [{ number: 1, dependsOn: [1] }],
    [{ number: 1, dependsOn: [2] }, { number: 2, dependsOn: [1] }],
    [{ number: 1, dependsOn: [9] }],
    [{ number: 1, dependsOn: [] }, { number: 1, dependsOn: [] }],
    [{ number: 1, dependsOn: [] }, { number: 2, dependsOn: [1, 1] }],
    [{ number: 0, dependsOn: [] }],
    [{ number: 1.5, dependsOn: [] }],
    [{ number: 1, dependsOn: null }],
    [{ number: 1, dependsOn: [] }, { number: 2, dependsOn: ["1"] }],
  ];
  for (const graph of rejected) assert.throws(() => validateM3IssueOrder(graph), /M3 issue/);
  validateM3IssueOrder(m3IssueGraph);
});

test("M3 contract rejects removed completion edges and invented graph nodes", () => {
  for (const target of [84, 85, 86]) {
    for (const prerequisite of [254, 255, 256]) {
      const contract = structuredClone(loadAndValidateM3Contract());
      const node = contract.issues.find(issue => issue.number === target);
      node.dependsOn = node.dependsOn.filter(number => number !== prerequisite);
      assert.throws(() => validateM3Contract(contract), /issue graph drifted/);
    }
  }
  const contract = structuredClone(loadAndValidateM3Contract());
  contract.issues.push({ number: 999, dependsOn: [90], gate: "unreviewed" });
  assert.throws(() => validateM3Contract(contract), /issue graph drifted/);
  assert.throws(() => m3IssueGraph[0].dependsOn.push(999), TypeError);
  assert.throws(() => { m3IssueGraph[0].number = 999; }, TypeError);
  loadAndValidateM3Contract();
});

const priorDependencies = new Map([
  [75, []], [76, [75]], [77, [75]], [78, [75, 77]], [79, [76, 77, 78]],
  [80, [75, 77]], [81, [78, 79, 80]], [82, [81]], [83, [80, 81, 82]],
  [254, [77, 78, 80, 81, 82]], [255, [76, 79, 81, 82, 254]],
  [256, [76, 80, 81, 82, 254]],
  [84, [79, 80, 81, 82, 83, 254, 255, 256]],
  [85, [79, 80, 81, 82, 83, 254, 255, 256]],
  [86, [78, 80, 81, 82, 83, 254, 255, 256]],
  [87, [77, 80, 86]], [88, [76, 84, 85, 87]], [89, [88]], [90, [89]],
]);
const stagedDependencies = new Map([
  [259, [80, 81, 82]], [277, [77, 78, 80, 82, 259]],
  [260, [80, 81, 82, 259, 277]], [278, [277]], [279, [277, 278, 260]],
  [261, [80, 81, 82, 259, 260, 278]], [262, [82, 259, 260, 261, 279]],
  [263, [82, 259, 260, 261, 262]], [264, [80, 81, 82, 259, 260, 261, 262, 263]],
  [270, [76, 77, 78, 79, 80, 81, 83, 277, 278]], [271, [270, 279]],
  [272, [270, 271]], [273, [270, 271]], [274, [254, 270]],
  [275, [82, 254, 270, 271, 272, 273]],
  [269, [83, 254, 255, 256, 270, 271, 272, 273, 274, 275, 277, 278, 279]],
]);

test("M3 staged closure preserves every prior edge and rejects all 81 added-edge removals", () => {
  const contract = loadAndValidateM3Contract();
  const graph = new Map(contract.issues.map(issue => [issue.number, issue.dependsOn]));
  assert.equal(graph.size, 35);
  for (const [number, dependencies] of priorDependencies) {
    const expected = [...dependencies];
    if (number === 83) expected.push(264);
    if ([84, 85, 86].includes(number)) expected.push(269);
    assert.deepEqual(graph.get(number), expected);
  }
  for (const [number, dependencies] of stagedDependencies)
    assert.deepEqual(graph.get(number), dependencies);
  let removedEdges = 0;
  for (const issue of contract.issues) {
    for (const dependency of issue.dependsOn) {
      if (priorDependencies.get(issue.number)?.includes(dependency)) continue;
      const mutated = structuredClone(contract);
      const node = mutated.issues.find(node => node.number === issue.number);
      node.dependsOn = node.dependsOn.filter(number => number !== dependency);
      assert.throws(() => validateM3Contract(mutated), /issue graph drifted/);
      removedEdges += 1;
    }
  }
  assert.equal(removedEdges, 81);
});

test("M3 cores remain independently closeable and every target retains complete blockers", () => {
  const contract = loadAndValidateM3Contract();
  const graph = new Map(contract.issues.map(issue => [issue.number, issue.dependsOn]));
  const ancestors = number => {
    const seen = new Set();
    const pending = [...graph.get(number)];
    while (pending.length) {
      const next = pending.pop();
      if (seen.has(next)) continue;
      seen.add(next);
      pending.push(...graph.get(next));
    }
    return seen;
  };
  for (const stage of [277, 278, 279])
    for (const consumer of [83, 261, 262, 269, 270, 271, 272, 273])
      assert(!ancestors(stage).has(consumer));
  for (const target of [84, 85, 86])
    for (const prerequisite of [83, 254, 255, 256, 269, 270, 271, 272, 273, 274, 275, 277, 278, 279])
      assert(ancestors(target).has(prerequisite));
  for (const stage of [277, 278, 279]) {
    for (const kind of ["missing", "duplicate", "cycle"]) {
      const mutated = structuredClone(contract);
      const node = mutated.issues.find(issue => issue.number === stage);
      if (kind === "missing") mutated.issues = mutated.issues.filter(issue => issue.number !== stage);
      if (kind === "duplicate") mutated.issues.push(structuredClone(node));
      if (kind === "cycle") node.dependsOn.push(269);
      assert.throws(() => validateM3Contract(mutated), /issue graph drifted/);
      assert.throws(() => validateM3IssueOrder(mutated.issues), /M3 issue/);
    }
  }
  for (const issue of m3IssueGraph) {
    assert(Object.isFrozen(issue));
    assert(Object.isFrozen(issue.dependsOn));
  }
  assert(Object.isFrozen(m3IssueGraph));
});

test("M3 source-completion documentation retains staged ownership and historical evidence", () => {
  const read = name => readFileSync(new URL(`../docs/${name}.md`, import.meta.url), "utf8");
  const roadmap = read("ROADMAP");
  for (const issue of [269, 270, 271, 272, 273, 274, 275, 277, 278, 279])
    assert(roadmap.includes(`#${issue}`));
  for (const name of ["ROADMAP", "STATUS", "M3_BORROWING_SEMANTICS"]) {
    const document = read(name);
    assert(document.includes("#269"));
    assert(document.includes("#277"));
    assert(document.includes("#278"));
    assert(document.includes("#279"));
  }
  assert.match(read("STATUS"), /d61d1ec50005bbed7d86f029fa6ece5efa7517d495b6aed6e9b0f1c15f69e20f/);
});

const genericMatrixDocument = readFileSync(
  new URL("../docs/M3_GENERIC_OWNED_COMPOSITION_MATRIX.md", import.meta.url),
  "utf8",
);
const genericMatrixBindings = [
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_construction.rs", "mixed_construction_vec_of_owned_struct_reaches_verified_ir"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_construction.rs", "mixed_construction_owned_struct_containing_vec_reaches_verified_ir"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/nested_mixed_construction.rs", "mixed_nested_selected_enum_vec_payload_reaches_verified_ir"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_recursive_vec.rs", "mixed_recursive_vec_indirection_constructs_finite_empty_child_through_full_ir"],
  ["E1", "crates/zryna-ir/src/data_ownership_v1/tests/mixed_constructor_authority.rs", "mixed_raw_constructor_mutations_fail_at_exact_authority_phase"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/constructor_child_preparation_matrix.rs", "constructor_child_matrix_nested_array_later_child_is_atomic"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/constructor_child_preparation_matrix.rs", "constructor_child_matrix_valid_nested_sources_replay_through_full_verifier"],
  ["E1", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_constructor_faults.rs", "mixed_nested_vec_and_selected_enum_faults_preserve_completed_children"],
  ["E2", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement.rs", "mixed_root_replacement_constructors_moves_and_repeated_commits_reach_verified_ir"],
  ["E2", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_struct_whole_moves.rs", "mixed_owned_struct_local_moves_once_into_outer_vec_with_exact_cleanup"],
  ["E2", "crates/zryna-ir/src/data_ownership_v1/tests/mixed_replacement_authority.rs", "mixed_replacement_hostile_mutations_reject_deterministically_after_valid_control"],
  ["E2", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement_controls.rs", "mixed_root_replacement_final_transition_exact_and_first_extra_preserve_state"],
  ["E2", "crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement_controls.rs", "mixed_root_replacement_invalid_targets_and_self_moves_preserve_prior_statements"],
  ["E3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_subtree_clone_and_move_keep_exact_parent_masks"],
  ["E3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_repeated_and_self_clone_replacements_retain_target_until_commit"],
  ["E3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer.rs", "generic_static_transfer_rejects_wrong_referent_and_unavailable_move"],
  ["E3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer.rs", "generic_static_transfer_rejects_reused_rhs_and_a_hole_at_the_commit_target"],
  ["E3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs", "generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional"],
  ["E3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs", "generic_static_move_resources_exact_extra_overflow_and_recovery_do_not_leak_masks"],
  ["E3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_partial_root_and_self_consuming_rhs_are_stable_rejections"],
  ["E4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_source.rs", "generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup"],
  ["E4", "crates/zryna-ir/src/data_ownership_v1/tests/generic_clone_hostile.rs", "generic_clone_forbidden_descendants_propagate_through_nested_and_recursive_graphs"],
  ["E4", "crates/zryna-ir/src/data_ownership_v1/tests/generic_clone_hostile.rs", "generic_clone_rejects_forged_recursive_prefix_owner_shape_order_and_site"],
  ["E4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_resources.rs", "generic_clone_resource_exact_first_extra_preserve_source_state_credits_and_recovery"],
  ["E4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_resources.rs", "generic_clone_missing_source_precedes_deferred_cleanup_capacity"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source.rs", "generic_vec_source_owned_observations_clone_exact_elements_and_retain_container_cleanup"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source.rs", "generic_vec_source_owned_replacement_checks_bounds_before_rhs_and_keeps_old_container_on_failure"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_push_source.rs", "generic_vec_push_source_prepares_owned_elements_before_growth_and_retains_exact_failure_owners"],
  ["E5", "crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_hostile.rs", "generic_vec_observation_rejects_inactive_foreign_and_wrong_referent_authority"],
  ["E5", "crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_hostile.rs", "generic_vec_observation_rejects_wrong_prefix_source_alias_and_premature_result_cleanup"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs", "generic_vec_replacement_resource_exact_first_extra_reserves_bounds_rhs_commit_and_end"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source_rejections.rs", "generic_vec_source_owned_bare_index_never_moves_an_element_or_leaves_a_hole"],
  ["E5", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source_rejections.rs", "generic_vec_source_replacement_blocks_same_container_clone_before_nested_index_evaluation"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_composition.rs", "finite_recursive_composition_reaches_verified_ir"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_projections.rs", "finite_recursive_static_move_and_replacement_bind_exact_owners_masks_and_commit"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_projections.rs", "finite_recursive_vec_observation_and_replacement_bind_borrows_and_exact_owners"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs", "recursive_construction_rejects_forged_variant_element_and_cleanup_then_recovers"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs", "recursive_whole_move_and_root_replacement_reject_identity_and_cleanup_forgery"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs", "recursive_static_move_and_replacement_reject_path_owner_and_cleanup_forgery"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs", "recursive_vec_observation_replacement_and_push_reject_forged_authority"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_composition.rs", "recursive_clone_rejects_wrong_identity_prefix_and_cleanup_order_then_recovers"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/finite_recursive_resources.rs", "finite_recursive_clone_and_push_resources_are_exact_atomic_and_recoverable"],
  ["R1", "crates/zryna-ir/src/data_ownership_v1/tests/generic_recursive_resources.rs", "recursive_clone_resource_preflight_is_exact_checked_and_replay_stable"],
  ["H1", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "shared_and_weak_structural_payload_categories_lower_through_verified_ir"],
  ["H1", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "handle_leaves_compose_through_struct_array_vec_projection_and_replacement"],
  ["H1", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_payload_closure.rs", "recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay"],
  ["H1", "crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs", "shared_weak_instruction_types_reject_wrong_payload_and_handle_categories"],
  ["H1", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs", "direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup"],
  ["H2", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "structural_handle_clone_seals_struct_enum_array_and_vec_count_recipes"],
  ["H2", "crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs", "seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles"],
  ["H2", "crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs", "rejects_non_handle_roots_and_inexact_prefix_cleanup_deterministically"],
  ["H2", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs", "handle_aware_structural_clone_has_exact_cleanup_frontier_and_pristine_retry"],
  ["H2", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_frontier_source.rs", "handle_frontier_source_vec_enum_occurrences_retain_replacement_owners"],
  ["H3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_subtree_clone_and_move_keep_exact_parent_masks"],
  ["H3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_repeated_and_self_clone_replacements_retain_target_until_commit"],
  ["H3", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_static_source.rs", "handle_static_source_rejects_wrong_rhs_repeated_move_and_moved_target"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_without_clone_preserves_recursive_masks_and_owner_identity"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_replacement_retains_old_target_through_count_failure"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_rejects_exact_type_borrow_and_consumption_forgery_then_recovers"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_rejects_repeated_move_forged_path_and_omitted_parent_cleanup"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_rejects_partial_target_and_wrong_move_result"],
  ["H3", "crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer/handles.rs", "handle_static_transfer_resource_preflight_exact_extra_overflow_and_recovery"],
  ["H3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs", "generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional"],
  ["H3", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs", "generic_static_move_resources_exact_extra_overflow_and_recovery_do_not_leak_masks"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_source.rs", "generic_vec_handle_observation_clones_counts_and_retains_the_complete_container"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_source.rs", "generic_vec_handle_replacement_checks_bounds_then_commits_one_exact_owner"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_source.rs", "generic_vec_handle_push_prepares_once_and_transfers_only_after_success"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_source.rs", "generic_vec_handle_source_replays_identically"],
  ["H4", "crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_handle_hostile.rs", "generic_vec_handle_ir_rejects_stale_or_foreign_borrow_then_recovers"],
  ["H4", "crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_handle_hostile.rs", "generic_vec_handle_ir_rejects_wrong_result_and_cleanup_identity_then_recovers"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_rejections.rs", "generic_vec_handle_bare_observation_rejects_implicit_move_and_recovers"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_rejections.rs", "generic_vec_handle_replacement_rejects_overlap_before_nested_index_effects"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_handle_rejections.rs", "generic_vec_handle_replacement_rejects_wrong_exact_owned_type_and_recovers"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs", "generic_vec_handle_push_resource_exact_first_extra_overflow_and_recovery"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs", "generic_vec_place_handle_push_clone_has_exact_direct_and_structural_resource_frontiers"],
  ["H4", "crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs", "generic_vec_place_handle_push_clone_overflow_replays_and_recovers"],
];

function rustTests(file) {
  const text = readFileSync(new URL(`../${file}`, import.meta.url), "utf8");
  return new Set([...text.matchAll(
    /#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([a-z][a-z0-9_]+)\s*\(/g,
  )].map(match => match[1]));
}

function genericMatrixRows(text) {
  const section = text.split("## Frozen type/operation matrix\n")[1]
    ?.split("## Exact existing evidence bindings")[0];
  assert(section, "generic owned operation matrix is missing");
  return section.split("\n")
    .filter(line => /^\| (Nested|Zero|Empty|Finite|Direct|Struct\/)/.test(line))
    .map(line => line.split("|").slice(1, -1).map(cell => cell.trim().replaceAll("`", "")));
}

function genericMatrixEvidenceBindings(text) {
  const section = text.split("## Exact existing evidence bindings\n")[1]
    ?.split("## Implemented child integration")[0];
  assert(section, "generic owned evidence bindings are missing");
  return section.split("\n")
    .filter(line => /^\| `(?:E[1-5]|H[1-4]|R1)` \|/.test(line))
    .flatMap(line => {
      const tokens = [...line.matchAll(/`([^`]+)`/g)].map(match => match[1]);
      const key = tokens.shift();
      let file;
      const bindings = [];
      for (const token of tokens) {
        if (token.endsWith(".rs")) {
          file = token;
        } else {
          assert(file, `${key}: test name appears before its Rust path`);
          bindings.push([key, file, token]);
        }
      }
      return bindings;
    });
}

export function validateGenericOwnedCompositionMatrix(text) {
  assert.deepEqual(genericMatrixRows(text), [
    ["Nested non-handle Struct", "E1", "E2", "E3", "E4", "E5"],
    ["Nested non-handle Enum, every active variant", "E1", "E2", "E3", "E4", "E5"],
    ["Zero/nonzero FixedArray with non-handle elements", "E1", "E2", "E3", "E4", "E5"],
    ["Empty/nonempty positive-stride Vec with non-handle elements", "E1", "E2", "E3", "E4", "E5"],
    ["Finite values through legal non-handle Vec indirection recursion", "R1", "R1", "R1", "R1", "R1"],
    ["Direct Shared/Weak", "H1", "H1", "H3", "H2", "H4"],
    ["Struct/Enum/FixedArray/Vec containing Shared/Weak leaves", "H1", "H1", "H3", "H2", "H4"],
    ["Finite values through legal Shared/Weak indirection recursion", "H1", "H1", "H3", "H2", "H4"],
  ]);
  for (const key of ["E1", "E2", "E3", "E4", "E5", "H1", "H2", "H3", "H4", "R1"])
    assert(text.includes(`| \`${key}\` |`), `generic matrix omits evidence key ${key}`);
  for (const [key, issue] of [["H3", 321], ["H4", 322], ["R1", 323]])
    assert(text.includes(`| \`${key}\` | #${issue} |`), `generic matrix omits child ${key}`);
  assert(!text.includes("G321") && !text.includes("G322"), "generic matrix retains a resolved gap");
  for (const issue of [269, 270, 271, 272, 273, 274, 275])
    assert(text.includes(`#${issue}`), `generic matrix omits sibling/downstream #${issue}`);
  for (const phrase of [
    "no executable evidence may be claimed",
    "runtime allocation/refcount/drop",
    "backend lowering, target execution",
    "public\n`data-ownership-v1` activation",
    "listing existing tests here is not a new execution receipt.",
  ]) assert(text.includes(phrase), `generic matrix boundary drifted: ${phrase}`);
  assert.deepEqual(
    genericMatrixEvidenceBindings(text),
    genericMatrixBindings,
    "generic matrix evidence path/name pairs drifted",
  );
  const inventories = new Map();
  for (const [, file, name] of genericMatrixBindings) {
    if (!inventories.has(file)) inventories.set(file, rustTests(file));
    assert(inventories.get(file).has(name), `${file}: missing generic matrix test ${name}`);
  }
}

test("generic owned composition matrix maps every implemented cell exactly", () => {
  validateGenericOwnedCompositionMatrix(genericMatrixDocument);
});

test("generic owned composition matrix rejects evidence and exclusion drift", () => {
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "| `H3` | #321 |", "| `E3` | #321 |",
  )));
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup",
    "missing_generic_clone_source_test",
  )));
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "mixed_construction_owned_struct_containing_vec_reaches_verified_ir",
    "missing_secondary_source_test",
  )));
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "runtime allocation/refcount/drop", "runtime allocation and refcount execution",
  )));
});

const structuredMatrixDocument = readFileSync(
  new URL("../docs/M3_STRUCTURED_OWNED_CONTROL_FLOW_MATRIX.md", import.meta.url),
  "utf8",
);
const structuredMatrixBindings = [
  ["S1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "structured_owned_nested_repeated_branches_and_loops_verify_without_owner_repair"],
  ["S1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "structured_owned_mixed_graphs_compose_nested_scopes_loops_and_returns"],
  ["S1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "legacy_string_and_vec_signatures_route_from_shared_structured_shapes"],
  ["S1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "one_arm_owned_match_with_a_block_continuation_uses_structured_cfg"],
  ["S1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "omitted_else_asymmetric_return_loop_return_and_post_loop_are_reachable"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "structured_owned_unequal_branches_and_loop_header_moves_reject_deterministically"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "lexical_shadowing_drops_the_inner_owner_and_restores_the_outer_binding"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "mutable_copy_statement_updates_a_loop_condition_exactly_once"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "structured_copy_mutability_type_and_unreachable_errors_replay_without_repair"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_owned_source.rs", "lexical_shadowing_does_not_admit_a_same_block_duplicate"],
  ["S2", "crates/zryna-semantics/src/data_ownership_v1/tests/weak_upgrade_source.rs", "weak_upgrade_exact_shadow_is_scoped_and_case_fold_collision_still_rejects"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/private_loop_cleanup_budgets.rs", "private_string_loop_rejects_incoming_owner_move_at_loop_join"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/private_loop_core.rs", "private_vec_mutation_loop_rejects_immutable_target_at_exact_operation"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/private_string_if.rs", "private_string_if_accepts_nested_owned_control_flow"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/private_string_mutation_loop.rs", "private_string_mutation_loop_resolves_nested_callees_but_allows_direct_reads"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/terminal_owned_if.rs", "terminal_string_if_returns_owned_results_directly_from_each_arm"],
  ["B1", "crates/zryna-semantics/src/data_ownership_v1/tests/terminal_owned_if.rs", "terminal_vec_if_returns_exact_vec_results_directly_from_each_arm"],
  ["I1", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_owner_nested_repeated_cfg_seals_transfers_variants_and_cleanup"],
  ["I2", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_cfg_rejects_state_mask_variant_owner_edge_and_cleanup_forgeries"],
  ["I2", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_cfg_rejects_a_foreign_result_owner_identity"],
  ["I2", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_cfg_rejects_missing_extra_and_reordered_cleanup"],
  ["I2", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_cfg_rejects_lexical_borrow_escape_at_every_edge_and_exit"],
  ["I2", "crates/zryna-ir/src/data_ownership_v1/tests/cfg_authority_hostile.rs", "mixed_cfg_join_diagnostic_is_independent_of_branch_target_order"],
  ["P1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_payload_cfg.rs", "payload_matrix_composes_handles_vec_calls_upgrade_and_nested_cfg"],
  ["F1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_payload_cfg.rs", "payload_cfg_fallible_operations_keep_source_ordered_cleanup"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_graph_resources.rs", "authenticated_graph_hits_exact_and_first_extra_block_and_edge_limits"],
  ["R1", "crates/zryna-semantics/src/data_ownership_v1/tests/structured_graph_resources.rs", "synthetic_held_block_and_edge_overflow_is_checked_and_recovers"],
];

function structuredMatrixEvidenceBindings(text) {
  const section = text.split("## Exact executable evidence bindings\n")[1]
    ?.split("## Closure boundary")[0];
  assert(section, "structured owned evidence bindings are missing");
  return section.split("\n")
    .filter(line => /^\| `(?:S1|S2|B1|I1|I2|P1|F1|R1)` \|/.test(line))
    .flatMap(line => {
      const tokens = [...line.matchAll(/`([^`]+)`/g)].map(match => match[1]);
      const key = tokens.shift();
      let file;
      const bindings = [];
      for (const token of tokens) {
        if (token.endsWith(".rs")) {
          file = token;
        } else {
          assert(file, `${key}: test name appears before its Rust path`);
          bindings.push([key, file, token]);
        }
      }
      return bindings;
    });
}

export function validateStructuredOwnedControlFlowMatrix(text) {
  assert(!/\b(?:placeholder|TBD|TODO)\b/i.test(text), "structured matrix contains a placeholder");
  for (const [key, issue] of [["S1", 325], ["S2", 325], ["B1", 325], ["I1", 326], ["I2", 326], ["P1", 327], ["F1", 327], ["R1", 327]])
    assert(text.includes(`| \`${key}\` | #${issue} |`), `structured matrix omits ${key}/#${issue}`);
  for (const issue of [269, 271, 272, 273, 275, 325, 326, 327])
    assert(text.includes(`#${issue}`), `structured matrix omits boundary #${issue}`);
  for (const phrase of [
    "compiler-only closure candidate",
    "verified cleanup, not execution",
    "break, continue, exceptions, unstructured control flow",
    "runtime allocation/refcount/drop",
    "backend lowering, target\nexecution",
    "public `data-ownership-v1` activation",
    "not an execution receipt",
    "#275 remains responsible",
    "authenticated source-produced graph reaches exact block/edge ceilings and rejects first-extra",
    "separate synthetic held-resource case proves checked overflow",
  ]) assert(text.includes(phrase), `structured matrix boundary drifted: ${phrase}`);
  assert.deepEqual(
    structuredMatrixEvidenceBindings(text),
    structuredMatrixBindings,
    "structured matrix evidence path/name pairs drifted",
  );
  assert.equal(new Set(structuredMatrixBindings.map(binding => binding.slice(1).join("/"))).size,
    structuredMatrixBindings.length, "structured matrix duplicates executable evidence");
  const inventories = new Map();
  for (const [, file, name] of structuredMatrixBindings) {
    if (!inventories.has(file)) inventories.set(file, rustTests(file));
    assert(inventories.get(file).has(name), `${file}: missing structured matrix test ${name}`);
  }
}

test("structured owned control-flow matrix binds every closure row exactly", () => {
  validateStructuredOwnedControlFlowMatrix(structuredMatrixDocument);
});

test("structured owned control-flow matrix rejects evidence and boundary drift", () => {
  for (const [from, to] of [
    ["| `I2` | #326 |", "| `I2` | #325 |"],
    ["mixed_cfg_rejects_a_foreign_result_owner_identity", "missing_hostile_test"],
    ["synthetic_held_block_and_edge_overflow_is_checked_and_recovers", "missing_overflow_test"],
    ["runtime allocation/refcount/drop", "runtime ownership execution"],
    ["#275 remains responsible", "#275 is complete"],
  ]) assert.throws(() => validateStructuredOwnedControlFlowMatrix(
    structuredMatrixDocument.replace(from, to),
  ));
});
