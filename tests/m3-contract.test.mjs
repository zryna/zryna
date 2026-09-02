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
import {
  borrowCallFixtureInventory,
  validateM3BorrowCallConformance,
} from "../scripts/lib/check-m3-borrow-call-conformance.mjs";

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

test("authenticates the complete borrow-call fixture and exclusion inventory", () => {
  const section = clonedContract().borrowCallConformance;
  assert.equal(section.protocolVersion, 4);
  assert.equal(section.fixtureFiles.length, 36);
  assert.equal(section.acceptedCases.length, 5);
  assert.equal(section.exclusions.length, 13);
  assert.deepEqual(
    section.fixtureFiles.map(({ path: fixturePath }) => fixturePath),
    borrowCallFixtureInventory(),
  );
  assert.doesNotThrow(() => validateM3BorrowCallConformance(section));
  assert.deepEqual(section.verifierEvidence.map(({ id }) => id), [
    "disjoint-projected-authority-remains-ir-verified",
    "mutual-recursion",
    "static-depth-exact-and-first-extra",
    "wrong-access",
    "repeated-exclusive",
  ]);
  assert.deepEqual(section.nonCapabilities.map(({ id }) => id), [
    "syntax-extension",
    "public-borrow-abi",
    "backend-activation",
    "runtime-lifetime-state",
    "projected-derived-forwarding",
    "borrow-return-retention",
    "branch-loop-crossing",
    "owned-aggregate-call-shapes",
  ]);
});

test("rejects borrow-call schema, inventory, hash, diagnostic, recovery, and exclusion drift", () => {
  const fixturePaths = borrowCallFixtureInventory();
  for (const mutate of [
    (section) => {
      section.unknown = true;
    },
    (section) => {
      section.fixtureFiles.shift();
    },
    (section) => {
      section.fixtureFiles.push(structuredClone(section.fixtureFiles[0]));
    },
    (section) => {
      section.fixtureFiles.reverse();
    },
    (section) => {
      section.fixtureFiles[0].path = section.fixtureFiles[0].path.toUpperCase();
    },
    (section) => {
      section.fixtureFiles[0].sha256 = "0".repeat(64);
    },
    (section) => {
      section.acceptedCases[0].coverage.pop();
    },
    (section) => {
      section.exclusions[0].diagnostics[0].message = "changed";
    },
    (section) => {
      section.exclusions[0].diagnostics[0].span.end += 1;
    },
    (section) => {
      section.exclusions[0].recovery.expectation = "changed";
    },
    (section) => {
      section.exclusions[0].ownerIssue = 176;
    },
    (section) => {
      section.verifierEvidence[0].rustTest = "missing_test";
    },
    (section) => {
      section.resourceEvidence[0].dimensions.pop();
    },
    (section) => {
      section.nonCapabilities.pop();
    },
  ]) {
    const section = structuredClone(clonedContract().borrowCallConformance);
    mutate(section);
    assert.throws(
      () => validateM3BorrowCallConformance(section, fixturePaths),
      /invalid M3 contract/,
    );
  }
});

