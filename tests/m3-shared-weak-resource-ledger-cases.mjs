import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import test from "node:test";

const root = new URL("../", import.meta.url);
const read = file => readFileSync(new URL(file, root), "utf8");
const document = read("docs/M3_SHARED_WEAK_EVIDENCE.md");
const scopeDocuments = [
  "docs/M3_SHARED_WEAK_AUTHORITY.md",
  "docs/M3_INDEXED_SOURCE_OPERATIONS.md",
  "docs/M3_OWNERSHIP_COMPOSITION.md",
  "docs/ROADMAP.md",
].map(file => [file, read(file)]);

const expectedBindings = [
  ["A1", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/weak_upgrade_fault_oracle.rs", "source_upgrade_binds_success_expiration_and_overflow_to_distinct_outcomes"],
  ["A1", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/boundaries.rs", "control_model_count_boundaries_remain_indivisible_existing_abi_claims"],
  ["A1", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/resources.rs", "control_model_synthetic_refcount_failure_cannot_issue_an_owner"],
  ["A1", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_counts.rs", "conformance_counts_all_increment_boundaries_preserve_exact_other_state"],
  ["A1", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_counts.rs", "conformance_counts_expired_weak_clone_is_not_upgrade_or_live_downgrade"],
  ["A2", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs", "direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup"],
  ["A2", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests.rs", "control_model_clone_expiration_and_payload_before_implicit_weak_finish"],
  ["A2", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/boundaries.rs", "control_model_allocation_failure_retains_embedded_handle_until_successful_retry"],
  ["A2", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "pending_last_strong_excludes_every_operation_except_finish"],
  ["A2", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_graph_distinct_cloned_edges_and_weak_observer_release_completely"],
  ["A3", "verified IR", "crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs", "shared_weak_cleanup_diagnostics_have_exact_order_span_and_valid_replay"],
  ["A3", "verified IR", "crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs", "shared_weak_instruction_types_reject_wrong_payload_and_handle_categories"],
  ["A3", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs", "direct_handle_faults_bind_source_operations_statuses_and_atomic_cleanup"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/boundaries.rs", "control_model_requires_exact_independent_surviving_owner_contract"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/boundaries.rs", "control_model_allocation_failure_retains_embedded_handle_until_successful_retry"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests.rs", "control_model_clone_expiration_and_payload_before_implicit_weak_finish"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests.rs", "control_model_provenance_topology_modes_and_allocation_failures_are_atomic"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "all_control_transitions_and_illegal_variants_are_checked"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "non_success_control_results_are_zero_shaped"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "failure_atomicity_is_bound_to_the_exact_operation_status_set"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "layout_binding_rejects_target_and_fingerprint_mismatch"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_graph_forged_future_cycles_and_pending_interposition_reject_exactly"],
  ["A3", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_status_corruption_is_not_expiration_or_a_language_trap"],
  ["A4", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests.rs", "control_model_immutable_handle_graph_and_recursive_release_are_exact"],
  ["A4", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_graph_distinct_cloned_edges_and_weak_observer_release_completely"],
  ["A4", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_graph_forged_future_cycles_and_pending_interposition_reject_exactly"],
  ["A4", "held-credit/planner control", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_reachability.rs", "handle_reachability_is_linear_for_deep_diamonds_and_cycles"],
  ["A5", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "shared_and_weak_structural_payload_categories_lower_through_verified_ir"],
  ["A5", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_payload_closure.rs", "recursive_and_multi_variant_enum_payloads_lower_with_exact_cleanup_and_replay"],
  ["A5", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_fault_oracle.rs", "structural_handle_fault_ordinals_bind_prefix_cleanup_and_source_retention"],
  ["A5", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/handle_frontier_source.rs", "handle_frontier_source_vec_enum_occurrences_retain_replacement_owners"],
  ["A5", "verified IR", "crates/zryna-ir/src/data_ownership_v1/tests/handle_aware_clone.rs", "seals_shared_and_weak_recursive_clone_recipe_without_unfolding_vec_cycles"],
  ["A5", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/boundaries.rs", "control_model_payload_shapes_derive_reverse_active_cleanup_and_vec_storage_last"],
  ["A6", "held-credit/planner control", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs", "shared_construct_cleanup_frontier_is_exact_atomic_and_recovers"],
  ["A6", "held-credit/planner control", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs", "temporary_handle_drop_transition_has_exact_extra_boundary_and_recovers"],
  ["A6", "held-credit/planner control", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_resources.rs", "handle_aware_structural_clone_has_exact_cleanup_frontier_and_pristine_retry"],
  ["A6", "held-credit/planner control", "crates/zryna-semantics/src/data_ownership_v1/tests/weak_upgrade_resources.rs", "weak_upgrade_exact_extra_overflow_resources_restore_pristine_state"],
  ["A6", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/resources.rs", "control_model_internal_resource_boundaries_publish_nothing_on_rejection"],
  ["A6", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "vec_transitions_use_sealed_stride_and_checked_byte_amplification"],
  ["A6", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "bound_vec_claim_rejects_cross_target_and_element_replay"],
  ["A6", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "byte_amplification_violations_replay_deterministically"],
  ["A6", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/tests.rs", "exact_contract_seals_all_declarations_and_layout_metadata"],
  ["A7", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/shared_weak_source.rs", "moved_handle_clone_reports_exact_diagnostic_and_replays"],
  ["A7", "authenticated source program", "crates/zryna-semantics/src/data_ownership_v1/tests/weak_upgrade_source.rs", "weak_upgrade_binding_collision_is_exact_deterministic_and_recovers"],
  ["A7", "verified IR", "crates/zryna-ir/src/data_ownership_v1/tests/shared_weak_authority.rs", "shared_weak_cleanup_rejects_missing_reordered_returned_and_foreign_owners"],
  ["A7", "synthetic ABI counter", "crates/zryna-ownership-runtime-abi/src/control_model/tests/conformance_graph.rs", "conformance_status_corruption_is_not_expiration_or_a_language_trap"],
];

function tableRows(section) {
  return section
    .split("\n")
    .filter(line => /^\| A[1-7] \|/.test(line))
    .map(line => line.split("|").slice(1, -1).map(cell => cell.trim()));
}

function testNames(file) {
  return new Set([...read(file).matchAll(
    /#\[test\]\s*(?:#\[[^\]]*\]\s*)*fn\s+([a-z][a-z0-9_]+)\s*\(/g,
  )].map(match => match[1]));
}

export function validateSharedWeakResourceLedger(text) {
  const section = text.split("## Issue #263 checked resource and evidence ledger\n")[1]
    ?.split("## Issue #262 source upgrade closure candidate")[0];
  assert(section, "Issue #263 ledger is missing");
  const [overview, remainder] = section.split("### Executable bindings\n");
  assert(remainder, "Issue #263 executable bindings are missing");
  const [bindings, boundaries] = remainder.split("### Coupled maxima and exclusions\n");
  assert(boundaries, "Issue #263 coupled maxima and exclusions are missing");

  assert.deepEqual(
    tableRows(overview).map(row => row[0]),
    ["A1", "A2", "A3", "A4", "A5", "A6", "A7"],
    "Issue #263 acceptance rows drifted",
  );
  const actualBindings = tableRows(bindings).map(row => [
    row[0],
    row[1].replaceAll("`", ""),
    row[2].replaceAll("`", ""),
    row[3].replaceAll("`", ""),
  ]);
  assert.deepEqual(actualBindings, expectedBindings, "Issue #263 evidence bindings drifted");

  const inventories = new Map();
  for (const [, evidenceClass, file, name] of actualBindings) {
    assert([
      "authenticated source program",
      "verified IR",
      "held-credit/planner control",
      "synthetic ABI counter",
    ].includes(evidenceClass), `unknown Issue #263 evidence class: ${evidenceClass}`);
    if (!inventories.has(file)) inventories.set(file, testNames(file));
    assert(inventories.get(file).has(name), `${file}: missing test ${name}`);
  }

  for (const claim of [
    "static per-function instruction limits do not by themselves impose a dynamic\npopulation cap",
    "One symbolic control trace is bounded\nto 4,194,304 status transitions",
    "1,048,576 allocation-operation/live-\nallocation",
    "per-function value, place, ownership-transition, cleanup-action and cleanup-plan maxima are\ncoupled",
    "does not execute allocation, mutate a concrete\nstrong or weak count",
    "does not claim Issue #263,\nIssue #83, or the M3 profile closed",
  ]) assert(boundaries.includes(claim), `Issue #263 boundary claim drifted: ${claim}`);
}

test("shared and weak resource ledger resolves every acceptance row to exact tests", () => {
  validateSharedWeakResourceLedger(document);
});

test("shared and weak resource ledger fails closed on stale evidence or boundary claims", () => {
  assert.throws(() => validateSharedWeakResourceLedger(document.replace(
    "moved_handle_clone_reports_exact_diagnostic_and_replays",
    "missing_shared_weak_test",
  )), /evidence bindings drifted/);
  assert.throws(() => validateSharedWeakResourceLedger(document.replace(
    "| A4 | `held-credit/planner control` |",
    "| A4 | `runtime execution` |",
  )), /evidence bindings drifted/);
  assert.throws(() => validateSharedWeakResourceLedger(document.replace(
    "does not execute allocation",
    "may execute allocation",
  )), /boundary claim drifted/);
  assert.throws(() => validateSharedWeakResourceLedger(document.replace(
    "static per-function instruction limits do not by themselves impose a dynamic",
    "static instruction limits impose a dynamic",
  )), /boundary claim drifted/);
});

test("shared and weak scope documents keep Issue #263 non-executable", () => {
  for (const [file, text] of scopeDocuments) {
    for (const stale of [
      "target execution remains owned by #263",
      "executed target behavior remains #263",
      "Executed count/allocation faults remain #263",
      "target outcome execution remains the #263 boundary",
      "Target execution remains #263 work",
    ]) assert(!text.includes(stale), `${file}: stale Issue #263 target claim`);
  }
  assert(scopeDocuments.every(([, text]) => text.includes("#263")));
  assert(scopeDocuments.every(([, text]) => text.includes("non-executable")));
});
