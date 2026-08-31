import { createHash } from 'node:crypto';
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readSync,
  readdirSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

import {
  expectedRegistrySha256 as historicalRegistrySha256,
  loadAndValidateM2Contract,
} from './check-m2-contract.mjs';
import { M2_QUICK_COMMANDS } from './run-m2-quick.mjs';

export const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const registryPath = path.join(workspaceRoot, 'tests', 'm2-conformance-v1.json');
export const expectedRegistrySha256 =
  'cf07d765c26364cd127b8fdba7d6cefec535876b71916402624c6d77c1140c18';

const maximumRegistryBytes = 1024 * 1024;
const maximumFixtureBytes = 2 * 1024 * 1024;
const hexadecimalSha256 = /^[0-9a-f]{64}$/u;
const expectedGraphSha256 = 'd9cafc44fb02760ed51bf989c0ec4fcf5db9c34918ba6cd07d784eeed1b57dca';
const expectedGraphSources = [
  {
    id: 0,
    path: 'tests/m2-fixtures/valid/main.zry',
    sha256: '833c3e7465ae4328d0ea31a94aa8293f73696e42c0cdef6b0859b60bdc2c30ca',
  },
  {
    id: 1,
    path: 'tests/m2-fixtures/valid/math.zry',
    sha256: 'a833600f480d718c86be001d55644b5e06a2d0cf9c52520f39fdc26e8185328c',
  },
];
const expectedGraphEdges = [
  {
    importer: 'tests/m2-fixtures/valid/main.zry',
    target: 'tests/m2-fixtures/valid/math.zry',
    specifier: './math.zry',
    imported: 'addPair',
    local: 'addPair',
  },
];
const expectedBuildArtifacts = [
  {
    target: 'javascript',
    kind: 'ecmascript-module',
    bytes: 12108,
    sha256: '1cc4ebdca55fde5f0156ccc47218cef68d9dc27a28dc406d2b937dd6f52f14b4',
  },
  {
    target: 'webassembly',
    kind: 'core-webassembly-module',
    bytes: 1276,
    sha256: '483330d8b8ae262dc76d55726b6e9eee89ad5c1f12a5610e23176f9cc74b9f4b',
  },
  {
    target: 'native',
    kind: 'linux-x86-64-relocatable-object',
    bytes: 3688,
    sha256: 'fcd7537b073d2b043c97a874bd7c9b589b68fd3b401a1ff39dcd81291821864a',
  },
];
const expectedInvalidCodes = new Map([
  ['numeric-condition', ['ZRYNA-M2014']],
  ['assign-to-const', ['ZRYNA-M2005']],
  ['use-before-declaration', ['ZRYNA-M2009', 'ZRYNA-M2004', 'ZRYNA-M2004']],
  ['recursive-call-cycle', ['ZRYNA-M2013']],
  ['bare-import', ['ZRYNA-F1103']],
  ['import-escapes-root', ['ZRYNA-D3001']],
  ['missing-import', ['ZRYNA-D3003']],
  ['case-colliding-module', ['ZRYNA-D3005']],
  ['import-cycle', ['ZRYNA-D3007']],
]);
const expectedInvalidDiagnosticSha256 = new Map([
  ['numeric-condition', 'a041271e0f30c0edadb023d499aac40a2c70c42d8dc347a42655fafb200c04f1'],
  ['assign-to-const', 'c46710bc102a6e71ea494fd2ec30f2d583c49bdc6fc902dfbd02d49d20bcf6ba'],
  ['use-before-declaration', '6c5dc21c5205f22b2f80826cf12b91afed9a24fb44788d3fa43dadc22bfbf537'],
  ['recursive-call-cycle', '1d008b27a22f8d40e259fed71e964dae9ab9ce5500f8751636766f02a98f28b8'],
  ['bare-import', '04b0a8437c911b6359d9dcfcb570035125117b8efdb07a92d3f5ab83227e3bdf'],
  ['import-escapes-root', '11c249a0e6c662155e5a5dbedbd12bda24fc4fb33d5eb55f97d83c31d514f7b2'],
  ['missing-import', '15b28fed5492e7e4ad6e0a39144231255bd93c0957cb44281747f089e523fb45'],
  ['case-colliding-module', '6591c979523c2f75cd3598a0c06600e59b7d83d187f31bf34c814a8a0e798fa0'],
  ['import-cycle', '45d4146e47e40312cbcc68aec9e83a3396cef4001ae7bdaadcd48cc4f5109a38'],
]);
const expectedResourceEvidenceSha256 =
  'dd1bbd6040da676828db9684842daa1be319aff20b158fb452b537822ec19614';
