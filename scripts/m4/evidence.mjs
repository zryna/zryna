import assert from 'node:assert/strict';
import path from 'node:path';
import { expectedRegistrySha256, calculateM4Coverage, loadAndValidateM4Conformance } from '../check-m4-conformance.mjs';
import { digest, parseCanonical, readStableFile } from './registry-files.mjs';
import { array, exact, MAX_RECEIPT, processResult, reject, text } from './evidence-input.mjs';
import { requireNodeTests, rustAccounting } from './evidence-counts.mjs';
import { foundationAccounting } from './evidence-foundations.mjs';
import { browserAccounting } from './evidence-browser.mjs';
import { installedAccounting } from './evidence-installed.mjs';
import { sourceAccounting } from './evidence-source.mjs';
import { quantitativeAccounting } from './evidence-quantitative.mjs';
import { sourceBinding, hostBinding, workflowBinding, observations } from './evidence-context.mjs';

export const supplementalGates = Object.freeze([
  { id: 'm4-hostile-self-tests', argv: ['node', '--test', 'tests/m4-conformance.test.mjs',
    'tests/m4-evidence.test.mjs', 'tests/m4-hosted.test.mjs', 'tests/m4-workflow.test.mjs',
    'tests/m4-public-docs.test.mjs', 'tests/m4-limit-contracts.test.mjs',
    'tests/m4-quantitative.test.mjs', 'tests/m4-browser-byte-evidence.test.mjs'], accounting: 'node-tests' },
  { id: 'm4-static-projection', argv: ['node', 'scripts/m4/support.mjs', '--check'], accounting: 'static-check' },
  { id: 'm4-documentation-export', argv: ['pnpm', 'docs:check'], accounting: 'node-tests' },
]);
const validatedAssessments = new WeakMap();
export function normalizeM4Host(host) {
  if (host === 'win32' || host === 'windows') return 'windows';
  if (host === 'linux') return 'linux';
  reject('unknown M4 host identity');
}
export function resolveM4Argv(gate, evidenceDirectory, host) {
  const paths = host === 'windows' ? path.win32 : path.posix;
  const placeholders = {
    '$ABSOLUTE_CREATE_ONLY_BROWSER_EVIDENCE_DIRECTORY': ['pinned-browser-three-proof', 'browser'],
    '$EXTERNAL_CREATE_ONLY_EVIDENCE_DIRECTORY': ['installed-qualification', 'installed'],
  };
  return gate.argv.map(arg => {
    const replacement = placeholders[arg];
    if (!replacement) {
      if (arg.startsWith('$')) reject('unknown evidence-directory placeholder');
      return arg;
    }
    assert.equal(gate.id, replacement[0], 'placeholder belongs to exact gate');
    assert(paths.isAbsolute(evidenceDirectory), 'absolute external evidence directory');
    return paths.join(evidenceDirectory, replacement[1]);
  });
}
export function selectM4Gates(registry, lane, host) {
  if (!['core', 'browser', 'installed'].includes(lane) || !['linux', 'windows'].includes(host)) reject('unsupported M4 lane/host');
  return registry.gates.filter(gate => gate.hosts.includes(host) && (lane === 'browser'
    ? gate.id === 'pinned-browser-three-proof' : lane === 'installed' ? gate.id === 'installed-qualification'
      : !['pinned-browser-three-proof', 'installed-qualification'].includes(gate.id)))
    .concat(lane === 'core' ? supplementalGates : []);
}
export function requireM4ResultInventory(results, gates) {
  array(results, 80, 'results');
  assert.deepEqual(results.map(result => result.id), gates.map(gate => gate.id), 'complete fixed ordered gate inventory');
}
export async function validateM4Evidence(receipt, { root, sourceRoot, registry, expected } = {}) {
  if (!expected || !root || !sourceRoot || !registry) reject('external expectations, source root and retained root required');
  exact(expected, ['source', 'registrySha256', 'host', 'lane', 'workflow', 'tools', 'materials', 'evidenceDirectory'], 'external expectations');
  const authority = loadAndValidateM4Conformance(path.join(sourceRoot, 'tests/m4-conformance-v1.json'), { root: sourceRoot });
  assert.deepEqual(registry, authority, 'caller registry equals actual pinned source authority');
  exact(receipt, ['format', 'registrySha256', 'source', 'sourceState', 'host', 'lane', 'tools', 'materials', 'workflow', 'results', 'evidenceDirectory'], 'receipt');
  assert.equal(receipt.format, 'zryna.m4-evidence.v1');
  assert.equal(expected.registrySha256, expectedRegistrySha256);
  assert.equal(receipt.registrySha256, expectedRegistrySha256, 'exact frozen receipt registry binding');
  text(receipt.evidenceDirectory, 'original evidence directory', 2048);
  assert.equal(receipt.evidenceDirectory, expected.evidenceDirectory, 'externally bound actual collection path');
  const paths = receipt.host.os === 'windows' ? path.win32 : path.posix;
  assert(paths.isAbsolute(receipt.evidenceDirectory), 'absolute evidence path required');
  sourceBinding(receipt.source, expected.source);
  sourceAccounting(receipt.sourceState, receipt.source, root);
  hostBinding(receipt.host, receipt.lane);
  workflowBinding(receipt.workflow, receipt.source);
  assert.deepEqual(receipt.host, expected.host, 'externally authenticated actual host');
  assert.equal(receipt.lane, expected.lane);
  assert.deepEqual(receipt.workflow, expected.workflow, 'externally authenticated workflow');
  observations(receipt, root, expected);
  const gates = selectM4Gates(registry, receipt.lane, receipt.host.os);
  requireM4ResultInventory(receipt.results, gates);
  const descriptors = [...receipt.tools.map(tool => tool.versionLog),
    ...Object.values(receipt.sourceState).flatMap(phase => Object.values(phase).flatMap(item => [item.stdout, item.stderr])),
    ...receipt.results.flatMap(result => [result.stdout, result.stderr, ...result.attachments])];
  if (descriptors.length > 256 || descriptors.reduce((sum, descriptor) => sum + descriptor.bytes, 0) > 768 * 1024 * 1024 ||
      new Set(descriptors.map(descriptor => descriptor.path.toLowerCase())).size !== descriptors.length) {
    reject('retained global count/bytes/identity bound');
  }
  const caseIds = [];
  let foundationEvidence = { executedEvidenceIds: [], exclusionIds: [] };
  for (const [index, result] of receipt.results.entries()) {
    const gate = gates[index];
    const expectedArgv = resolveM4Argv(gate, receipt.evidenceDirectory, receipt.host.os);
    assert.deepEqual(result.argv, expectedArgv, 'fixed actual executable argv');
    const executable = receipt.tools.find(tool => tool.name === expectedArgv[0]);
    if (!executable) reject('gate executable has no actual tool identity');
    const launch = expectedArgv[0] === 'pnpm'
      ? [receipt.tools[0].executable.path, executable.executable.path, ...expectedArgv.slice(1)]
      : [executable.executable.path, ...expectedArgv.slice(1)];
    assert.deepEqual(result.launchArgv, launch, 'actual direct process launch argv');
    const logs = processResult(result, root, `logs/${gate.id}`);
    quantitativeAccounting(logs.stdout, { registry, sourceRoot, gateId: gate.id, host: receipt.host.os });
    const cases = registry.cases.filter(item => item.gateId === gate.id);
    if (gate.accounting === 'one-exact-passing-nonignored-rust-test') {
      rustAccounting(logs.stdout, cases[0].test.selector);
      assert.deepEqual(result.attachments, [], 'exact Rust proofs have no substituted attachments');
    } else if (gate.accounting === 'complete-foundation-gate') {
      const accounting = foundationAccounting(gate.id, logs.stdout, registry, { sourceRoot, host: receipt.host.os });
      foundationEvidence.executedEvidenceIds.push(...accounting.executedEvidenceIds);
      foundationEvidence.exclusionIds.push(...accounting.exclusionIds);
      assert.deepEqual(result.attachments, []);
    } else if (gate.accounting === 'exact-three-proof-pinned-browser-receipt') {
      await browserAccounting(result, { root, sourceRoot, source: receipt.source, host: receipt.host });
    } else if (gate.accounting === 'exact-installed-invocation-inventory') {
      await installedAccounting(result, { root, sourceRoot, source: receipt.source, host: receipt.host,
        evidenceDirectory: receipt.evidenceDirectory, tools: receipt.tools, registry });
    } else {
      if (gate.accounting === 'node-tests') requireNodeTests(logs.stdout);
      assert.deepEqual(result.attachments, []);
    }
    caseIds.push(...cases.map(item => item.id));
  }
  const assessment = { source: expected.source, registrySha256: receipt.registrySha256, host: receipt.host,
    lane: receipt.lane, workflow: receipt.workflow, foundationEvidence, gateIds: gates.map(gate => gate.id), caseIds,
    executionAuthenticated: receipt.workflow !== null, eligible: false,
    blockers: [...calculateM4Coverage(registry).blockers, 'other required M4 lanes/hosts and live publication are separate evidence'] };
  const immutable = immutableAssessment(assessment);
  validatedAssessments.set(immutable, digest(JSON.stringify(authority)));
  return immutable;
}
function immutableAssessment(value) {
  const clone = structuredClone(value);
  const freeze = item => {
    if (item && typeof item === 'object') {
      for (const child of Object.values(item)) freeze(child);
      Object.freeze(item);
    }
    return item;
  };
  return freeze(clone);
}
export async function loadM4Evidence(receiptPath, options) {
  const receipt = parseCanonical(readStableFile(receiptPath, MAX_RECEIPT, 'M4 receipt'));
  return validateM4Evidence(receipt, options);
}
export function composeM4Evidence(registry, assessments) {
  array(assessments, 6, 'authenticated lane assessments');
  assert.equal(assessments.length, 6, 'all six lane/host receipts required');
  const identities = assessments.map(item => `${item.host.os}/${item.lane}`);
  assert.deepEqual([...identities].sort(), ['linux/browser', 'linux/core', 'linux/installed',
    'windows/browser', 'windows/core', 'windows/installed']);
  for (const item of assessments) {
    if (!validatedAssessments.has(item)) reject('unvalidated or caller-fabricated lane assessment');
    assert.equal(digest(JSON.stringify(registry)), validatedAssessments.get(item), 'composition retains exact frozen registry/gaps');
    assert.equal(item.executionAuthenticated, true, 'hosted source authentication required');
    assert.deepEqual(item.source, assessments[0].source, 'one externally expected candidate');
    assert.equal(item.registrySha256, expectedRegistrySha256);
  }
  const windowsCore = assessments.find(item => item.host.os === 'windows' && item.lane === 'core');
  const linuxCore = assessments.find(item => item.host.os === 'linux' && item.lane === 'core');
  assert.equal(windowsCore.foundationEvidence.exclusionIds.length, 8, 'seven M3 and one source-cfg M2 Windows foundation exclusions');
  assert(windowsCore.foundationEvidence.exclusionIds.includes('m2/retained-stage-identity'), 'explicit M2 Windows source-cfg exclusion');
  for (const id of windowsCore.foundationEvidence.exclusionIds) {
    assert(linuxCore.foundationEvidence.executedEvidenceIds.includes(id), 'Windows exclusion requires same-source authenticated Linux proof');
  }
  const coverage = calculateM4Coverage(registry);
  const rows = registry.rows.map(row => {
    const receipts = assessments.filter(item => item.host.os === normalizeM4Host(row.host.os));
    const gates = new Set(receipts.flatMap(item => item.gateIds));
    const cases = new Set(receipts.flatMap(item => item.caseIds));
    const blockers = coverage.blockers.filter(reason => reason.startsWith(`${row.id}:`) || reason.startsWith(`${row.id}/`));
    if (row.requiredGates.some(id => !gates.has(id)) || row.cases.some(id => !cases.has(id))) blockers.push('required execution evidence absent');
    return { id: row.id, executionAuthenticated: blockers.every(reason => reason !== 'required execution evidence absent'),
      eligible: blockers.length === 0, blockers };
  });
  return { registrySha256: expectedRegistrySha256, source: assessments[0].source, rows,
    executionComplete: rows.every(row => row.executionAuthenticated), eligible: false,
    blockers: [...coverage.blockers, 'exact-main documentation export, deployment and live publication authentication required'] };
}
