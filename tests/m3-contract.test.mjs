import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import test from "node:test";

import {
  expectedRegistrySha256,
  loadAndValidateM3Contract,
  validateM3Contract,
} from "../scripts/check-m3-contract.mjs";

function clonedContract() {
  return structuredClone(loadAndValidateM3Contract());
}

test("digest-pins the real M3 issue graph and regression authorities", () => {
  const contract = loadAndValidateM3Contract();
  assert.equal(expectedRegistrySha256.length, 64);
  assert.equal(contract.status, "specified-not-implemented");
  assert.deepEqual(
    contract.issues.map(({ number }) => number),
    [75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90],
  );
  assert.deepEqual(
    contract.issues.find(({ number }) => number === 88).dependsOn,
    [76, 84, 85, 87],
  );
  assert.deepEqual(contract.regressionAuthorities, [
    "m0",
    "m1-i32-v1",
    "m2-control-flow-v1",
  ]);
});

test("freezes layout ownership runtime failure target and budget inventories", () => {
  const contract = loadAndValidateM3Contract();
  assert(contract.layoutRules.includes("canonical-structural-type-ids"));
  assert(contract.layoutRules.includes("checked-u64-arithmetic"));
  assert.deepEqual(contract.ownershipStates, [
    "uninitialized",
    "initialized",
    "shared-borrowed(k)",
    "exclusive-borrowed",
    "moved",
    "dropped",
  ]);
  assert.deepEqual(contract.runtimeAbi.operations, [
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
  ]);
  assert.deepEqual(contract.runtimeStatuses, [
    "OK",
    "ALLOCATION",
    "CAPACITY",
    "REFCOUNT",
    "UTF8",
    "EXPIRED",
    "ABI_VIOLATION",
  ]);
  assert.deepEqual(contract.limits.allocationAlignmentBytes, [1, 2, 4, 8]);
  assert.equal(contract.limits.aggregateConstructionOperands, 262144);
  assert.equal(contract.limits.runtimeStatusTransitionsPerInvocation, 4194304);
  assert.equal(contract.limits.wasmMemoryMinimumPages, 1);
  assert.equal(contract.limits.wasmMemoryMaximumPages, 32768);
  assert.equal(contract.runtimeAbi.exposesRustLayout, false);
  assert.deepEqual(contract.targetRepresentations, [
    "javascript-private-dense",
    "webassembly-linear32-v1",
    "native-linux-x86-64-v1",
  ]);
  assert(contract.unsupported.includes("tracing-gc"));
  assert(contract.unsupported.includes("freestanding-targets"));
});

test("rejects every authority class when the canonical contract drifts", () => {
  for (const mutate of [
    (contract) => {
      contract.unknown = true;
    },
    (contract) => {
      contract.profile = "different";
    },
    (contract) => {
      contract.status = "implemented";
    },
    (contract) => {
      contract.specifications.pop();
    },
    (contract) => {
      contract.regressionAuthorities[2] = "moving-m2";
    },
    (contract) => {
      contract.issues[13].dependsOn.pop();
    },
    (contract) => {
      contract.types.push(contract.types[0]);
    },
    (contract) => {
      contract.layoutRules[0] = "wrapping-arithmetic";
    },
    (contract) => {
      contract.ownershipStates.reverse();
    },
    (contract) => {
      contract.runtimeAbi.exposesRustLayout = true;
    },
    (contract) => {
      contract.runtimeAbi.operations.reverse();
    },
    (contract) => {
      contract.runtimeStatuses[5] = "INVALID_WEAK_UPGRADE";
    },
    (contract) => {
      contract.limits.staticLayoutBytesPerValue = 0;
    },
    (contract) => {
      contract.limits.allocationAlignmentBytes[2] = 3;
    },
    (contract) => {
      contract.limits.wasmMemoryMaximumPages = 65536;
    },
    (contract) => {
      contract.contractFixtures.checkedArithmetic[0].value = "1";
    },
    (contract) => {
      contract.contractFixtures.recursiveLayouts[0].edges[1][1] = 3;
    },
    (contract) => {
      contract.contractFixtures.ownershipTransitions[0].from = "initialized";
    },
    (contract) => {
      contract.contractFixtures.resourceBoundaries[0].firstExtra = 1026;
    },
    (contract) => {
      contract.plannedCases[0].expected.value = 34;
    },
    (contract) => {
      contract.plannedInvalidCases[0].ownerIssue = 999;
    },
    (contract) => {
      contract.plannedInvalidCases[0].phase = "semantics";
    },
    (contract) => {
      contract.plannedInvalidCases[0].expectedFamily = "ZRYNA-M3xxx";
    },
    (contract) => {
      contract.plannedInvalidCases[4].ownerIssue = 83;
    },
    (contract) => {
      contract.plannedInvalidCases.reverse();
    },
    (contract) => {
      contract.plannedInvalidCases[1] = structuredClone(
        contract.plannedInvalidCases[0],
      );
    },
    (contract) => {
      contract.unsupported.pop();
    },
  ]) {
    const contract = clonedContract();
    mutate(contract);
    assert.throws(() => validateM3Contract(contract), /invalid M3 contract/);
  }
});