const evidenceSourcesByCommand = new Map([
  ['adapter-boundaries', [{ path: 'adapters/typescript-6/test/worker-v3.test.mjs' }]],
  ['driver-pipeline', [{ path: 'crates/zryna-driver/src/pipeline.rs', module: 'pipeline::tests' }]],
  ['module-closure', [
    { path: 'crates/zryna-driver/src/module_closure.rs', module: 'module_closure::tests' },
    { path: 'crates/zryna-driver/src/module_closure_tests.rs', module: 'module_closure_tests' },
  ]],
  ['native-mir', [{ path: 'crates/zryna-native-mir/src/control_flow_v1.rs', module: 'control_flow_v1::tests' }]],
  ['portable-ir', [{ path: 'crates/zryna-ir/src/control_flow_v1/tests.rs', module: 'control_flow_v1::tests' }]],
  ['syntax-boundaries', [{ path: 'crates/zryna-syntax/src/v3.rs', module: 'v3::tests' }]],
  ['control-flow-semantics', [{ path: 'crates/zryna-semantics/src/control_flow_v1.rs', module: 'control_flow_v1::tests' }]],
]);

function fail(message) {
  throw new Error(`invalid M2 executable conformance: ${message}`);
}

function exactKeys(value, keys, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (!isDeepStrictEqual(actual, expected)) fail(`${label} must contain exactly ${expected.join(', ')}`);
}

function sameIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino && left.mode === right.mode;
}

function sameState(left, right) {
  return sameIdentity(left, right) &&
    left.size === right.size &&
    left.mtimeNs === right.mtimeNs &&
    left.ctimeNs === right.ctimeNs;
}

