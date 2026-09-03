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
