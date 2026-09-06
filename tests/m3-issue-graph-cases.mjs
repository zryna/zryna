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
  ["crates/zryna-semantics/src/data_ownership_v1/tests/mixed_construction.rs", "mixed_construction_vec_of_owned_struct_reaches_verified_ir"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/nested_mixed_construction.rs", "mixed_nested_selected_enum_vec_payload_reaches_verified_ir"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/mixed_recursive_vec.rs", "mixed_recursive_vec_indirection_constructs_finite_empty_child_through_full_ir"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/mixed_constructor_authority.rs", "mixed_raw_constructor_mutations_fail_at_exact_authority_phase"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/constructor_child_preparation_matrix.rs", "constructor_child_matrix_valid_nested_sources_replay_through_full_verifier"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/mixed_constructor_faults.rs", "mixed_nested_vec_and_selected_enum_faults_preserve_completed_children"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement.rs", "mixed_root_replacement_constructors_moves_and_repeated_commits_reach_verified_ir"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/mixed_replacement_authority.rs", "mixed_replacement_hostile_mutations_reject_deterministically_after_valid_control"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/mixed_root_replacement_controls.rs", "mixed_root_replacement_final_transition_exact_and_first_extra_preserve_state"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_source.rs", "generic_static_source_subtree_clone_and_move_keep_exact_parent_masks"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/generic_static_transfer.rs", "generic_static_transfer_rejects_reused_rhs_and_a_hole_at_the_commit_target"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_static_resources.rs", "generic_static_replacement_resources_exact_extra_overflow_and_recovery_are_transactional"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_source.rs", "generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/generic_clone_hostile.rs", "generic_clone_rejects_forged_recursive_prefix_owner_shape_order_and_site"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_clone_resources.rs", "generic_clone_resource_exact_first_extra_preserve_source_state_credits_and_recovery"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source.rs", "generic_vec_source_owned_observations_clone_exact_elements_and_retain_container_cleanup"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_push_source.rs", "generic_vec_push_source_prepares_owned_elements_before_growth_and_retains_exact_failure_owners"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/generic_vec_hostile.rs", "generic_vec_observation_rejects_inactive_foreign_and_wrong_referent_authority"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_resources.rs", "generic_vec_replacement_resource_exact_first_extra_reserves_bounds_rhs_commit_and_end"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/generic_vec_source_rejections.rs", "generic_vec_source_owned_bare_index_never_moves_an_element_or_leaves_a_hole"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "shared_and_weak_structural_payload_categories_lower_through_verified_ir"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_payload_closure.rs", "recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs", "shared_weak_instruction_types_reject_wrong_payload_and_handle_categories"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs", "direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "structural_handle_clone_seals_struct_enum_array_and_vec_count_recipes"],
  ["crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs", "seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs", "handle_aware_structural_clone_has_exact_cleanup_frontier_and_pristine_retry"],
  ["crates/zryna-semantics/src/data_ownership_v1/tests/handle_frontier_source.rs", "handle_frontier_source_vec_enum_occurrences_retain_replacement_owners"],
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

export function validateGenericOwnedCompositionMatrix(text) {
  assert.deepEqual(genericMatrixRows(text), [
    ["Nested non-handle Struct", "E1", "E2", "E3", "E4", "E5"],
    ["Nested non-handle Enum, every active variant", "E1", "E2", "E3", "E4", "E5"],
    ["Zero/nonzero FixedArray with non-handle elements", "E1", "E2", "E3", "E4", "E5"],
    ["Empty/nonempty positive-stride Vec with non-handle elements", "E1", "E2", "E3", "E4", "E5"],
    ["Finite values through legal non-handle Vec indirection recursion", "E1", "E2", "E3", "E4", "E5"],
    ["Direct Shared/Weak", "H1", "H1", "G321", "H2", "G322"],
    ["Struct/Enum/FixedArray/Vec containing Shared/Weak leaves", "H1", "H1", "G321", "H2", "G322"],
    ["Finite values through legal Shared/Weak indirection recursion", "H1", "H1", "G321", "H2", "G322"],
  ]);
  for (const key of ["E1", "E2", "E3", "E4", "E5", "H1", "H2"])
    assert(text.includes(`| \`${key}\` |`), `generic matrix omits evidence key ${key}`);
  for (const [key, issue] of [["G321", 321], ["G322", 322]]) {
    assert(text.includes(`| \`${key}\` | #${issue} |`), `generic matrix omits gap ${key}`);
    assert(text.includes(`both #321 and #322 plus`), "generic matrix must keep both gaps blocking #270");
  }
  for (const issue of [269, 270, 271, 272, 273, 274, 275])
    assert(text.includes(`#${issue}`), `generic matrix omits sibling/downstream #${issue}`);
  for (const phrase of [
    "no executable evidence may be claimed",
    "runtime allocation/refcount/drop",
    "backend lowering, target execution",
    "public\n`data-ownership-v1` activation",
    "listing existing tests here is not a new execution receipt.",
  ]) assert(text.includes(phrase), `generic matrix boundary drifted: ${phrase}`);
  const inventories = new Map();
  for (const [file, name] of genericMatrixBindings) {
    assert(text.includes(`\`${file}\``), `generic matrix omits ${file}`);
    assert(text.includes(`\`${name}\``), `generic matrix omits ${name}`);
    if (!inventories.has(file)) inventories.set(file, rustTests(file));
    assert(inventories.get(file).has(name), `${file}: missing generic matrix test ${name}`);
  }
}

test("generic owned composition matrix maps evidence and both blocking gaps exactly", () => {
  validateGenericOwnedCompositionMatrix(genericMatrixDocument);
});

test("generic owned composition matrix rejects evidence, gap and exclusion drift", () => {
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "| `G321` | #321 |", "| `E3` | #321 |",
  )));
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "generic_clone_source_mixed_roots_retain_source_and_seal_recursive_prefix_cleanup",
    "missing_generic_clone_source_test",
  )));
  assert.throws(() => validateGenericOwnedCompositionMatrix(genericMatrixDocument.replace(
    "runtime allocation/refcount/drop", "runtime allocation and refcount execution",
  )));
});