function readStableFile(filePath, maximumBytes, label) {
  const pathState = lstatSync(filePath, { bigint: true });
  if (pathState.isSymbolicLink() || !pathState.isFile()) fail(`${label} is not a real regular file`);
  if (pathState.size > BigInt(maximumBytes)) fail(`${label} exceeds its byte limit`);
  const noFollow = constants.O_NOFOLLOW ?? 0;
  let descriptor;
  try {
    descriptor = openSync(filePath, constants.O_RDONLY | noFollow);
    const opened = fstatSync(descriptor, { bigint: true });
    if (!opened.isFile() || !sameIdentity(pathState, opened)) fail(`${label} identity changed`);
    const bytes = Buffer.alloc(Number(opened.size) + 1);
    let length = 0;
    while (length < bytes.length) {
      const count = readSync(descriptor, bytes, length, bytes.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length > maximumBytes || length !== Number(opened.size)) fail(`${label} length changed`);
    const finalOpened = fstatSync(descriptor, { bigint: true });
    const finalPath = lstatSync(filePath, { bigint: true });
    if (!sameState(opened, finalOpened) || !sameState(finalOpened, finalPath)) {
      fail(`${label} changed while reading`);
    }
    return bytes.subarray(0, length);
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

function readCanonicalRegistry(filePath = registryPath) {
  const bytes = readStableFile(filePath, maximumRegistryBytes, 'registry');
  let document;
  try {
    document = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  } catch (error) {
    fail(`registry is not strict UTF-8 JSON: ${error.message}`);
  }
  if (!bytes.equals(Buffer.from(`${JSON.stringify(document, null, 2)}\n`))) {
    fail('registry bytes are not canonical JSON');
  }
  return { bytes, document };
}

function fixtureInventory(directory, prefix = 'tests/m2-fixtures') {
  const entries = readdirSync(directory, { withFileTypes: true });
  const folded = new Set();
  const files = [];
  for (const entry of entries) {
    if (entry.isSymbolicLink()) fail(`fixture entry ${entry.name} is a symbolic link`);
    const portable = `${prefix}/${entry.name}`;
    const identity = entry.name.toLowerCase();
    if (folded.has(identity)) fail(`fixture directory ${prefix} has an ASCII case collision`);
    folded.add(identity);
    const absolute = path.join(directory, entry.name);
    if (entry.isDirectory()) files.push(...fixtureInventory(absolute, portable));
    else if (entry.isFile()) files.push(portable);
    else fail(`fixture entry ${portable} is not a regular file or directory`);
  }
  return files.sort();
}

function typedInput(value) {
  return { type: typeof value === 'boolean' ? 'bool' : 'i32', value };
}

export function rustTestSelectorExists(source, selector, moduleName) {
  if (!selector.startsWith(`${moduleName}::`)) return false;
  const testName = selector.slice(moduleName.length + 2);
  if (!/^[A-Za-z_][A-Za-z0-9_]*$/u.test(testName)) return false;
  const escapedName = testName.replace(/[.*+?^${}()|[\]\\]/gu, '\\$&');
  const declaration = new RegExp(
    `((?:^[ \t]*#\\[[^\\]\r\n]+\\][ \t]*\r?\n)+)[ \t]*fn[ \t]+${escapedName}[ \t]*\\(`,
    'gmu',
  );
  return [...source.matchAll(declaration)].some((match) =>
    /^[ \t]*#\[test\][ \t]*$/mu.test(match[1]));
}

function evidenceSelectorExists(commandId, selector) {
  const candidates = evidenceSourcesByCommand.get(commandId);
  if (!candidates) return false;
  return candidates.some((candidate) => {
    const relative = candidate.path;
    const source = new TextDecoder('utf-8', { fatal: true }).decode(
      readStableFile(path.join(workspaceRoot, relative), maximumFixtureBytes, relative),
    );
    return relative.endsWith('.mjs')
      ? source.includes(`test('${selector}'`)
      : rustTestSelectorExists(source, selector, candidate.module);
  });
}

function validateTypedValue(value, label) {
  exactKeys(value, ['type', 'value'], label);
  if (value.type === 'bool') {
    if (typeof value.value !== 'boolean') fail(`${label} bool value is invalid`);
  } else if (value.type === 'i32') {
    if (!Number.isInteger(value.value) || value.value < -2147483648 || value.value > 2147483647) {
      fail(`${label} i32 value is invalid`);
    }
  } else {
    fail(`${label} type is unsupported`);
  }
}

export function validateM2Conformance(document, fixturePaths = fixtureInventory(
  path.join(workspaceRoot, 'tests', 'm2-fixtures'),
)) {
  exactKeys(
    document,
    [
      'schemaVersion',
      'profile',
      'historicalPlanningSha256',
      'targetOrder',
      'graph',
      'fixtureFiles',
      'validCases',
      'invalidCases',
      'boundaryEvidence',
      'determinism',
    ],
    'registry',
  );
  if (
    document.schemaVersion !== 1 ||
    document.profile !== 'zryna-control-flow-fixed-oracle-v1' ||
    document.historicalPlanningSha256 !== historicalRegistrySha256
  ) fail('registry identity or historical planning authentication drifted');
  if (!isDeepStrictEqual(document.targetOrder, ['javascript', 'webassembly', 'native'])) {
    fail('target order drifted');
  }

  const planning = loadAndValidateM2Contract();
  const plannedById = new Map(planning.plannedCases.map((item) => [item.id, item]));
  const executableIds = document.validCases.map((item) => item.id);
  const expectedIds = planning.plannedCases.map((item) => item.id);
  expectedIds.splice(14, 0, 'argument-order-is-positional');
  if (!isDeepStrictEqual(executableIds, expectedIds)) fail('valid case inventory or order drifted');
  for (const [index, item] of document.validCases.entries()) {
    exactKeys(item, ['id', 'feature', 'export', 'arguments', 'expected'], `valid case #${index}`);
    if (!/^[A-Za-z][A-Za-z0-9]*$/u.test(item.export)) fail(`valid case ${item.id} export is invalid`);
    if (!Array.isArray(item.arguments)) fail(`valid case ${item.id} arguments are invalid`);
    item.arguments.forEach((value, argumentIndex) =>
      validateTypedValue(value, `valid case ${item.id} argument #${argumentIndex}`));
    validateTypedValue(item.expected, `valid case ${item.id} expected`);
    const planned = plannedById.get(item.id);
    if (planned) {
      if (
        item.feature !== planned.feature ||
        !isDeepStrictEqual(item.arguments, planned.inputs.map(typedInput)) ||
        !isDeepStrictEqual(item.expected, {
          type: planned.expected.type,
          value: planned.expected.value,
        })
      ) fail(`valid case ${item.id} differs from historical planning`);
    }
  }
  const argumentOrder = document.validCases.at(14);
  if (
    argumentOrder?.id !== 'argument-order-is-positional' ||
    argumentOrder.export !== 'argumentOrder' ||
    !isDeepStrictEqual(argumentOrder.expected, { type: 'i32', value: -2 })
  ) fail('noncommutative argument-order oracle drifted');

  const plannedInvalidIds = planning.plannedInvalidCases
    .filter(({ id }) => !['irreducible-cfg', 'resource-limit-plus-one'].includes(id))
    .map(({ id }) => id);
  if (!isDeepStrictEqual(document.invalidCases.map(({ id }) => id), plannedInvalidIds)) {
    fail('public invalid case inventory or order drifted');
  }
  for (const [index, item] of document.invalidCases.entries()) {
    exactKeys(
      item,
      ['id', 'entrypoint', 'exitCode', 'diagnosticCodes', 'diagnostics'],
      `invalid case #${index}`,
    );
    if (item.exitCode !== 3 || !Array.isArray(item.diagnosticCodes) || item.diagnosticCodes.length < 1) {
      fail(`invalid case ${item.id} outcome is invalid`);
    }
    if (item.diagnosticCodes.some((code) => !/^ZRYNA-[A-Z][0-9]{4}$/u.test(code))) {
      fail(`invalid case ${item.id} diagnostic code is invalid`);
    }
    if (!isDeepStrictEqual(item.diagnosticCodes, expectedInvalidCodes.get(item.id))) {
      fail(`invalid case ${item.id} diagnostic oracle drifted`);
    }
    if (!Array.isArray(item.diagnostics) || item.diagnostics.length !== item.diagnosticCodes.length) {
      fail(`invalid case ${item.id} fixed diagnostic snapshot is invalid`);
    }
    if (!isDeepStrictEqual(item.diagnostics.map(({ code }) => code), item.diagnosticCodes)) {
      fail(`invalid case ${item.id} fixed diagnostic snapshot code order drifted`);
    }
    const diagnosticDigest = createHash('sha256')
      .update(JSON.stringify(item.diagnostics))
      .digest('hex');
    if (diagnosticDigest !== expectedInvalidDiagnosticSha256.get(item.id)) {
      fail(`invalid case ${item.id} fixed diagnostic snapshot drifted`);
    }
  }

  const registeredPaths = document.fixtureFiles.map(({ path: fixturePath }) => fixturePath);
  if (!isDeepStrictEqual(registeredPaths, fixturePaths)) fail('fixture inventory drifted');
  for (const [index, fixture] of document.fixtureFiles.entries()) {
    exactKeys(fixture, ['path', 'sha256'], `fixture #${index}`);
    if (!hexadecimalSha256.test(fixture.sha256)) fail(`fixture ${fixture.path} hash is invalid`);
    const bytes = readStableFile(path.join(workspaceRoot, fixture.path), maximumFixtureBytes, fixture.path);
    const digest = createHash('sha256').update(bytes).digest('hex');
    if (digest !== fixture.sha256) fail(`fixture ${fixture.path} hash drifted`);
  }

  exactKeys(document.graph, ['entrypoint', 'sha256', 'sources', 'edges', 'buildArtifacts'], 'graph');
  if (
    document.graph.entrypoint !== 'tests/m2-fixtures/valid/main.zry' ||
    document.graph.sha256 !== expectedGraphSha256
  ) fail('graph identity drifted');
  const fixtureHashByPath = new Map(document.fixtureFiles.map((item) => [item.path, item.sha256]));
  for (const source of document.graph.sources) {
    if (fixtureHashByPath.get(source.path) !== source.sha256) fail(`graph source ${source.path} drifted`);
  }
  if (
    !isDeepStrictEqual(document.graph.sources, expectedGraphSources) ||
    !isDeepStrictEqual(document.graph.edges, expectedGraphEdges)
  ) fail('canonical graph source or edge oracle drifted');
  for (const artifact of document.graph.buildArtifacts) {
    exactKeys(artifact, ['target', 'kind', 'bytes', 'sha256'], `artifact ${artifact.target}`);
    if (!hexadecimalSha256.test(artifact.sha256) || !Number.isSafeInteger(artifact.bytes)) {
      fail(`artifact ${artifact.target} oracle is invalid`);
    }
  }
  if (!isDeepStrictEqual(
    document.graph.buildArtifacts.map(({ target }) => target),
    document.targetOrder,
  )) fail('artifact target order drifted');
  if (!isDeepStrictEqual(document.graph.buildArtifacts, expectedBuildArtifacts)) {
    fail('fixed build artifact oracle drifted');
  }

  exactKeys(
    document.boundaryEvidence,
    ['irreducibleCfg', 'atomicFailures', 'sourceRaces', 'resourceLimits'],
    'boundary evidence',
  );
  exactKeys(document.boundaryEvidence.irreducibleCfg,
    ['suite', 'test', 'diagnosticCode'], 'irreducible CFG evidence');
  if (
    document.boundaryEvidence.irreducibleCfg.suite !== 'portable-ir' ||
    document.boundaryEvidence.irreducibleCfg.test !==
      'control_flow_v1::tests::rejects_non_dominating_use_and_irreducible_cfg' ||
    document.boundaryEvidence.irreducibleCfg.diagnosticCode !== 'ZRYNA-I2020'
  ) fail('irreducible CFG evidence drifted');

  exactKeys(document.boundaryEvidence.atomicFailures, ['suite', 'test'], 'atomic-failure evidence');
  if (
    document.boundaryEvidence.atomicFailures.suite !== 'driver-pipeline' ||
    document.boundaryEvidence.atomicFailures.test !==
      'pipeline::tests::every_m2_pipeline_phase_failure_leaves_no_final_bundle' ||
    !evidenceSelectorExists(
      document.boundaryEvidence.atomicFailures.suite,
      document.boundaryEvidence.atomicFailures.test,
    )
  ) fail('atomic-failure evidence drifted');

  exactKeys(document.boundaryEvidence.sourceRaces, ['suite', 'test'], 'source-race evidence');
  if (
    document.boundaryEvidence.sourceRaces.suite !== 'module-closure' ||
    document.boundaryEvidence.sourceRaces.test !==
      'module_closure_tests::rejects_case_collisions_links_and_source_mutation_during_final_analysis' ||
    !evidenceSelectorExists(
      document.boundaryEvidence.sourceRaces.suite,
      document.boundaryEvidence.sourceRaces.test,
    )
  ) fail('source-race evidence drifted');

  exactKeys(document.boundaryEvidence.resourceLimits, ['rows'], 'resource-limit evidence');
  const rows = document.boundaryEvidence.resourceLimits.rows;
  if (!Array.isArray(rows)) fail('resource-limit evidence rows must be an array');
  const evidenceDigest = createHash('sha256').update(JSON.stringify(rows)).digest('hex');
  if (evidenceDigest !== expectedResourceEvidenceSha256) {
    fail('resource-limit evidence selector contract drifted');
  }
  const expectedLimits = Object.entries(planning.limits);
  if (!isDeepStrictEqual(rows.map(({ key, limit }) => [key, limit]), expectedLimits)) {
    fail('resource-limit evidence must cover every historical limit exactly once in order');
  }
  const quickCommandIds = new Set(M2_QUICK_COMMANDS.map(({ id }) => id));
  for (const [index, row] of rows.entries()) {
    const expectedKeys = row.mode === 'lowered-test-limit'
      ? ['key', 'limit', 'commandId', 'test', 'mode', 'productionBindingTest']
      : ['key', 'limit', 'commandId', 'test', 'mode'];
    exactKeys(row, expectedKeys, `resource-limit evidence row #${index}`);
    if (!quickCommandIds.has(row.commandId)) {
      fail(`resource-limit evidence ${row.key} references an unexecuted command`);
    }
    if (typeof row.test !== 'string' || row.test.length < 8 || row.test.length > 200) {
      fail(`resource-limit evidence ${row.key} has an invalid test selector`);
    }
    if (!evidenceSelectorExists(row.commandId, row.test)) {
      fail(`resource-limit evidence ${row.key} test selector does not exist`);
    }
    if (!['production-boundary', 'lowered-test-limit'].includes(row.mode)) {
      fail(`resource-limit evidence ${row.key} has an invalid mode`);
    }
    if (
      row.mode === 'lowered-test-limit' &&
      (
        typeof row.productionBindingTest !== 'string' ||
        !evidenceSelectorExists(row.commandId, row.productionBindingTest)
      )
    ) fail(`resource-limit evidence ${row.key} lacks its production binding`);
  }
  exactKeys(
    document.determinism,
    [
      'sameStemBuildRepeats',
      'compareManifestBytes',
      'compareArtifactBytes',
      'nativeLinkedExecutableScope',
    ],
    'determinism',
  );
  if (
    document.determinism.sameStemBuildRepeats !== 2 ||
    document.determinism.compareManifestBytes !== true ||
    document.determinism.compareArtifactBytes !== true ||
    document.determinism.nativeLinkedExecutableScope !== 'same-pinned-toolchain-and-host'
  ) fail('determinism contract drifted');
  return document;
}

export function loadAndValidateM2Conformance(
  filePath = registryPath,
  { verifyDigest = true } = {},
) {
  const { bytes, document } = readCanonicalRegistry(filePath);
  if (verifyDigest) {
    const digest = createHash('sha256').update(bytes).digest('hex');
    if (digest !== expectedRegistrySha256) fail('registry digest mismatch');
  }
  return validateM2Conformance(document);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : '';
if (import.meta.url === invokedPath) {
  loadAndValidateM2Conformance();
  process.stdout.write(`M2 executable conformance verified: ${expectedRegistrySha256}\n`);
}
