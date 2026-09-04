import { createHash } from "node:crypto";
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

import {
  borrowCallFixtureInventory,
  validateM3BorrowCallConformance,
} from "./lib/check-m3-borrow-call-conformance.mjs";
import { m3IssueGraph, validateM3IssueOrder } from "./lib/m3-issue-graph.mjs";

export const expectedRegistrySha256 =
  "4840114001e53f510a285114a33fb15e3a9599067473e8707d2607333c339d18";
const registryPath = fileURLToPath(
  new URL("../tests/m3-contract-v1.json", import.meta.url),
);
const MAXIMUM_BYTES = 1024 * 1024;

const ROOT_KEYS = [
  "schemaVersion",
  "profile",
  "status",
  "specifications",
  "governanceIssue",
  "regressionAuthorities",
  "issues",
  "types",
  "layoutRules",
  "ownershipStates",
  "ownershipTransitions",
  "runtimeAbi",
  "languageTraps",
  "runtimeStatuses",
  "targetRepresentations",
  "limits",
  "contractFixtures",
  "plannedCases",
  "plannedInvalidCases",
  "borrowCallConformance",
  "unsupported",
];

const U64_MAXIMUM = (1n << 64n) - 1n;

function alignUpU64(value, alignment) {
  if (alignment <= 0n || (alignment & (alignment - 1n)) !== 0n)
    return undefined;
  const remainder = value % alignment;
  const addition = remainder === 0n ? 0n : alignment - remainder;
  return value > U64_MAXIMUM - addition ? undefined : value + addition;
}

function hasDirectedCycle(edges) {
  const adjacency = new Map();
  for (const [from, to] of edges) {
    if (!adjacency.has(from)) adjacency.set(from, []);
    adjacency.get(from).push(to);
    if (!adjacency.has(to)) adjacency.set(to, []);
  }
  const visiting = new Set();
  const complete = new Set();
  function visit(node) {
    if (visiting.has(node)) return true;
    if (complete.has(node)) return false;
    visiting.add(node);
    for (const target of adjacency.get(node)) if (visit(target)) return true;
    visiting.delete(node);
    complete.add(node);
    return false;
  }
  return [...adjacency.keys()].some(visit);
}

function ownershipOutcome(from, operation) {
  if (from === "initialized" && operation === "drop") return "dropped";
  if (from === "moved" && operation === "read") return "reject";
  return undefined;
}

function fail(message) {
  throw new Error(`invalid M3 contract: ${message}`);
}

function exactKeys(value, expected, label) {
  if (!value || typeof value !== "object" || Array.isArray(value))
    fail(`${label} is not an object`);
  if (JSON.stringify(Object.keys(value)) !== JSON.stringify(expected))
    fail(`${label} keys drifted`);
}

function exactArray(actual, expected, label) {
  if (JSON.stringify(actual) !== JSON.stringify(expected))
    fail(`${label} drifted`);
}

function uniqueStrings(values, label) {
  if (!Array.isArray(values) || values.length === 0) fail(`${label} is empty`);
  if (values.some((value) => typeof value !== "string" || value.length === 0))
    fail(`${label} contains a non-string identity`);
  if (new Set(values).size !== values.length)
    fail(`${label} contains duplicates`);
}

function validExpected(expected) {
  exactKeys(expected, ["kind", "type", "value"], "planned case expected");
  return (
    expected.kind === "returned" &&
    expected.type === "i32" &&
    Number.isSafeInteger(expected.value)
  );
}

