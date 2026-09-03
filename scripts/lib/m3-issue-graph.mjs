export const m3IssueGraph = [
  [75, [], "normative-profile"],
  [76, [75], "syntax-protocol-v4"],
  [77, [75], "verified-layout-authority"],
  [78, [75, 77], "verified-data-ir"],
  [79, [76, 77, 78], "aggregate-semantic-lowering"],
  [80, [75, 77], "ownership-runtime-abi"],
  [81, [78, 79, 80], "owned-data-move-drop"],
  [82, [81], "bounded-borrowing"],
  [259, [80, 81, 82], "shared-weak-interface"],
  [277, [77, 78, 80, 82, 259], "generic-ownership-integration-contract"],
  [260, [80, 81, 82, 259, 277], "verified-shared-weak-transitions"],
  [278, [277], "non-handle-owned-operation-core"],
  [279, [277, 278, 260], "compositional-ownership-cfg-core"],
  [261, [80, 81, 82, 259, 260, 278], "shared-weak-source-handles"],
  [262, [82, 259, 260, 261, 279], "weak-upgrade-source-control-flow"],
  [263, [82, 259, 260, 261, 262], "shared-weak-failure-resources"],
  [264, [80, 81, 82, 259, 260, 261, 262, 263], "shared-weak-integrated-closure"],
  [83, [80, 81, 82, 264], "shared-weak-semantics"],
  [254, [77, 78, 80, 81, 82], "verified-indexed-borrow-authority"],
  [255, [76, 79, 81, 82, 254], "dynamic-fixed-array-borrowing"],
  [256, [76, 80, 81, 82, 254], "vec-element-borrowing"],
  [270, [76, 77, 78, 79, 80, 81, 83, 277, 278], "generic-owned-composition"],
  [271, [270, 279], "structured-owned-control-flow"],
  [272, [270, 271], "owned-calls-module-resolution"],
  [273, [270, 271], "enum-payload-composition"],
  [274, [254, 270], "ordinary-dynamic-array-access"],
  [275, [82, 254, 270, 271, 272, 273], "non-indexed-owned-borrowing"],
  [269, [83, 254, 255, 256, 270, 271, 272, 273, 274, 275, 277, 278, 279], "normative-source-completion"],
  [84, [79, 80, 81, 82, 83, 254, 255, 256, 269], "javascript"],
  [85, [79, 80, 81, 82, 83, 254, 255, 256, 269], "webassembly"],
  [86, [78, 80, 81, 82, 83, 254, 255, 256, 269], "verified-native-mir"],
  [87, [77, 80, 86], "native-object-link-run"],
  [88, [76, 84, 85, 87], "atomic-candidate-manifest-v3"],
  [89, [88], "fixed-oracle-conformance"],
  [90, [89], "public-activation-authenticated-documentation"],
].map(([number, dependsOn, gate]) => Object.freeze({
  number, dependsOn: Object.freeze(dependsOn), gate,
}));
Object.freeze(m3IssueGraph);

// Dependency order is topological, not GitHub issue-number order.
export function validateM3IssueOrder(issues) {
  if (!Array.isArray(issues) || issues.length === 0)
    throw new Error("M3 issue graph must be a nonempty array");
  const seen = new Set();
  for (const issue of issues) {
    if (!issue || !Number.isSafeInteger(issue.number) || issue.number <= 0 || seen.has(issue.number))
      throw new Error("M3 issue graph has an invalid or duplicate issue");
    if (!Array.isArray(issue.dependsOn) || new Set(issue.dependsOn).size !== issue.dependsOn.length)
      throw new Error(`M3 issue ${issue.number} has invalid or duplicate dependencies`);
    if (issue.dependsOn.some(dependency => !seen.has(dependency)))
      throw new Error(`M3 issue ${issue.number} has a non-prior dependency`);
    seen.add(issue.number);
  }
}
