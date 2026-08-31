import { createHash } from 'node:crypto';
import {
  closeSync,
  constants,
  fstatSync,
  lstatSync,
  openSync,
  readSync,
} from 'node:fs';
import path from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { isDeepStrictEqual } from 'node:util';

export const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const registryPath = path.join(workspaceRoot, 'tests', 'm2-contract-v1.json');
export const expectedRegistrySha256 = 'ce227c0834ec9c2c01bddc2ff393bc3c9deebbbd75476bea95cb69cbb07c9f6b';

const expectedIssues = [
  [45, [], 'normative-profile'],
  [46, [45], 'syntax-protocol-v3'],
  [47, [45, 46], 'module-closure'],
  [48, [45], 'verified-universal-ir'],
  [49, [46, 47, 48], 'strict-semantic-lowering'],
  [50, [49], 'if-while-lowering'],
  [51, [50], 'javascript'],
  [52, [50], 'webassembly'],
  [53, [50], 'verified-native-mir'],
  [54, [53], 'native-object-link-run'],
  [55, [47, 51, 52, 54], 'atomic-multi-file-cli'],
  [56, [55], 'fixed-oracle-conformance'],
  [57, [56], 'authenticated-documentation'],
].map(([number, dependsOn, gate]) => ({ number, dependsOn, gate }));

const expectedOperators = [
  ['i32-add', '+', ['i32', 'i32'], 'i32', 'wrap-modulo-2^32'],
  ['i32-sub', '-', ['i32', 'i32'], 'i32', 'wrap-modulo-2^32'],
  ['i32-mul', '*', ['i32', 'i32'], 'i32', 'low-32-product-bits'],
  ['i32-neg', 'unary -', ['i32'], 'i32', 'zero-minus-value-modulo-2^32'],
  ['equal', '===', ['same-scalar', 'same-scalar'], 'bool', 'exact-value-equality'],
  ['not-equal', '!==', ['same-scalar', 'same-scalar'], 'bool', 'exact-value-inequality'],
  ['i32-less-than-s', '<', ['i32', 'i32'], 'bool', 'signed-order'],
  ['i32-less-equal-s', '<=', ['i32', 'i32'], 'bool', 'signed-order'],
  ['i32-greater-than-s', '>', ['i32', 'i32'], 'bool', 'signed-order'],
  ['i32-greater-equal-s', '>=', ['i32', 'i32'], 'bool', 'signed-order'],
].map(([id, syntax, operands, result, behavior]) => ({
  id,
  syntax,
  operands,
  result,
  behavior,
  traps: [],
}));

const expectedLimits = {
  sourceFiles: 4096,
  aggregateSourceBytes: 8388608,
  discoveryRounds: 4096,
  providerAnalysisCalls: 4097,
  cumulativeProviderInputBytes: 16777216,
  protocolRequestBytes: 75497472,
  protocolResponseBytes: 67108864,
  importEdges: 65536,
  importDeclarationsPerModule: 4096,
  importDeclarationsPerProgram: 65536,
  importedNamesPerDeclaration: 256,
  importedNamesPerProgram: 65536,
  functionsPerModule: 4096,
  functionsPerProgram: 16384,
  parametersPerFunction: 256,
  parametersPerProgram: 262144,
  lexicalBlocksPerFunction: 4096,
  lexicalBlocksPerProgram: 65536,
  statementsPerFunction: 4096,
  statementsPerProgram: 65536,
  expressionsPerFunction: 16384,
  expressionsPerProgram: 262144,
  localsPerFunction: 4096,
  localsPerProgram: 65536,
  liveMutableBindingsPerMerge: 256,
  irBlocksPerFunction: 4096,
  irBlocksPerProgram: 65536,
  blockParametersPerBlock: 256,
  irValuesPerFunction: 16384,
  irValuesPerProgram: 262144,
  cfgEdgesPerFunction: 8192,
  cfgEdgesPerProgram: 131072,
  callEdges: 65536,
  staticCallDepth: 128,
  nestingDepth: 128,
  moduleSpecifierBytes: 1024,
  diagnostics: 256,
};

const returned = (type, value) => ({ kind: 'returned', type, value });

const expectedPlannedCases = [
  ['add-wraps-maximum', 'arithmetic', [2147483647, 1], returned('i32', -2147483648)],
  ['subtract-wraps-minimum', 'arithmetic', [-2147483648, 1], returned('i32', 2147483647)],
  ['multiply-keeps-low-bits', 'arithmetic', [2147483647, 2], returned('i32', -2)],
  ['negate-minimum-wraps', 'arithmetic', [-2147483648], returned('i32', -2147483648)],
  ['signed-less-than', 'comparison', [-1, 0], returned('bool', true)],
  ['signed-less-equal', 'comparison', [-1, -1], returned('bool', true)],
  ['signed-greater-than', 'comparison', [1, 0], returned('bool', true)],
  ['signed-greater-equal', 'comparison', [1, 1], returned('bool', true)],
  ['i32-exact-equality', 'comparison', [42, 42], returned('bool', true)],
  ['i32-exact-inequality', 'comparison', [42, 7], returned('bool', true)],
  ['boolean-exact-equality', 'comparison', [true, false], returned('bool', false)],
  ['boolean-exact-inequality', 'comparison', [true, false], returned('bool', true)],
  ['mutable-local-assignment', 'locals', [1, 2], returned('i32', 3)],
  ['direct-call-chain', 'calls', [20, 22], returned('i32', 42)],
  ['if-selects-true-edge', 'if', [true, 11, 22], returned('i32', 11)],
  ['if-selects-false-edge', 'if', [false, 11, 22], returned('i32', 22)],
  ['while-zero-iterations', 'while', [0], returned('i32', 0)],
  ['while-many-iterations', 'while', [5], returned('i32', 5)],
  ['named-cross-module-call', 'modules', [40, 2], returned('i32', 42)],
].map(([id, feature, inputs, expected]) => ({ id, feature, inputs, expected }));