export function validateM3Contract(
  contract,
  fixturePaths = borrowCallFixtureInventory(),
) {
  exactKeys(contract, ROOT_KEYS, "root");
  if (contract.schemaVersion !== 1) fail("schemaVersion drifted");
  if (contract.profile !== "zryna-data-ownership-v1") fail("profile drifted");
  if (contract.status !== "specified-not-implemented") fail("status drifted");
  exactArray(
    contract.specifications,
    [
      "spec/language/DATA_OWNERSHIP_V1.md",
      "spec/memory-model/AGGREGATE_LAYOUT_V1.md",
      "spec/abi/OWNERSHIP_RUNTIME_V1.md",
    ],
    "specifications",
  );
  if (contract.governanceIssue !== 75) fail("governance issue drifted");
  exactArray(
    contract.regressionAuthorities,
    ["m0", "m1-i32-v1", "m2-control-flow-v1"],
    "regressions",
  );
  exactArray(contract.issues, m3IssueGraph, "issue graph");
  validateM3IssueOrder(contract.issues);
  for (const issue of contract.issues) {
    exactKeys(issue, ["number", "dependsOn", "gate"], `issue ${issue.number}`);
  }
  for (const [label, values] of [
    ["types", contract.types],
    ["layout rules", contract.layoutRules],
    ["ownership states", contract.ownershipStates],
    ["ownership transitions", contract.ownershipTransitions],
    ["language traps", contract.languageTraps],
    ["runtime statuses", contract.runtimeStatuses],
    ["target representations", contract.targetRepresentations],
    ["unsupported capabilities", contract.unsupported],
  ])
    uniqueStrings(values, label);
  exactArray(
    contract.types,
    [
      "bool",
      "i32",
      "struct",
      "enum",
      "fixed-array",
      "String",
      "Vec",
      "Shared",
      "Weak",
      "shared-borrow",
      "exclusive-borrow",
    ],
    "types",
  );
  exactArray(
    contract.layoutRules,
    [
      "canonical-structural-type-ids",
      "checked-u64-arithmetic",
      "source-field-order",
      "align-up-without-wrap",
      "fixed-array-stride",
      "fixed-u32-enum-tag",
      "no-niche-optimization",
      "reject-by-value-recursion",
      "little-endian-storage",
    ],
    "layout rules",
  );
  exactArray(
    contract.ownershipStates,
    [
      "uninitialized",
      "initialized",
      "shared-borrowed(k)",
      "exclusive-borrowed",
      "moved",
      "dropped",
    ],
    "ownership states",
  );
  exactArray(
    contract.ownershipTransitions,
    [
      "initialize",
      "move",
      "borrow-shared",
      "borrow-exclusive",
      "end-borrow",
      "drop",
      "shared-clone",
      "shared-release",
      "weak-downgrade",
      "weak-upgrade",
      "weak-release",
    ],
    "ownership transitions",
  );
  exactKeys(
    contract.runtimeAbi,
    ["id", "operations", "exposesRustLayout"],
    "runtime ABI",
  );
  if (contract.runtimeAbi.id !== "zryna-ownership-runtime-v1")
    fail("runtime ABI id drifted");
  exactArray(
    contract.runtimeAbi.operations,
    [
      "allocate",
      "grow",
      "release",
      "stringFromUtf8Copy",
      "stringClone",
      "stringConcat",
      "stringRelease",
      "vecAllocate",
      "vecReserve",
      "vecReleaseStorage",
      "strongClone",
      "weakDowngrade",
      "weakClone",
      "weakUpgrade",
      "strongReleaseBegin",
      "strongReleaseFinish",
      "weakRelease",
    ],
    "runtime ABI operations",
  );
  if (contract.runtimeAbi.exposesRustLayout !== false)
    fail("runtime ABI exposes a Rust layout");
  exactArray(
    contract.languageTraps,
    [
      "zryna.trap.bounds-v1",
      "zryna.trap.allocation-v1",
      "zryna.trap.capacity-v1",
      "zryna.trap.refcount-v1",
      "zryna.trap.utf8-v1",
    ],
    "language traps",
  );
  exactArray(
    contract.runtimeStatuses,
    [
      "OK",
      "ALLOCATION",
      "CAPACITY",
      "REFCOUNT",
      "UTF8",
      "EXPIRED",
      "ABI_VIOLATION",
    ],
    "runtime statuses",
  );
  exactArray(
    contract.targetRepresentations,
    [
      "javascript-private-dense",
      "webassembly-linear32-v1",
      "native-linux-x86-64-v1",
    ],
    "target representations",
  );
  exactKeys(
    contract.limits,
    [
      "nominalDeclarationsPerProgram",
      "fullyInstantiatedTypesPerProgram",
      "layoutTypeNodesPerProgram",
      "fieldsAndVariantsPerProgram",
      "fieldsOrVariantsPerDeclaration",
      "aggregateConstructionOperands",
      "layoutDependencyEdges",
      "fixedArrayElements",
      "layoutDepth",
      "storedAlignmentBytes",
      "staticLayoutBytesPerValue",
      "dynamicAllocationBytes",
      "stringBytes",
      "vecElements",
      "allocationAlignmentBytes",
      "strongHandleCount",
      "weakCount",
      "liveAllocationsPerInvocation",
      "allocationGrowthOperationsPerInvocation",
      "runtimeStatusTransitionsPerInvocation",
      "wasmMemoryMinimumPages",
      "wasmMemoryMaximumPages",
      "wasmHeapAlignmentBytes",
      "ownershipPlacesPerFunction",
      "ownershipTransitionsPerFunction",
      "activeBorrowsPerFunction",
      "dropActionsPerFunction",
      "runtimeOperations",
      "runtimeSymbols",
      "runtimeLayoutReferences",
      "runtimeEdges",
      "runtimeObjectBytes",
      "diagnostics",
    ],
    "limits",
  );
  for (const [name, limit] of Object.entries(contract.limits)) {
    if (name === "allocationAlignmentBytes") {
      exactArray(limit, [1, 2, 4, 8], "allocation alignments");
      continue;
    }
    if (!Number.isSafeInteger(limit) || limit <= 0)
      fail(`limit ${name} is invalid`);
  }
  exactArray(
    Object.entries(contract.limits),
    [
      ["nominalDeclarationsPerProgram", 4096],
      ["fullyInstantiatedTypesPerProgram", 65536],
      ["layoutTypeNodesPerProgram", 65536],
      ["fieldsAndVariantsPerProgram", 65536],
      ["fieldsOrVariantsPerDeclaration", 1024],
      ["aggregateConstructionOperands", 262144],
      ["layoutDependencyEdges", 262144],
      ["fixedArrayElements", 1048576],
      ["layoutDepth", 256],
      ["storedAlignmentBytes", 8],
      ["staticLayoutBytesPerValue", 4294967295],
      ["dynamicAllocationBytes", 2147483647],
      ["stringBytes", 2147483647],
      ["vecElements", 1048576],
      ["allocationAlignmentBytes", [1, 2, 4, 8]],
      ["strongHandleCount", 4294967295],
      ["weakCount", 4294967295],
      ["liveAllocationsPerInvocation", 1048576],
      ["allocationGrowthOperationsPerInvocation", 1048576],
      ["runtimeStatusTransitionsPerInvocation", 4194304],
      ["wasmMemoryMinimumPages", 1],
      ["wasmMemoryMaximumPages", 32768],
      ["wasmHeapAlignmentBytes", 8],
      ["ownershipPlacesPerFunction", 65536],
      ["ownershipTransitionsPerFunction", 262144],
      ["activeBorrowsPerFunction", 16384],
      ["dropActionsPerFunction", 262144],
      ["runtimeOperations", 256],
      ["runtimeSymbols", 4096],
      ["runtimeLayoutReferences", 65536],
      ["runtimeEdges", 65536],
      ["runtimeObjectBytes", 16777216],
      ["diagnostics", 256],
    ],
    "limit values",
  );
  exactKeys(
    contract.contractFixtures,
    [
      "checkedArithmetic",
      "recursiveLayouts",
      "ownershipTransitions",
      "resourceBoundaries",
    ],
    "contract fixtures",
  );
  exactArray(
    contract.contractFixtures.checkedArithmetic,
    [
      {
        id: "align-up-u64-overflow",
        operation: "alignUp",
        value: "18446744073709551615",
        alignment: "8",
        expected: "overflow",
      },
    ],
    "checked arithmetic fixtures",
  );
  const arithmetic = contract.contractFixtures.checkedArithmetic[0];
  if (
    alignUpU64(BigInt(arithmetic.value), BigInt(arithmetic.alignment)) !==
    undefined
  )
    fail("checked arithmetic fixture did not overflow");
  exactArray(
    contract.contractFixtures.recursiveLayouts,
    [
      {
        id: "indirect-two-node-cycle",
        edges: [
          [1, 2],
          [2, 1],
        ],
        expected: "reject",
        family: "ZRYNA-L3xxx",
      },
    ],
    "recursive layout fixtures",
  );
  if (!hasDirectedCycle(contract.contractFixtures.recursiveLayouts[0].edges))
    fail("recursive layout fixture is acyclic");
  exactArray(
    contract.contractFixtures.ownershipTransitions,
    [
      {
        id: "read-after-move",
        from: "moved",
        operation: "read",
        expected: "reject",
        family: "ZRYNA-M3xxx",
      },
      {
        id: "drop-initialized",
        from: "initialized",
        operation: "drop",
        expected: "dropped",
      },
    ],
    "ownership transition fixtures",
  );
  for (const transition of contract.contractFixtures.ownershipTransitions) {
    if (ownershipOutcome(transition.from, transition.operation) !== transition.expected)
      fail(`ownership fixture ${transition.id} has the wrong outcome`);
  }
  exactArray(
    contract.contractFixtures.resourceBoundaries,
    [
      {
        id: "fields-per-declaration-limit",
        limitName: "fieldsOrVariantsPerDeclaration",
        exact: 1024,
        firstExtra: 1025,
        exactExpected: "accept",
        firstExtraExpected: "reject",
        family: "ZRYNA-L3xxx",
        ownerIssue: 77,
      },
    ],
    "resource boundary fixtures",
  );
  const boundary = contract.contractFixtures.resourceBoundaries[0];
  if (
    contract.limits[boundary.limitName] !== boundary.exact ||
    boundary.firstExtra !== boundary.exact + 1
  )
    fail("resource boundary fixture does not test exact and first-extra values");
  if (
    !Array.isArray(contract.plannedCases) ||
    contract.plannedCases.length !== 2
  )
    fail("planned cases drifted");
  for (const planned of contract.plannedCases) {
    exactKeys(
      planned,
      ["id", "feature", "inputs", "expected"],
      `planned case ${planned.id}`,
    );
    if (
      planned.feature !== "internal-struct" ||
      !Array.isArray(planned.inputs) ||
      planned.inputs.some((value) => !Number.isSafeInteger(value)) ||
      !validExpected(planned.expected)
    )
      fail(`planned case ${planned.id} is invalid`);
  }
  exactArray(
    contract.plannedCases.map(({ id }) => id),
    ["pair-score", "pair-score-wrapping"],
    "planned case ids",
  );
  exactArray(
    contract.plannedCases.map(({ inputs, expected }) => ({ inputs, expected })),
    [
      {
        inputs: [1, 2],
        expected: { kind: "returned", type: "i32", value: 33 },
      },
      {
        inputs: [2147483647, 1],
        expected: { kind: "returned", type: "i32", value: 2147483618 },
      },
    ],
    "planned case oracles",
  );
  if (
    !Array.isArray(contract.plannedInvalidCases) ||
    contract.plannedInvalidCases.length !== 6
  )
    fail("planned invalid cases drifted");
  for (const invalid of contract.plannedInvalidCases) {
    exactKeys(
      invalid,
      ["id", "phase", "expectedFamily", "ownerIssue"],
      `invalid case ${invalid.id}`,
    );
    if (!contract.issues.some(({ number }) => number === invalid.ownerIssue))
      fail(`invalid case ${invalid.id} is not bound to an owner`);
  }
  uniqueStrings(
    contract.plannedInvalidCases.map(({ id }) => id),
    "planned invalid case ids",
  );
  exactArray(
    contract.plannedInvalidCases,
    [
      {
        id: "recursive-layout",
        phase: "layout",
        expectedFamily: "ZRYNA-L3xxx",
        ownerIssue: 77,
      },
      {
        id: "layout-overflow",
        phase: "layout",
        expectedFamily: "ZRYNA-L3xxx",
        ownerIssue: 77,
      },
      {
        id: "use-after-move",
        phase: "semantics",
        expectedFamily: "ZRYNA-M3xxx",
        ownerIssue: 81,
      },
      {
        id: "borrow-conflict",
        phase: "semantics",
        expectedFamily: "ZRYNA-M3xxx",
        ownerIssue: 82,
      },
      {
        id: "reference-count-overflow",
        phase: "runtime-abi",
        expectedFamily: "ZRYNA-R3xxx",
        ownerIssue: 80,
      },
      {
        id: "fields-per-declaration-limit-plus-one",
        phase: "layout",
        expectedFamily: "ZRYNA-L3xxx",
        ownerIssue: 77,
      },
    ],
    "planned invalid cases",
  );
  validateM3BorrowCallConformance(contract.borrowCallConformance, fixturePaths);
  exactArray(
    contract.unsupported,
    [
      "public-aggregate-abi",
      "tracing-gc",
      "raw-pointers",
      "unsafe",
      "ffi",
      "threads",
      "custom-allocators",
      "wasi",
      "component-model",
      "freestanding-targets",
    ],
    "unsupported capabilities",
  );
  return contract;
}