test("rejects unlisted, missing, reordered, and case-colliding borrow-call files", async () => {
  const fixturePaths = borrowCallFixtureInventory();
  const section = clonedContract().borrowCallConformance;
  for (const paths of [
    fixturePaths.slice(1),
    [...fixturePaths, "tests/m3-fixtures/borrow-call-unlisted.zry"],
    [...fixturePaths].reverse(),
  ]) {
    assert.throws(
      () => validateM3BorrowCallConformance(section, paths),
      /borrow-call fixture inventory drifted/,
    );
  }

  const caseCollidingEntries = [
    { name: "borrow-call-case.zry" },
    { name: "Borrow-call-case.zry" },
  ];
  assert.throws(
    () =>
      borrowCallFixtureInventory("unused", "fixtures", () => caseCollidingEntries),
    /ASCII case collision/,
  );
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

test("implemented data IR document freezes the internal authority without runtime activation", async () => {
  const document = await readFile(
    new URL("../docs/M3_DATA_OWNERSHIP_IR.md", import.meta.url),
    "utf8",
  );
  assert.match(document, /expected entry FileId/);
  assert.match(document, /owned Linear32V1 VerifiedLayouts/);
  assert.match(document, /owned LinuxX8664V1 VerifiedLayouts/);
  assert.match(document, /same `SourceMapIdentity`/);
  assert.match(document, /same `TypeUniverseIdentity`/);
  assert.match(document, /ZRYNA-I3001/);
  assert.match(document, /ZRYNA-I3014/);
  assert.match(document, /ZRYNA-I3201/);
  assert.match(document, /ZRYNA-I3202/);
  assert.match(document, /Cleanup plans per function \| 65,536/);
  assert.match(document, /already enforced by the supplied sealed `zryna-layout` authorities/);
  assert.match(document, /derived_drop_actions\(\)/);
  assert.match(document, /moved_projections\(\)/);
  assert.match(document, /initialized_projections\(\)/);
  assert.match(document, /active_variant\(\)/);
  assert.match(document, /`AggregateCloneElementFailure`/);
  assert.match(document, /`AggregateInitializedPrefix`/);
  assert.match(document, /aggregate_clone_element_failure_drop_actions\(\)/);
  assert.match(document, /aggregate_clone_fallible_leaf_count\(\)/);
  assert.match(document, /root Enum's active variant from source ownership state/);
  assert.match(document, /final `Return` may also transfer an exact-topology partial temporary/);
  assert.match(document, /generic consumed-value path remains initialized-only/);
  assert.match(document, /ordered weak-upgrade success\/expired edges/);
  assert.match(document, /separate issue #80 authority now seals the exact runtime/);
  assert.match(document, /M1 and M2 remain the only public compiler profiles/);
});

test("bounded borrowing contract freezes the graph and root Copy producers", async () => {
  const document = await readFile(
    new URL("../docs/M3_BORROWING_SEMANTICS.md", import.meta.url),
    "utf8",
  );
  const status = await readFile(new URL("../docs/STATUS.md", import.meta.url), "utf8");
  const roadmap = await readFile(new URL("../docs/ROADMAP.md", import.meta.url), "utf8");
  const architecture = await readFile(
    new URL("../docs/ARCHITECTURE.md", import.meta.url),
    "utf8",
  );
  const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
  const bundleInventory = await readFile(
    new URL("../docs/website-bundle-v1.json", import.meta.url),
    "utf8",
  );
  const borrowCallConformance = clonedContract().borrowCallConformance;
  assert.match(
    document,
    /Status: Issues #113, #114, #115, #116, #117, #119, #120, and #121 complete/,
  );
  assert.match(document, /#113 -> #114 -> #115 -> \{#116, #117, #119, #120, #121\} -> #122/);
  assert.match(document, /BorrowParameter/);
  assert.match(document, /BeginBorrow\(BorrowDefinition\)/);
  assert.match(document, /BorrowRead/);
  assert.match(document, /BorrowWrite/);
  assert.match(document, /EndBorrow/);
  assert.match(document, /CallArgument::Borrow/);
  assert.match(document, /16,384 simultaneously active borrows per function/);
  assert.match(document, /Unused signature metadata\s+is invalid/);
  assert.match(document, /One private parameter-free straight-line function may now declare/);
  assert.match(document, /exact Copy owner reads while shared aliases\s+are active/);
  assert.match(document, /emits `EndBorrow` in reverse declaration\s+order/);
  assert.match(document, /The producer computes the complete values, places, ownership transitions/);
  assert.match(document, /const alias: BorrowMut<i32> = borrowMut\(root\)/);
  assert.match(document, /assignment is a write through the exclusive authority, not rebinding/);
  assert.match(document, /shared-from-shared reborrow resolves to the same\s+sealed root/);
  assert.match(document, /Each prospective alias is resolved and conflict-checked before receiving its dense planned/);
  assert.match(document, /No raw function, instruction, or program is materialized until the complete plan/);
  assert.match(document, /Mutable-from-shared reborrow and every reborrow from an exclusive alias fail/);
  assert.match(document, /exactly four dense blocks and four edges in entry\/then\/else\/join order/);
  assert.match(document, /borrow authority is never an `OwnershipFlow` value or edge argument/);
  assert.match(document, /active peak is `max\(then, else\)`/);
  assert.match(document, /A complete borrow in only one arm is valid/);
  assert.match(document, /reversing hostile branch targets cannot select a\s+different join-mismatch diagnostic/);
  assert.match(document, /excludes nested or repeated conditionals, loops, calls, parameters/);
  assert.match(document, /## Issue #119 bounded internal call checkpoint/);
  assert.match(document, /private straight-line function may declare a recursively Copy result/);
  assert.match(document, /value parameters and one or more shared or exclusive borrow parameters/);
  assert.match(document, /Source arguments evaluate once from left to right/);
  assert.match(document, /caller\s+retains responsibility for reverse lexical `EndBorrow`/);
  assert.match(document, /accepts static depth\s+128, rejects depth 129/);
  assert.match(document, /32e3f0607389dd1274c21770088456c765ee4fb7/);
  assert.match(document, /d61d1ec50005bbed7d86f029fa6ece5efa7517d495b6aed6e9b0f1c15f69e20f/);
  assert.match(document, /ca7ca013771f8ebb0ddc3f7791bc46db6378892e89f3e8e570a44e42e687fc20/);
  assert.match(document, /exactly 36 source and\s+protocol-v4 snapshot files, 5 accepted cases, and 13 excluded cases/);
  for (const { id } of [
    ...borrowCallConformance.acceptedCases,
    ...borrowCallConformance.exclusions,
  ]) {
    assert(document.includes(`\`${id}\``));
  }
  assert.match(
    document,
    /values, places,\s+ownership transitions, blocks, edges, active borrows, cleanup plans, call edges, and static call\s+depth/,
  );
  assert.match(document, /checked arithmetic before limit selection/);
  for (const command of [
    "pnpm m3:contract",
    "pnpm m3:syntax:quick",
    "pnpm m3:owned:quick",
    "pnpm docs:check",
    "pnpm preflight",
    "pnpm m0:check",
  ]) {
    assert(document.includes(command));
  }
  assert.match(document, /Protocol v4 is consumed unchanged/);
  assert.match(document, /adds no syntax contract, runtime lifetime\s+state, ABI carrier, JavaScript\/WebAssembly\/native lowering/);
  assert.match(document, /## Issue #120 projected-disjointness checkpoint/);
  assert.match(document, /same path and every ancestor\/descendant pair overlap/);
  assert.match(document, /distinct static siblings are disjoint/);
  assert.match(document, /positive formula yields 19 values, 14 places, and 38 transitions/);
  assert.match(document, /Dynamic index reasoning,\s+Vec\/enum projected borrowing/);
  assert.match(document, /## Issue #121 loop-edge checkpoint/);
  assert.match(document, /loop body itself is the single lexical borrow scope/);
  assert.match(document, /exactly four dense blocks and four empty-argument edges/);
  assert.match(document, /exact root owner.initialization state is restored/);
  assert.match(document, /M1 and explicit `control-flow-v1` M2 remain the only public profiles/);
  assert.match(status, /Issue #82 is now active through its checked child-issue dependency graph/);
  assert.match(status, /Issue #119 is complete at merged-main provenance/);
  assert.match(status, /exactly 36 source\/snapshot files, 5 accepted\s+cases, and 13 exclusions/);
  assert.match(status, /d61d1ec50005bbed7d86f029fa6ece5efa7517d495b6aed6e9b0f1c15f69e20f/);
  assert.match(status, /ca7ca013771f8ebb0ddc3f7791bc46db6378892e89f3e8e570a44e42e687fc20/);
  assert.match(status, /This adds no runtime,\s+ABI, backend, driver, CLI, artifact, website-support, or public-profile capability/);
  assert.match(
    status,
    /implemented bounded lexical-borrow child\s+slices remain internal; #122 owns aggregate borrowing closure/,
  );
  assert.doesNotMatch(
    status,
    /later dependency-ready issues add\s+bounded lexical borrows/,
  );
  assert.match(roadmap, /Issue #119 completes the bounded private straight-line\s+whole-root call-only nonescape slice/);
  assert.match(roadmap, /\|\s+#82 \| bounded nonescaping lexical borrowing\s+\| #81\s+\| in progress \|/);
  assert.match(
    architecture,
    /Issue #119 adds one bounded private straight-line whole-root direct-call boundary/,
  );
  assert.match(
    architecture,
    /This adds no\s+runtime lifetime state, ABI, JavaScript\/WebAssembly\/native backend path/,
  );
  assert.match(readme, /One bounded private\s+straight-line exception now passes active whole-root shared or exclusive authority/);
  assert.match(
    readme,
    /runtime, ABI, backend, driver, CLI, artifact, and public-profile capability remain unavailable/,
  );
  assert.match(bundleInventory, /"source": "docs\/M3_BORROWING_SEMANTICS\.md"/);
});

test("implemented Copy aggregate semantics remain internal and runtime-free", async () => {
  const document = await readFile(
    new URL("../docs/M3_COPY_AGGREGATE_SEMANTICS.md", import.meta.url),
    "utf8",
  );
  assert.match(document, /verified protocol-v4 syntax snapshot/);
  assert.match(document, /only the sealed\s+`zryna_semantics::data_ownership_v1::VerifiedProgram`/);
  assert.match(document, /retains the mandatory-verifier-approved\s+IR together with the exact verified ownership-runtime ABI declaration authority/);
  assert.match(document, /Raw layout, IR,\s+and runtime declarations remain private/);
  assert.match(document, /only a compile-time constant index/);
  assert.match(document, /negative, nonconstant,\s+and out-of-bounds source indices are semantic errors/);
  assert.match(document, /Reading a Copy aggregate does not\s+move, clone, drop, allocate/);
  assert.match(document, /test-only scalar evaluator/);
  assert.match(document, /ZRYNA-M3002/);
  assert.match(document, /ZRYNA-M3010/);
  assert.match(document, /ZRYNA-M3202/);
  assert.match(document, /retain their owning `ZRYNA-Y4xxx`,\s+`ZRYNA-L3xxx`, and `ZRYNA-I3xxx` codes/);
  assert.match(document, /OwnershipRuntimeV1.*non-executable contract identity/s);
  assert.match(document, /M1 and explicit M2 remain the only public compiler profiles/);
  assert.match(document, /Public aggregate parameters\s+or results/);
  assert.match(document, /implements no allocator or runtime\. No runtime import, heap helper body, target artifact/);
});

test("completed owned-data semantics preserve the bounded internal boundary", async () => {
  const document = await readFile(
    new URL("../docs/M3_OWNED_DATA_SEMANTICS.md", import.meta.url),
    "utf8",
  );
  const architecture = await readFile(
    new URL("../docs/ARCHITECTURE.md", import.meta.url),
    "utf8",
  );
  const status = await readFile(new URL("../docs/STATUS.md", import.meta.url), "utf8");
  const roadmap = await readFile(new URL("../docs/ROADMAP.md", import.meta.url), "utf8");
  const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
  const semanticsReadme = await readFile(
    new URL("../crates/zryna-semantics/README.md", import.meta.url),
    "utf8",
  );
  assert.match(document, /Status: bounded compiler-boundary implementation complete for Issue #81/);
  assert.match(status, /Issue #81 is complete at its bounded internal private compiler boundary/);
  assert.match(status, /Issue #82 is now active through its checked child-issue dependency graph/);
  assert.match(roadmap, /\|\s+#81 \| owned String\/Vec, move checking, and deterministic drop\s+\| #78, #79, #80\s+\| complete\s+\|/);
  assert.match(roadmap, /Issue #82 is active through the checked dependency graph/);
  assert.match(readme, /completed internal Issue #81 boundary/);
  assert.match(semanticsReadme, /completed internal Issue #81 boundary/);
  assert.doesNotMatch(status, /Issue #81 is (?:in progress|not complete)/);
  assert.doesNotMatch(roadmap, /Issue #81 is (?:in progress|not complete)/);
  assert.match(document, /## Current implementation checkpoint/);
  assert.match(document, /String creation from UTF-8 literals, explicit clone, checked concatenation/);
  assert.match(document, /canonical Vec construction, explicit clone of exact `Vec<bool>`, `Vec<i32>`, and `Vec<String>`,\s+local moves, return, push, checked indexing that yields a Copy element/);
  assert.match(document, /replacement of one initialized mutable root-local String/);
  assert.match(document, /replacement of one\s+initialized mutable supported exact Vec root/);
  assert.match(document, /private zero-argument producers and one-argument owned identity calls/);
  assert.match(document, /one bounded top-level no-phi `if`\/`else` for String and exact Vec functions/);
  assert.match(document, /reverse drops\s+of branch-local owners, and exact restoration of every incoming owner/);
  assert.match(document, /one bounded terminal owned `if`\/`else` for private String and exact Vec results/);
  assert.match(document, /canonical one-parameter join; the join owns the\s+selected value exactly once and excludes it from return-site cleanup/);
  assert.match(document, /one bounded top-level no-carried-owner `while` for private String and exact Vec functions/);
  assert.match(document, /condition evaluation in a canonical loop header, reverse drops of every\s+iteration-local owner before the backedge, exact restoration of incoming ownership state/);
  assert.match(document, /push or replacement of an incoming Vec is rejected before its right-hand side/);
  assert.match(document, /bounded construction, whole-value local moves, return, and reverse-order survivor cleanup/);
  assert.match(document, /explicit structural clone of supported non-Copy Struct, FixedArray, and root Enum values/);
  assert.match(document, /fallible-leaf count and root-enum active variant from sealed authorities/);
  assert.match(document, /mutable whole-root assignment for the same supported Struct, FixedArray, and root Enum graphs/);
  assert.match(document, /direct\s+self-consumption is rejected, and `ReplacePlace` commits the prepared owner/);
  assert.match(document, /canonical static struct-field and constant fixed-array projection reads/);
  assert.match(document, /exact String leaves moved once while the enclosing root keeps its masked cleanup obligation/);
  assert.match(document, /one exact direct-local move of a supported acyclic Struct or FixedArray subobject/);
  assert.match(document, /materializing all descendants, masking the whole subtree/);
  assert.match(document, /one exact final return of a supported acyclic Struct or FixedArray subobject/);
  assert.match(document, /parameter-free local root[\s\S]*excluding\s+that returned owner from reverse survivor cleanup/);
  assert.match(document, /explicit clone of initialized available String leaves under those canonical paths/);
  assert.match(document, /explicit clone of at most one initialized available non-Copy Struct or FixedArray subobject/);
  assert.match(document, /immediately following exact same-type local/);
  assert.match(document, /at most one combined private straight-line projected aggregate assignment that moves or explicitly\s+clones one complete static supported Struct\/FixedArray subobject between distinct local roots/);
  assert.match(document, /whole-root clone retains its source and both\s+clone failure paths retain source plus destination/);
  assert.match(document, /Commit drops only the old target subtree while both\s+roots, pending order, and sibling masks survive/);
  assert.match(document, /immediate source operation -> sole-use typed temporary ->\s+`ReplacePlace`/);
  assert.match(document, /exact-type direct local transfer of one partially moved supported Struct or FixedArray root/);
  assert.match(document, /migrating its exact mask from source\s+through the move-result temporary to the new local/);
  assert.match(document, /final exact-reference return of one partially moved supported Struct or FixedArray root/);
  assert.match(document, /migrating the same complete topology and mask into the returned temporary before reverse cleanup/);
  assert.match(document, /whole-root assignment from one distinct exact-reference partially moved supported Struct or\s+FixedArray source/);
  assert.match(document, /drops the old destination and\s+installs the exact mask/);
  assert.match(document, /static projection\s+commit exposes the old subobject's exact pre-state recursive drop action/);
  assert.match(document, /private String use-after-move rejected as `ZRYNA-M3011`, aggregate\/enum moved-owner violations as\s+`ZRYNA-M3014`/);
  assert.match(document, /unresolved binding names as `ZRYNA-M3002`/);
  assert.match(document, /`InitializePlace`, `MoveFromPlace`, and prepare-then-commit `ReplacePlace`/);
  assert.match(document, /one-plan\/one-site cleanup roles/);
  assert.match(document, /cumulative String-literal preflight at 8 MiB/);
  assert.match(document, /sealed semantic `VerifiedProgram` retaining mandatory-verifier-approved IR together with the\s+exact verified ownership-runtime ABI authority/);
  assert.match(document, /aggregate-subobject moves outside one exact direct local, one exact\s+parameter-free final return, or the exact\s+single-variant match-local enum extraction/);
  assert.match(document, /fresh sources, same-root\/overlapping or partial\/moved projected sources,\s+dynamic\/Vec\/Enum targets, projected clone outside one\s+immediate exact same-type local/);
  assert.match(document, /public\s+functions, second move-or-clone assignment sites, and broader aggregate match/);
  assert.match(document, /Owned String\/Vec signatures remain bounded\s+to zero arguments or one exact owned\/bool argument/);
  assert.match(document, /## Issue #81 implementation ledger/);
  assert.match(document, /no-carried-owner loop\/backedge cleanup \| complete/);
  assert.match(document, /exact `Vec<bool>`\/`Vec<i32>`\/`Vec<String>` clone \| complete/);
  assert.match(document, /authenticated allocation and element-clone failures, prefix-safe reverse cleanup/);
  assert.match(document, /supported String-bearing aggregate clone \| complete/);
  assert.match(document, /supported whole-root owned aggregate assignment \| complete/);
  assert.match(document, /static owned projection reads, String-leaf moves, clone, and assignment \| complete/);
  assert.match(document, /direct-local static Struct\/FixedArray subobject move \| complete/);
  assert.match(document, /whole-subtree parent mask, distinct local owner/);
  assert.match(document, /final-return static Struct\/FixedArray subobject move \| complete/);
  assert.match(document, /returned-owner cleanup exclusion, one-site sealing, and atomic value\/place\/transition\/cleanup preflight/);
  assert.match(document, /direct-local static Struct\/FixedArray subobject clone \| complete/);
  assert.match(document, /layout-derived prefix failure cleanup, one-site private straight-line verifier/);
  assert.match(document, /static Struct\/FixedArray subobject move\/clone or whole-root move\/clone into a static projection \| complete/);
  assert.match(document, /distinct-root complete projected move with source-subtree masking/);
  assert.match(document, /atomic `S \+ D \+ T \+ 1` places/);
  assert.match(document, /single-variant Enum payload move through a match-local \| complete/);
  assert.match(document, /active ordinal proof, complete Struct\/FixedArray payload topology/);
  assert.match(document, /direct local transfer of a partial Struct\/FixedArray root \| complete/);
  assert.match(document, /source-to-temporary-to-local owner\/mask migration/);
  assert.match(document, /final return transfer of a partial Struct\/FixedArray root \| complete/);
  assert.match(document, /source-to-return-temporary mask migration, returned-owner exclusion/);
  assert.match(document, /whole-root assignment transfer of a partial Struct\/FixedArray root \| complete/);
  assert.match(document, /source-to-temporary-to-destination mask migration, old-destination recursive drop at commit/);
  assert.match(document, /`3N - E_source - E_target \+ 1` places/);
  assert.match(document, /## Future extensions/);
  assert.match(document, /These extensions require their own dependency-ordered child issues and acceptance evidence/);
  assert.match(document, /`D \+ 1`\s+places/);
  assert.match(document, /projected aggregate clone with `M` missing source-path places and `P` pending owners/);
  assert.match(document, /final-return form of that static subobject move/);
  assert.match(document, /one return cleanup\s+plan and exactly `P` cleanup actions/);
  assert.match(document, /`M \+ 2` places.*two\s+ownership transitions, two cleanup plans, and `2P \+ 1` cleanup actions/);
  assert.match(document, /projected aggregate assignment with `M` missing target-path places/);
  assert.match(document, /`M \+ 1` places[\s\S]*two ownership transitions/);
  assert.match(document, /It adds no cleanup plan or\s+cleanup action/);
  assert.match(document, /static-subobject move form of that combined site/);
  assert.match(document, /`S` is the number of missing canonical\s+source-path places, `D` the missing descendants.*`T` the\s+missing target-path places/);
  assert.match(document, /`S \+ D \+ T \+ 1` places[\s\S]*two ownership\s+transitions \(`MoveFromPlace`, then `ReplacePlace`\)/);
  assert.match(document, /no cleanup plan, cleanup action, or\s+aggregate operand/);
  assert.match(document, /static-subobject clone form/);
  assert.match(document, /`S \+ T \+ 1` places[\s\S]*two ownership transitions \(`ClonePlace`, then `ReplacePlace`\), two cleanup plans, and `2P \+ 1`\s+cleanup actions/);
  assert.match(document, /No source descendants are materialized: sealed layout supplies recursive clone and\s+prefix-drop authority/);
  assert.match(document, /explicit whole-root clone form of that same combined site/);
  assert.match(document, /`M \+ 1` places[\s\S]*two ownership transitions[\s\S]*two cleanup plans, and `2P \+ 1` cleanup actions/);
  assert.match(document, /Prepare failure\s+drops only the pre-existing roots, including source and destination/);
  assert.match(document, /three blocks, two edges, three values, `D \+ 5` places, four\s+ownership transitions, one cleanup plan, and zero cleanup actions/);
  assert.match(architecture, /Vec construction, explicit clone for exact `Vec<bool>`, `Vec<i32>`, and `Vec<String>`/);
  assert.match(architecture, /General structural Vec clone beyond\s+String elements, nested aggregate clone graphs containing Enum, Vec, Shared, or Weak values/);
  assert.match(architecture, /complete static\s+topology for all three roots, then migrates the exact root-relative mask at both owner renames/);
  assert.match(architecture, /final exact-reference return uses the same sealed topology/);
  assert.match(architecture, /single-\s+variant enum is exhaustively matched into an exact direct local/);
  assert.match(architecture, /distinct mutable fully initialized same-type whole-root assignment destination/);
  assert.match(architecture, /supported acyclic Struct or FixedArray subobject/);
  assert.match(architecture, /marks that projection and every descendant moved under the enclosing root/);
  assert.match(architecture, /one initialized available\s+non-Copy Struct or FixedArray projection under those static paths/);
  assert.match(architecture, /independently rejected outside a private one-block context/);
  assert.match(architecture, /at most one combined private straight-line aggregate site/);
  assert.match(architecture, /complete same-type static Struct\/FixedArray subobject between distinct local roots/);
  assert.match(architecture, /unique typed temporary by `ReplacePlace`/);
  assert.match(architecture, /Projected move masks the\s+complete source subtree/);
  assert.match(architecture, /projected clone retains its source without descendant place expansion/);
  assert.match(architecture, /Commit recursively drops only the exact old target\.\s+Projected-subobject move or clone retains both pending roots plus every sibling mask; whole-root clone\s+retains source and destination; whole-root move consumes its source and retains the destination/);
  assert.match(document, /controlled allocation\/capacity\/bounds\/UTF-8 fault closure \| complete/);
  assert.match(document, /authenticated internal fault\/drop traces, including Vec<String> and aggregate-clone partial initialization, cover every admitted operation.*executable target fault injection is outside this compiler boundary/);
  assert.match(document, /test-only fault oracle additionally consumes the ABI authority's sealed status\s+declarations/);
  assert.match(document, /`VecCloneElementFailure`, for the separately authenticated failure of one exact String element\s+clone after a runtime-recorded destination prefix has initialized/);
  assert.match(document, /`AggregateCloneElementFailure`, for the separately authenticated failure of one exact String leaf\s+after a verifier-derived aggregate destination prefix has initialized/);
  assert.match(document, /allocation failure authenticates `VecAllocate`, while an element\s+failure separately authenticates `StringClone`/);
  assert.match(document, /completed prefix must be strictly shorter than that source length before\s+trace allocation/);
  assert.match(document, /zero, middle, last-valid, first-extra, and\s+arithmetic\/event-limit boundaries are deterministic/);
  assert.match(document, /caller-supplied leaf counts or enum variants are not accepted/);
  assert.match(document, /Bounds failure is modeled separately as the verified\s+IR's `BoundsV1` trap/);
  assert.match(document, /does not inject a failure into an allocator or execute a\s+target runtime/);
  assert.match(document, /This ledger records implementation evidence, not public language availability/);
  assert.match(document, /## Issue #81 completed boundary/);
  assert.match(document, /inventory summarizes\s+that compiler-internal boundary; the exclusions in this document remain normative/);
  assert.match(document, /atomic failure is\s+validated against one exact `LogicalOperation`/);
  assert.match(document, /sealed verified element layout whose positive stride is used for checked `capacity \* stride` byte\s+amplification/);
  assert.match(document, /No runtime, backend, driver route, CLI selector, manifest profile,\s+or public aggregate ABI is activated here/);
});

test("implemented ownership-runtime ABI document freezes declarations without runtime activation", async () => {
  const document = await readFile(
    new URL("../docs/M3_OWNERSHIP_RUNTIME_ABI.md", import.meta.url),
    "utf8",
  );
  assert.match(document, /exact\s+identifier `zryna-ownership-runtime-v1`/);
  assert.match(document, /exactly 17 logical operations/);
  assert.match(document, /owned, verified `Linear32V1` and `LinuxX8664V1` layout authorities/);
  assert.match(document, /checked C-header evidence/);
  assert.match(document, /opaque immutable views/);
  assert.match(document, /Vec allocation and reserve plus all 12 canonical Shared\/Weak control/);
  assert.match(document, /Atomic failure validation binds\s+the returned status to one exact logical operation/);
  assert.match(document, /checked `capacity \* stride` amplification/);
  assert.match(document, /opaque `BoundVecTransitionClaim`/);
  assert.match(document, /rejects cross-target or cross-element replay/);
  assert.match(document, /successful no-growth reserve to return\s+the exact old storage pointer/);
  assert.match(document, /legacy `validate_vec_transition` remains\s+layout-bound/);
  assert.match(document, /Raw storage and owned-String behavior are sealed as exact operation/);
  assert.match(document, /`ZRYNA-R3001` covers ABI\s+identity, version, inventory, operation, and symbol/);
  assert.match(document, /`ZRYNA-R3002` covers invalid carriers,\s+signatures, results, records, and transitions/);
  assert.match(document, /ZRYNA-R3201/);
  assert.match(document, /256 operation records/);
  assert.match(document, /4,096 target declarations aggregated/);
  assert.match(document, /65,536 record declarations/);
  assert.match(document, /65,536 nested declaration\s+items/);
  assert.match(document, /16 MiB of checked header bytes/);
  assert.match(document, /256-violation cap/);
  assert.match(document, /relocation\/call-edge and runtime object\/module\s+byte limits remain reserved for later artifact auditors/);
  assert.match(document, /failure\s+returns no partial verified authority/);
  assert.match(document, /no allocator, runtime implementation, target helper body, backend lowering/);
  assert.match(document, /does not activate `data-ownership-v1`/);
  assert.match(document, /does not widen or reinterpret M1,\s+M2, scalar ABI v1/);
});

test("canonical ownership-runtime fixture freezes exact inventories", async () => {
  const fixture = JSON.parse(
    await readFile(
      new URL("../spec/abi/ownership-runtime-v1-fixtures.json", import.meta.url),
      "utf8",
    ),
  );
  const operationOrder = [
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
  ];
  assert.equal(fixture.schemaVersion, 1);
  assert.equal(fixture.abiId, "zryna-ownership-runtime-v1");
  assert.deepEqual(
    fixture.statuses.map(({ number, name }) => [number, name]),
    [[0, "OK"], [1, "ALLOCATION"], [2, "CAPACITY"], [3, "REFCOUNT"], [4, "UTF8"], [5, "EXPIRED"], [255, "ABI_VIOLATION"]],
  );
  assert.deepEqual(fixture.operationOrder, operationOrder);
  assert.deepEqual(fixture.operations.map(({ name }) => name), operationOrder);
  for (const operation of fixture.operations) {
    assert.equal(typeof operation.javascript.result.carrier, "string");
    assert.equal(typeof operation.webAssembly.result.carrier, "string");
    assert.match(operation.nativeLinuxX8664.symbol, /^zryna_rt_o1_[a-z0-9_]+$/);
    assert.equal(operation.nativeLinuxX8664.result.carrier, "u32-status");
  }
  assert.equal(fixture.records.length, 4);
  assert.equal(fixture.transitionCases.length, 12);
  assert.deepEqual(
    fixture.transitionCases.map(({ id }) => id),
    [
      "strong-clone",
      "strong-clone-overflow",
      "weak-downgrade",
      "weak-clone-after-expiry",
      "weak-upgrade",
      "weak-upgrade-expired",
      "strong-release-nonlast",
      "strong-release-last-begin",
      "strong-release-finish-deallocates",
      "strong-release-finish-retains-explicit-weak",
      "weak-release-deallocates",
      "finish-without-pending-last-strong",
    ],
  );
  assert.equal(fixture.limits.runtimeOperations, 256);
  assert.equal(fixture.limits.runtimeSymbols, 4_096);
  assert.equal(fixture.limits.runtimeLayoutReferences, 65_536);
  assert.equal(fixture.limits.runtimeEdges, 65_536);
  assert.equal(fixture.limits.runtimeObjectBytes, 16 * 1024 * 1024);
  assert.equal(fixture.limits.diagnostics, 256);
  assert.ok(fixture.nonCapabilities.includes("runtime-implementation"));
  assert.ok(fixture.nonCapabilities.includes("public-aggregate-host-abi"));
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
  const readme = await readFile(new URL("../README.md", import.meta.url), "utf8");
  assert.match(readme, /pnpm m3:runtime-abi:quick/);
  assert.match(readme, /runs 25 ordinary unit tests and two compile-fail doctests/);
  assert.match(readme, /full `pnpm preflight` gate includes all 28 unit\s+tests and both compile-fail doctests/);
});
