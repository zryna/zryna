import path from 'node:path';
import { createRequire } from 'node:module';
import { isDeepStrictEqual } from 'node:util';
import { fileURLToPath, pathToFileURL } from 'node:url';
import {
  digest, fail, inventory, operationalSourcePath, parseCanonical, portablePath, readStableFile,
} from './m4/registry-files.mjs';
import {
  expectedRegistrySha256, maximumRegistryBytes,
  maximumSourceBytes, rowInventory, sectionSha256,
} from './m4/registry-authority.mjs';

import { expectedMappedRegistrySchemaSha256 } from './m4/limit-contract-authority.mjs';
import { loadReviewedLimitContracts, validateLimitMappings, calculateContractCoverage } from './m4/limit-contracts.mjs';
import { validateNamedGateCases } from './m4/named-gate-cases.mjs';
const validatedCoverage = new WeakMap();
export { expectedRegistrySha256, digest };
export const workspaceRoot = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '..');
export const registryPath = path.join(workspaceRoot, 'tests/m4-conformance-v1.json');
const schemaRelative = 'schemas/zryna-m4-conformance-v1.schema.json';
const schemaValidators = new Map();
function schemaValidator(root) {
  const bytes = readStableFile(path.join(root, schemaRelative), 128 * 1024, 'schema');
  if (!expectedMappedRegistrySchemaSha256 || digest(bytes) !== expectedMappedRegistrySchemaSha256) fail('mapped schema authority not frozen or drifted');
  if (!schemaValidators.has(root)) {
    const require = createRequire(path.join(root, 'package.json'));
    const Ajv2020 = require('ajv/dist/2020.js');
    const ajv = new Ajv2020({ strict: true, allErrors: false });
    schemaValidators.set(root, ajv.compile(parseCanonical(bytes, 'schema')));
  }
  return schemaValidators.get(root);
}
function uniqueById(items, label) {
  const map = new Map();
  for (const item of items) {
    if (map.has(item.id)) fail(`duplicate ${label} id`);
    map.set(item.id, item);
  }
  return map;
}
function exact(left, right, label) {
  if (!isDeepStrictEqual(left, right)) fail(`${label} drifted`);
}
export function validateM4Conformance(document, { root = workspaceRoot } = {}) {
  const validate = schemaValidator(root);
  if (!validate(document)) fail(`closed schema rejected ${validate.errors?.[0]?.instancePath}`);
  exact(document.rows.map(row => row.id), rowInventory, 'twelve-row inventory');
  const sources = new Map();
  const folded = new Set();
  for (const source of document.sources) {
    portablePath(source.path);
    const key = source.path.toLowerCase();
    if (folded.has(key)) fail('source duplicate or case collision');
    operationalSourcePath(source.path);
    folded.add(key);
    sources.set(source.path, source);
  }
  exact(document.sources.map(source => source.path), [...sources.keys()].sort(), 'source order');
  const rows = uniqueById(document.rows, 'row');
  const cases = uniqueById(document.cases, 'case');
  const gates = uniqueById(document.gates, 'gate');
  for (const foundation of document.foundations) {
    exact(foundation.registry, sources.get(foundation.registry.path), 'foundation source descriptor');
    if (!gates.has(foundation.gateId)) fail('foundation gate missing');
  }
  for (const wit of document.scope.witClosure) {
    portablePath(wit.logicalPath);
    exact({ path: wit.path, bytes: wit.bytes, sha256: wit.sha256 }, sources.get(wit.path), 'WIT descriptor');
  }
  for (const gate of gates.values()) {
    if (!sources.has(gate.source)) fail('gate source missing');
    if (gate.argv.includes('--skip') || gate.argv.some(arg => /skip-architecture/.test(arg))) {
      fail('gate contains a skipped proof');
    }
  }
  for (const entry of cases.values()) {
    if (!gates.has(entry.gateId) || !sources.has(entry.test.source)) fail('case gate or test source missing');
    if (entry.rowIds.some(id => !rows.has(id))) fail('case names unknown row');
    if (entry.fixture.some(file => !sources.has(file))) fail('case fixture descriptor missing');
    if (['exact-limit', 'first-extra'].includes(entry.kind) !== (entry.boundary !== null)) {
      fail('case boundary kind disagrees');
    }
  }
  validateNamedGateCases(document);
  const plan = loadReviewedLimitContracts(document, root);
  exact(document.contractPlan, sources.get(document.contractPlan.path), 'contract plan source descriptor');
  for (const row of rows.values()) {
    portablePath(row.contract);
    for (const artifact of row.artifacts) portablePath(artifact.path);
    const associated = document.cases.filter(entry => entry.rowIds.includes(row.id));
    exact(row.cases, associated.map(entry => entry.id), 'row case inventory');
    const required = [...document.foundations.map(item => item.gateId),
      ...new Set(associated.map(entry => entry.gateId))];
    exact(row.requiredGates, required, 'row required gate inventory');
    exact(row.identities.witClosure, document.scope.witClosure.map(item => item.logicalPath), 'row WIT closure');
    if (!row.capabilities.effective.length && row.capabilities.requested.length) fail('grant mismatch');
    const limits = uniqueById(row.limits, 'limit');
    for (const entry of associated.filter(item => item.boundary)) {
      if (!limits.has(entry.boundary.limitId)) fail('case boundary has no row limit');
    }
  }
  validateLimitMappings(document, plan);
  // Independent pins follow structural checks so malformed input has precise diagnostics.
  for (const [key, expected] of Object.entries(sectionSha256)) {
    if (digest(JSON.stringify(document[key])) !== expected) fail(`frozen ${key} authority drifted`);
  }
  validatedCoverage.set(document, calculateContractCoverage(document, plan));
  return document;
}
export function calculateM4Coverage(document) {
  const coverage = validatedCoverage.get(document);
  if (!coverage) fail('coverage requires independently validated contract authority');
  return structuredClone(coverage);
}
export const calculateContractComplete = calculateM4Coverage;
export function loadAndValidateM4Conformance(file = registryPath, { root = workspaceRoot } = {}) {
  const bytes = readStableFile(file, maximumRegistryBytes, 'registry');
  if (digest(bytes) !== expectedRegistrySha256) fail('registry digest mismatch');
  const document = validateM4Conformance(parseCanonical(bytes), { root });
  for (const source of document.sources) {
    const actual = readStableFile(path.join(root, source.path), maximumSourceBytes, source.path);
    if (actual.length !== source.bytes || digest(actual) !== source.sha256) fail('source descriptor drifted');
  }
  for (const directory of document.scope.fixtureDirectories) {
    const registered = document.sources.filter(item => item.path.startsWith(`${directory}/`)).map(item => item.path);
    exact(inventory(root, directory), registered, 'fixture no-extra inventory');
  }
  for (const entry of document.cases) {
    if (entry.test.runner !== 'rust') continue;
    const source = new TextDecoder('utf-8', { fatal: true }).decode(
      readStableFile(path.join(root, entry.test.source), maximumSourceBytes));
    const name = entry.test.selector.split('::').at(-1);
    if (!/^[A-Za-z_][A-Za-z0-9_]*$/.test(name) ||
        !new RegExp(`(?:^|\\n)(?:#\\[[^\\]\\n]+\\]\\n)+fn ${name}\\s*\\(`).test(source)) {
      fail('named test declaration missing');
    }
  }
  return document;
}
const invoked = process.argv[1] ? pathToFileURL(path.resolve(process.argv[1])).href : '';
if (import.meta.url === invoked) {
  const registry = loadAndValidateM4Conformance();
  process.stdout.write(`${JSON.stringify({ registrySha256: expectedRegistrySha256,
    ...calculateM4Coverage(registry) }, null, 2)}\n`);
}
