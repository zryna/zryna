import assert from "node:assert/strict";
import test from "node:test";
import { m3IssueGraph, validateM3IssueOrder } from "../scripts/lib/m3-issue-graph.mjs";
import { loadAndValidateM3Contract, validateM3Contract } from "../scripts/check-m3-contract.mjs";

test("M3 completion dependencies use topological rather than issue-number order", () => {
  validateM3IssueOrder(m3IssueGraph);
  validateM3IssueOrder([{ number: 254, dependsOn: [] }, { number: 84, dependsOn: [254] }]);
  const graph = loadAndValidateM3Contract().issues;
  for (const target of [84, 85, 86]) {
    const dependencies = graph.find(issue => issue.number === target).dependsOn;
    for (const prerequisite of [254, 255, 256]) assert(dependencies.includes(prerequisite));
  }
  assert.deepEqual(graph.find(issue => issue.number === 83).dependsOn, [80, 81, 82]);
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