test("bounds canonical registry reads and separates structure from digest authentication", async () => {
  const directory = await mkdtemp(path.join(os.tmpdir(), "zryna-m3-contract-"));
  try {
    const changed = clonedContract();
    changed.status = "implemented";
    const changedPath = path.join(directory, "changed.json");
    await writeFile(changedPath, `${JSON.stringify(changed, null, 2)}\n`);
    assert.throws(
      () => loadAndValidateM3Contract(changedPath),
      /registry digest mismatch/,
    );

    const noncanonicalPath = path.join(directory, "noncanonical.json");
    await writeFile(noncanonicalPath, JSON.stringify(clonedContract()));
    assert.throws(
      () =>
        loadAndValidateM3Contract(noncanonicalPath, { verifyDigest: false }),
      /registry bytes are not canonical JSON/,
    );

    const oversizedPath = path.join(directory, "oversized.json");
    await writeFile(oversizedPath, Buffer.alloc(1024 * 1024 + 1, 0x20));
    assert.throws(
      () => loadAndValidateM3Contract(oversizedPath, { verifyDigest: false }),
      /registry exceeds one MiB/,
    );
  } finally {
    await rm(directory, { recursive: true, force: true });
  }
});

test("normative documents preserve old profiles and keep M3 specification-only", async () => {
  const documents = await Promise.all(
    [
      "../spec/language/DATA_OWNERSHIP_V1.md",
      "../spec/memory-model/AGGREGATE_LAYOUT_V1.md",
      "../spec/abi/OWNERSHIP_RUNTIME_V1.md",
    ].map((relative) => readFile(new URL(relative, import.meta.url), "utf8")),
  );
  const normalized = documents.map((document) => document.replace(/\s+/g, " "));
  for (const phrase of [
    "DataOwnershipV1",
    "data-ownership-v1",
    "No compiler",
    "tracing garbage collection",
  ])
    assert(
      normalized.some((document) => document.includes(phrase)),
      `missing phrase: ${phrase}`,
    );
  assert.match(normalized[0], /omitting `--profile` continues to select M1/);
  assert.match(
    normalized[0],
    /`--profile control-flow-v1` continues to select M2/,
  );
  assert.match(normalized[0], /pairScore/);
  assert.match(normalized[1], /checked arithmetic/);
  assert.match(normalized[2], /never exposes Rust `String`/);
});

test("package and preflight expose one focused M3 contract gate", async () => {
  const packageDocument = JSON.parse(
    await readFile(new URL("../package.json", import.meta.url), "utf8"),
  );
  assert.equal(
    packageDocument.scripts["m3:contract"],
    "node scripts/check-m3-contract.mjs",
  );
  assert.match(
    packageDocument.scripts["docs:check"],
    /tests\/m3-contract\.test\.mjs/,
  );
  const preflight = await readFile(
    new URL("../scripts/run-preflight.mjs", import.meta.url),
    "utf8",
  );
  assert.match(preflight, /tests\/m3-contract\.test\.mjs/);
});
