import { createHash } from "node:crypto";
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readdirSync,
  readSync,
} from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const workspaceRoot = fileURLToPath(new URL("../..", import.meta.url));
const maximumFileBytes = 1024 * 1024;
const fixturePrefixes = [
  "borrow-call-",
  "borrow-forwarding-",
  "borrow-parameter-",
  "lexical-borrow-call-",
];
const expectedSectionSha256 =
  "ca7ca013771f8ebb0ddc3f7791bc46db6378892e89f3e8e570a44e42e687fc20";

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
  if (JSON.stringify(actual) !== JSON.stringify(expected)) fail(`${label} drifted`);
}

function uniqueStrings(values, label) {
  if (!Array.isArray(values) || values.length === 0) fail(`${label} is empty`);
  if (values.some((value) => typeof value !== "string" || value.length === 0))
    fail(`${label} contains a non-string identity`);
  if (new Set(values).size !== values.length) fail(`${label} contains duplicates`);
}

function sameIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino && left.mode === right.mode;
}

function sameState(left, right) {
  return (
    sameIdentity(left, right) &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs
  );
}

function readStableFile(filePath, label) {
  const pathState = lstatSync(filePath, { bigint: true });
  if (pathState.isSymbolicLink() || !pathState.isFile())
    fail(`${label} is not a regular file`);
  let descriptor;
  try {
    descriptor = openSync(filePath, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
    const opened = fstatSync(descriptor, { bigint: true });
    if (!opened.isFile() || !sameIdentity(pathState, opened))
      fail(`${label} identity changed while opening`);
    if (opened.size > BigInt(maximumFileBytes)) fail(`${label} exceeds one MiB`);
    const bounded = Buffer.alloc(maximumFileBytes + 1);
    let length = 0;
    while (length < bounded.length) {
      const count = readSync(descriptor, bounded, length, bounded.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length > maximumFileBytes) fail(`${label} exceeds one MiB`);
    const final = fstatSync(descriptor, { bigint: true });
    const finalPath = lstatSync(filePath, { bigint: true });
    if (
      !sameState(opened, final) ||
      final.size !== BigInt(length) ||
      finalPath.isSymbolicLink() ||
      !sameState(final, finalPath)
    ) {
      fail(`${label} changed while reading`);
    }
    return bounded.subarray(0, length);
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

export function borrowCallFixtureInventory(
  directory = path.join(workspaceRoot, "tests", "m3-fixtures"),
  prefix = "tests/m3-fixtures",
) {
  const identities = new Set();
  const entries = readdirSync(directory, { withFileTypes: true }).filter((entry) =>
    fixturePrefixes.some((fixturePrefix) => entry.name.toLowerCase().startsWith(fixturePrefix)),
  );
  for (const entry of entries) {
    const folded = entry.name.toLowerCase();
    if (identities.has(folded)) fail("borrow-call fixture inventory has an ASCII case collision");
    identities.add(folded);
  }
  const files = [];
  for (const entry of entries) {
    if (!/^[a-z0-9-]+\.(?:json|zry)$/u.test(entry.name))
      fail(`borrow-call fixture ${entry.name} is not a canonical portable filename`);
    if (entry.isSymbolicLink() || !entry.isFile())
      fail(`borrow-call fixture ${entry.name} is not a regular file`);
    files.push(`${prefix}/${entry.name}`);
  }
  return files.sort();
}

function canonicalFixturePath(value) {
  return (
    typeof value === "string" &&
    value === path.posix.normalize(value) &&
    !path.posix.isAbsolute(value) &&
    /^tests\/m3-fixtures\/(?:borrow-call-|borrow-forwarding-|borrow-parameter-|lexical-borrow-call-)[a-z0-9-]*\.(?:json|zry)$/u.test(
      value,
    )
  );
}

function rustTestSelectorExists(source, selector) {
  const name = selector.split("::").at(-1);
  if (!name || !/^[A-Za-z_][A-Za-z0-9_]*$/u.test(name)) return false;
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/gu, "\\$&");
  const declaration = new RegExp(
    `((?:^[ \\t]*#\\[[^\\]\\r\\n]+\\][ \\t]*\\r?\\n)+)[ \\t]*fn[ \\t]+${escaped}[ \\t]*\\(`,
    "gmu",
  );
  return [...source.matchAll(declaration)].some((match) =>
    /^[ \\t]*#\[test\][ \\t]*$/mu.test(match[1]),
  );
}

function validateSnapshot(snapshotPath, sourceBytes) {
  const bytes = readStableFile(path.join(workspaceRoot, snapshotPath), snapshotPath);
  let snapshot;
  try {
    snapshot = JSON.parse(new TextDecoder("utf-8", { fatal: true }).decode(bytes));
  } catch (error) {
    fail(`borrow-call snapshot ${snapshotPath} is not strict UTF-8 JSON: ${error.message}`);
  }
  if (!bytes.equals(Buffer.from(`${JSON.stringify(snapshot)}\n`)))
    fail(`borrow-call snapshot ${snapshotPath} is not canonical worker JSON`);
  if (
    snapshot.schema_version !== 4 ||
    !Array.isArray(snapshot.files) ||
    snapshot.files.length !== 1 ||
    snapshot.files[0]?.path !== "src/main.zry" ||
    !Array.isArray(snapshot.diagnostics) ||
    snapshot.diagnostics.length !== 0
  ) {
    fail(`borrow-call snapshot ${snapshotPath} is not one diagnostic-free protocol-v4 unit`);
  }
  if (sourceBytes.length === 0) fail(`borrow-call source paired with ${snapshotPath} is empty`);
}

function fixtureBytesAndHashes(section, fixturePaths) {
  if (!Array.isArray(section.fixtureFiles) || section.fixtureFiles.length === 0)
    fail("borrow-call fixture files are empty");
  const registeredPaths = section.fixtureFiles.map(({ path: fixturePath }) => fixturePath);
  if (JSON.stringify(registeredPaths) !== JSON.stringify(fixturePaths))
    fail("borrow-call fixture inventory drifted");
  const exactPaths = new Set();
  const foldedPaths = new Set();
  const fixtures = new Map();
  for (const [index, fixture] of section.fixtureFiles.entries()) {
    exactKeys(fixture, ["path", "sha256"], `borrow-call fixture #${index}`);
    if (!canonicalFixturePath(fixture.path)) fail(`borrow-call fixture #${index} path is invalid`);
    if (exactPaths.has(fixture.path)) fail("borrow-call fixture registry contains a duplicate path");
    exactPaths.add(fixture.path);
    const folded = fixture.path.toLowerCase();
    if (foldedPaths.has(folded))
      fail("borrow-call fixture registry contains an ASCII case collision");
    foldedPaths.add(folded);
    if (!/^[0-9a-f]{64}$/u.test(fixture.sha256))
      fail(`borrow-call fixture ${fixture.path} hash is invalid`);
    const bytes = readStableFile(path.join(workspaceRoot, fixture.path), fixture.path);
    if (createHash("sha256").update(bytes).digest("hex") !== fixture.sha256)
      fail(`borrow-call fixture ${fixture.path} hash drifted`);
    fixtures.set(fixture.path, bytes);
  }
  return fixtures;
}

function semanticCases(section, fixturePaths, fixtures) {
  if (!Array.isArray(section.acceptedCases) || section.acceptedCases.length === 0)
    fail("borrow-call accepted cases are empty");
  if (!Array.isArray(section.exclusions) || section.exclusions.length === 0)
    fail("borrow-call exclusions are empty");
  const semanticsTests = new TextDecoder("utf-8", { fatal: true }).decode(
    readStableFile(
      path.join(
        workspaceRoot,
        "crates/zryna-semantics/src/data_ownership_v1/tests/borrow_call_conformance.rs",
      ),
      "borrow-call semantic evidence",
    ),
  );
  const caseIds = [];
  const acceptedIds = [];
  const excludedIds = [];
  const pairedPaths = [];
  for (const [index, accepted] of section.acceptedCases.entries()) {
    exactKeys(
      accepted,
      ["id", "source", "snapshot", "coverage", "rustTest"],
      `borrow-call accepted case #${index}`,
    );
    if (
      typeof accepted.id !== "string" ||
      accepted.source !== `tests/m3-fixtures/${accepted.id}.zry` ||
      accepted.snapshot !== `tests/m3-fixtures/${accepted.id}.json` ||
      !fixtures.has(accepted.source) ||
      !fixtures.has(accepted.snapshot)
    ) {
      fail(`borrow-call accepted case #${index} pair drifted`);
    }
    uniqueStrings(accepted.coverage, `borrow-call accepted case ${accepted.id} coverage`);
    if (!rustTestSelectorExists(semanticsTests, accepted.rustTest))
      fail(`borrow-call accepted case ${accepted.id} lacks registered Rust evidence`);
    validateSnapshot(accepted.snapshot, fixtures.get(accepted.source));
    caseIds.push(accepted.id);
    acceptedIds.push(accepted.id);
    pairedPaths.push(accepted.source, accepted.snapshot);
  }
  for (const [index, excluded] of section.exclusions.entries()) {
    exactKeys(
      excluded,
      [
        "id",
        "category",
        "source",
        "snapshot",
        "diagnostics",
        "recovery",
        "ownerIssue",
        "rationale",
      ],
      `borrow-call exclusion #${index}`,
    );
    if (
      typeof excluded.id !== "string" ||
      excluded.source !== `tests/m3-fixtures/${excluded.id}.zry` ||
      excluded.snapshot !== `tests/m3-fixtures/${excluded.id}.json` ||
      !fixtures.has(excluded.source) ||
      !fixtures.has(excluded.snapshot)
    ) {
      fail(`borrow-call exclusion #${index} pair drifted`);
    }
    if (
      typeof excluded.category !== "string" ||
      excluded.category.length === 0 ||
      excluded.ownerIssue !== 119 ||
      typeof excluded.rationale !== "string" ||
      excluded.rationale.length === 0
    ) {
      fail(`borrow-call exclusion ${excluded.id} metadata is invalid`);
    }
    if (!Array.isArray(excluded.diagnostics) || excluded.diagnostics.length === 0)
      fail(`borrow-call exclusion ${excluded.id} diagnostics are empty`);
    const sourceBytes = fixtures.get(excluded.source);
    for (const [diagnosticIndex, diagnostic] of excluded.diagnostics.entries()) {
      exactKeys(
        diagnostic,
        ["code", "message", "guidance", "span"],
        `borrow-call exclusion ${excluded.id} diagnostic #${diagnosticIndex}`,
      );
      exactKeys(
        diagnostic.span,
        ["path", "start", "end"],
        `borrow-call exclusion ${excluded.id} span #${diagnosticIndex}`,
      );
      if (
        !/^ZRYNA-[A-Z][0-9]{4}$/u.test(diagnostic.code) ||
        typeof diagnostic.message !== "string" ||
        diagnostic.message.length === 0 ||
        typeof diagnostic.guidance !== "string" ||
        diagnostic.guidance.length === 0 ||
        diagnostic.span.path !== "src/main.zry" ||
        !Number.isSafeInteger(diagnostic.span.start) ||
        !Number.isSafeInteger(diagnostic.span.end) ||
        diagnostic.span.start < 0 ||
        diagnostic.span.start >= diagnostic.span.end ||
        diagnostic.span.end > sourceBytes.length
      ) {
        fail(`borrow-call exclusion ${excluded.id} diagnostic #${diagnosticIndex} is invalid`);
      }
    }
    exactKeys(
      excluded.recovery,
      ["acceptedFixture", "expectation", "rustTest"],
      `borrow-call exclusion ${excluded.id} recovery`,
    );
    if (
      excluded.recovery.acceptedFixture !== "borrow-forwarding-shared" ||
      excluded.recovery.expectation !== "same-verified-program" ||
      !rustTestSelectorExists(semanticsTests, excluded.recovery.rustTest)
    ) {
      fail(`borrow-call exclusion ${excluded.id} recovery drifted`);
    }
    validateSnapshot(excluded.snapshot, sourceBytes);
    caseIds.push(excluded.id);
    excludedIds.push(excluded.id);
    pairedPaths.push(excluded.source, excluded.snapshot);
  }
  uniqueStrings(caseIds, "borrow-call case ids");
  if (
    JSON.stringify(acceptedIds) !== JSON.stringify([...acceptedIds].sort()) ||
    JSON.stringify(excludedIds) !== JSON.stringify([...excludedIds].sort())
  ) {
    fail("borrow-call cases are not in canonical id order");
  }
  if (JSON.stringify([...pairedPaths].sort()) !== JSON.stringify(fixturePaths))
    fail("borrow-call fixture pairs do not cover the complete inventory exactly once");
}

function namedEvidence(section) {
  const irTests = new TextDecoder("utf-8", { fatal: true }).decode(
    readStableFile(
      path.join(workspaceRoot, "crates/zryna-ir/src/data_ownership_v1/tests.rs"),
      "borrow-call IR evidence",
    ),
  );
  if (!Array.isArray(section.verifierEvidence) || section.verifierEvidence.length !== 5)
    fail("borrow-call verifier evidence drifted");
  for (const [index, evidence] of section.verifierEvidence.entries()) {
    exactKeys(evidence, ["id", "rustTest", "expectation"], `verifier evidence #${index}`);
    if (
      typeof evidence.id !== "string" ||
      typeof evidence.expectation !== "string" ||
      !rustTestSelectorExists(irTests, evidence.rustTest)
    ) {
      fail(`borrow-call verifier evidence ${evidence.id} is invalid`);
    }
  }
  uniqueStrings(section.verifierEvidence.map(({ id }) => id), "borrow-call verifier evidence ids");

  const resourceTests = new TextDecoder("utf-8", { fatal: true }).decode(
    readStableFile(
      path.join(
        workspaceRoot,
        "crates/zryna-semantics/src/data_ownership_v1/tests/lexical_borrow_calls.rs",
      ),
      "borrow-call resource evidence",
    ),
  );
  if (!Array.isArray(section.resourceEvidence) || section.resourceEvidence.length !== 4)
    fail("borrow-call resource evidence drifted");
  for (const [index, evidence] of section.resourceEvidence.entries()) {
    exactKeys(
      evidence,
      ["id", "rustTest", "dimensions", "boundary"],
      `resource evidence #${index}`,
    );
    uniqueStrings(evidence.dimensions, `resource evidence ${evidence.id} dimensions`);
    if (
      typeof evidence.id !== "string" ||
      typeof evidence.boundary !== "string" ||
      !rustTestSelectorExists(resourceTests, evidence.rustTest)
    ) {
      fail(`borrow-call resource evidence ${evidence.id} is invalid`);
    }
  }
  uniqueStrings(section.resourceEvidence.map(({ id }) => id), "borrow-call resource evidence ids");
}

function nonCapabilities(section) {
  if (!Array.isArray(section.nonCapabilities) || section.nonCapabilities.length !== 8)
    fail("borrow-call non-capabilities drifted");
  for (const [index, item] of section.nonCapabilities.entries()) {
    exactKeys(item, ["id", "ownerIssue", "rationale"], `non-capability #${index}`);
    if (
      typeof item.id !== "string" ||
      item.ownerIssue !== 119 ||
      typeof item.rationale !== "string" ||
      item.rationale.length === 0
    ) {
      fail(`borrow-call non-capability #${index} is invalid`);
    }
  }
  uniqueStrings(section.nonCapabilities.map(({ id }) => id), "borrow-call non-capability ids");
}

export function validateM3BorrowCallConformance(
  section,
  fixturePaths = borrowCallFixtureInventory(),
) {
  exactKeys(
    section,
    [
      "schemaVersion",
      "protocolVersion",
      "fixturePrefixes",
      "fixtureFiles",
      "acceptedCases",
      "exclusions",
      "verifierEvidence",
      "resourceEvidence",
      "nonCapabilities",
    ],
    "borrow-call conformance",
  );
  if (section.schemaVersion !== 1 || section.protocolVersion !== 4)
    fail("borrow-call conformance version drifted");
  exactArray(section.fixturePrefixes, fixturePrefixes, "borrow-call fixture prefixes");
  const fixtures = fixtureBytesAndHashes(section, fixturePaths);
  semanticCases(section, fixturePaths, fixtures);
  namedEvidence(section);
  nonCapabilities(section);
  const digest = createHash("sha256").update(JSON.stringify(section)).digest("hex");
  if (digest !== expectedSectionSha256) fail("borrow-call conformance oracle drifted");
}