const expectedPlannedInvalidCases = [
  ['numeric-condition', 'semantics', 'ZRYNA-M2xxx', 50],
  ['assign-to-const', 'semantics', 'ZRYNA-M2xxx', 49],
  ['use-before-declaration', 'semantics', 'ZRYNA-M2xxx', 49],
  ['recursive-call-cycle', 'semantics', 'ZRYNA-M2xxx', 49],
  ['bare-import', 'module-discovery', 'ZRYNA-D3xxx', 47],
  ['import-escapes-root', 'module-discovery', 'ZRYNA-D3xxx', 47],
  ['missing-import', 'module-discovery', 'ZRYNA-D3xxx', 47],
  ['case-colliding-module', 'module-discovery', 'ZRYNA-D3xxx', 47],
  ['import-cycle', 'module-discovery', 'ZRYNA-D3xxx', 47],
  ['irreducible-cfg', 'universal-ir', 'ZRYNA-I2xxx', 48],
  ['resource-limit-plus-one', 'all-new-m2-boundaries', 'x2xx1', 56],
].map(([id, phase, expectedFamily, ownerIssue]) => ({ id, phase, expectedFamily, ownerIssue }));

function fail(message) {
  throw new Error(`invalid M2 contract: ${message}`);
}

function exactKeys(value, keys, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) fail(`${label} must be an object`);
  const actual = Object.keys(value).sort();
  const expected = [...keys].sort();
  if (!isDeepStrictEqual(actual, expected)) fail(`${label} must contain exactly ${expected.join(', ')}`);
}

export function validateM2Contract(document) {
  exactKeys(
    document,
    [
      'schemaVersion',
      'profile',
      'status',
      'specification',
      'governanceIssue',
      'issues',
      'operators',
      'trapCodes',
      'limits',
      'plannedCases',
      'plannedInvalidCases',
    ],
    'registry',
  );
  if (
    document.schemaVersion !== 1 ||
    document.profile !== 'zryna-control-flow-v1' ||
    document.status !== 'specified-not-implemented' ||
    document.specification !== 'spec/language/CONTROL_FLOW_MODULES_V1.md' ||
    document.governanceIssue !== 45
  ) {
    fail('identity or implementation status drifted');
  }
  if (!isDeepStrictEqual(document.issues, expectedIssues)) fail('issue dependency ledger drifted');
  if (!isDeepStrictEqual(document.operators, expectedOperators)) fail('operator contract drifted');
  if (!isDeepStrictEqual(document.trapCodes, [])) fail('trap contract drifted');
  if (!isDeepStrictEqual(document.limits, expectedLimits)) fail('resource budgets drifted');
  if (!isDeepStrictEqual(document.plannedCases, expectedPlannedCases)) {
    fail('planned conformance case inventory drifted');
  }
  if (!isDeepStrictEqual(document.plannedInvalidCases, expectedPlannedInvalidCases)) {
    fail('planned invalid case inventory drifted');
  }
  return document;
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

function readCanonicalRegistry(filePath = registryPath) {
  const maximumBytes = 1024 * 1024;
  const pathState = lstatSync(filePath, { bigint: true });
  if (pathState.isSymbolicLink() || !pathState.isFile()) fail('registry is not a regular file');

  const noFollow = constants.O_NOFOLLOW ?? 0;
  let descriptor;
  try {
    descriptor = openSync(filePath, constants.O_RDONLY | noFollow);
    const openedState = fstatSync(descriptor, { bigint: true });
    if (!openedState.isFile() || !sameIdentity(pathState, openedState)) {
      fail('registry identity changed while opening');
    }
    if (openedState.size > BigInt(maximumBytes)) fail('registry exceeds one MiB');

    const bounded = Buffer.alloc(maximumBytes + 1);
    let length = 0;
    while (length < bounded.length) {
      const count = readSync(descriptor, bounded, length, bounded.length - length, null);
      if (count === 0) break;
      length += count;
    }
    if (length > maximumBytes) fail('registry exceeds one MiB');

    const finalState = fstatSync(descriptor, { bigint: true });
    if (!sameState(openedState, finalState) || finalState.size !== BigInt(length)) {
      fail('registry changed while reading');
    }
    const finalPathState = lstatSync(filePath, { bigint: true });
    if (finalPathState.isSymbolicLink() || !sameState(finalState, finalPathState)) {
      fail('registry path changed while reading');
    }

    const bytes = bounded.subarray(0, length);
    let document;
    try {
      document = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    } catch (error) {
      fail(`registry is not strict UTF-8 JSON: ${error.message}`);
    }
    const canonical = Buffer.from(`${JSON.stringify(document, null, 2)}\n`);
    if (!bytes.equals(canonical)) fail('registry bytes are not canonical JSON');
    return { bytes, document };
  } finally {
    if (descriptor !== undefined) closeSync(descriptor);
  }
}

export function loadAndValidateM2Contract(filePath = registryPath, { verifyDigest = true } = {}) {
  const { bytes, document } = readCanonicalRegistry(filePath);
  if (verifyDigest) {
    const digest = createHash('sha256').update(bytes).digest('hex');
    if (digest !== expectedRegistrySha256) fail('registry digest mismatch');
  }
  return validateM2Contract(document);
}

const invokedPath = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : '';
if (import.meta.url === invokedPath) {
  loadAndValidateM2Contract();
  process.stdout.write(`M2 contract verified: ${expectedRegistrySha256}\n`);
}