function sameIdentity(left, right) {
  return (
    left.dev === right.dev && left.ino === right.ino && left.mode === right.mode
  );
}

function sameState(left, right) {
  return (
    sameIdentity(left, right) &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs
  );
}

function readStableFile(filePath, maximumBytes, label) {
  const pathState = lstatSync(filePath, { bigint: true });
  if (pathState.isSymbolicLink() || !pathState.isFile())
    fail(`${label} is not a regular file`);
  let descriptor;
  try {
    descriptor = openSync(
      filePath,
      constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0),
    );
    const opened = fstatSync(descriptor, { bigint: true });
    if (!opened.isFile() || !sameIdentity(pathState, opened))
      fail(`${label} identity changed while opening`);
    if (opened.size > BigInt(maximumBytes)) fail(`${label} exceeds one MiB`);
    const bounded = Buffer.alloc(maximumBytes + 1);
    let length = 0;
    while (length < bounded.length) {
      const count = readSync(
        descriptor,
        bounded,
        length,
        bounded.length - length,
        null,
      );
      if (count === 0) break;
      length += count;
    }
    if (length > maximumBytes) fail(`${label} exceeds one MiB`);
    const final = fstatSync(descriptor, { bigint: true });
    const finalPath = lstatSync(filePath, { bigint: true });
    if (
      !sameState(opened, final) ||
      final.size !== BigInt(length) ||
      finalPath.isSymbolicLink() ||
      !sameState(final, finalPath)
    )
      fail(`${label} changed while reading`);
    return bounded.subarray(0, length);
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

function readCanonicalRegistry(filePath) {
  const bytes = readStableFile(filePath, MAXIMUM_BYTES, "registry");
  let document;
  try {
    document = JSON.parse(
      new TextDecoder("utf-8", { fatal: true }).decode(bytes),
    );
  } catch (error) {
    fail(`registry is not strict UTF-8 JSON: ${error.message}`);
  }
  if (!bytes.equals(Buffer.from(`${JSON.stringify(document, null, 2)}\n`)))
    fail("registry bytes are not canonical JSON");
  return { bytes, document };
}

export function loadAndValidateM3Contract(
  filePath = registryPath,
  { verifyDigest = true } = {},
) {
  const { bytes, document } = readCanonicalRegistry(filePath);
  if (
    verifyDigest &&
    createHash("sha256").update(bytes).digest("hex") !== expectedRegistrySha256
  )
    fail("registry digest mismatch");
  return validateM3Contract(document);
}

const invokedPath = process.argv[1]
  ? pathToFileURL(path.resolve(process.argv[1])).href
  : "";
if (import.meta.url === invokedPath) {
  loadAndValidateM3Contract();
  process.stdout.write(`M3 contract verified: ${expectedRegistrySha256}\n`);
}
