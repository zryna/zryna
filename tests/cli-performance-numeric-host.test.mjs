import assert from 'node:assert/strict';
import { resolve, join } from 'node:path';
import test from 'node:test';
import { baselineRound, runHostedRound, runNumericHostedRound } from '../scripts/stability-gates/cli-performance-hosted-adapter.mjs';
import { affinityLocations, captureNumericCompanion, numericOperatorInput, numericRound,
  validateAffinityBuild } from '../scripts/stability-gates/cli-performance-numeric-host.mjs';

function operator() {
  return { calibrationDirectory: resolve('calibration'), environment: { INCLUDE: 'synthetic' },
    expectedHead: 'a'.repeat(40), expectedTree: 'b'.repeat(40), root: resolve('checkout'),
    round: 'held-out-candidate', tools: { node: 'synthetic-node' } };
}
function fixture() {
  const pin = name => ({ path: `/synthetic/${name}`, bytes: 1, sha256: 'c'.repeat(64) });
  const expected = { checkout: { head: 'a'.repeat(40), tree: 'b'.repeat(40) },
    sourceBefore: ['probe.c', 'build-msvc.cmd', 'decode.mjs', 'decode.test.mjs', 'hosted.ps1', 'live-controls.mjs'].map(pin),
    workflow: pin('windows-job-metrics.yml'), toolchain: { compiler: pin('cl.exe'), linker: pin('link.exe'), node: pin('node.exe') },
    dependencies: Array.from({ length: 18 }, (_, i) => pin(`dependency-${i}`)),
    calibratedToolchain: pin('toolchain-before-build.json'), supplementalDirectHeader: pin('dependency-17'),
    helper: pin('windows-parent-affinity-v1.exe') };
  const sdk = { includeSearch: ['synthetic-include'], libSearch: ['synthetic-lib'], sdkDirectory: 'synthetic-sdk',
    sdkVersion: 'synthetic-version', vcToolsDirectory: 'synthetic-vc', vcToolsVersion: 'synthetic-vc-version' };
  const context = { expected, declaredDependencies: structuredClone(expected.dependencies.slice(0, -1)),
    calibratedTools: structuredClone(expected.toolchain), sdkExpected: sdk, sdkDeclared: structuredClone(sdk),
    runtimeNode: pin('node.exe'), runtimeNodeRealpath: '/synthetic/node.exe', selectedNodeRealpath: '/synthetic/node.exe',
    buildRecord: pin('affinity-build.json'), buildRawSha256: 'c'.repeat(64) };
  const build = { format: 'zryna.windows-parent-affinity-build.v1', ...structuredClone(expected),
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false, hostAdmitted: false,
    closureStatus: 'blocked', thresholds: null };
  return { build, context };
}
function rejectsMutation(mutate) {
  const { build, context } = fixture();
  mutate(build, context);
  assert.throws(() => validateAffinityBuild(build, context));
  const pristine = fixture();
  assert.equal(validateAffinityBuild(pristine.build, pristine.context), undefined);
}

test('numeric roles add held-out directly while the legacy baseline boundary remains strict', async () => {
  for (const role of ['baseline-round-1', 'baseline-round-2', 'held-out-candidate']) assert.equal(numericRound(role), role);
  assert.throws(() => numericRound('baseline-round-3'));
  assert.throws(() => numericRound('heldout'));
  assert.throws(() => baselineRound('held-out-candidate'), /only two unqualified baseline/u);
  assert.equal(baselineRound('baseline-round-2'), 'baseline-round-2');
  assert.equal(runHostedRound.length, 2); assert.equal(runNumericHostedRound.length, 2);
  await assert.rejects(runNumericHostedRound(operator(), 'unused', () => {}), /fixed numeric hosted API/u);
});
test('numeric operator input has exactly the existing seven keys and snapshots mutable declarations', () => {
  const input = operator(), copy = numericOperatorInput(input);
  input.environment.INCLUDE = 'changed'; input.tools.node = 'changed';
  assert.equal(copy.environment.INCLUDE, 'synthetic'); assert.equal(copy.tools.node, 'synthetic-node');
  assert.equal(copy.round, 'held-out-candidate');
  for (const key of ['callback', 'executable', 'args', 'timeoutMs']) {
    const changed = operator(); changed[key] = 'override'; assert.throws(() => numericOperatorInput(changed));
  }
  const changed = operator(); changed.expectedTree = 'not-a-tree'; assert.throws(() => numericOperatorInput(changed));
});
test('affinity location is a fixed calibration sibling and fixed helper, never an input executable', () => {
  const root = resolve('checkout'), calibration = resolve('outside', 'windows-job-metrics-calibration');
  const locations = affinityLocations(root, calibration);
  assert.equal(locations.evidence, resolve('outside', 'windows-parent-affinity-observation'));
  assert.equal(locations.helper, join(locations.evidence, 'build', 'windows-parent-affinity-v1.exe'));
  assert.equal(locations.workflow, join(root, '.github', 'workflows', 'windows-job-metrics.yml'));
  assert.throws(() => affinityLocations(root, 'relative-calibration'));
});
test('matching observed build metadata returns no authority capability', () => {
  const { build, context } = fixture();
  assert.equal(validateAffinityBuild(build, context), undefined);
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted']) assert.equal(build[key], false);
  assert.equal(build.thresholds, null); assert.equal(build.closureStatus, 'blocked');
});
test('checkout, workflow and every source material are bound rather than trusted from build metadata', () => {
  rejectsMutation(build => { build.checkout.tree = 'd'.repeat(40); });
  rejectsMutation(build => { build.workflow.sha256 = 'd'.repeat(64); });
  for (let i = 0; i < 6; i++) rejectsMutation(build => { build.sourceBefore[i].bytes++; });
});
test('fixed helper, calibrated tools, SDK dependencies and search order cannot be substituted', () => {
  rejectsMutation(build => { build.helper.path = '/synthetic/other.exe'; });
  rejectsMutation(build => { build.helper.sha256 = 'd'.repeat(64); });
  for (const key of ['compiler', 'linker', 'node']) rejectsMutation(build => { build.toolchain[key].bytes++; });
  rejectsMutation(build => { build.calibratedToolchain.sha256 = 'd'.repeat(64); });
  rejectsMutation(build => { build.dependencies[17].sha256 = 'd'.repeat(64); });
  rejectsMutation(build => { build.supplementalDirectHeader.path = '/synthetic/other.h'; });
  rejectsMutation((build, context) => { context.declaredDependencies[0].sha256 = 'd'.repeat(64); });
  rejectsMutation((build, context) => { context.calibratedTools.node.bytes++; });
  rejectsMutation((build, context) => { context.sdkDeclared.includeSearch.unshift('shadow-header-directory'); });
  rejectsMutation((build, context) => { context.sdkDeclared.sdkVersion = 'other-version'; });
});
test('the executing collector and bounded original metadata bytes have separate bindings', () => {
  rejectsMutation((build, context) => { context.runtimeNodeRealpath = '/synthetic/other-node.exe'; });
  rejectsMutation((build, context) => { context.runtimeNode.bytes++; });
  rejectsMutation((build, context) => { context.runtimeNode.sha256 = 'd'.repeat(64); });
  rejectsMutation((build, context) => { context.buildRawSha256 = 'd'.repeat(64); });
});
test('build declarations cannot promote authority or admit extra override fields', () => {
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted']) {
    rejectsMutation(build => { build[key] = true; });
  }
  rejectsMutation(build => { build.thresholds = {}; });
  rejectsMutation(build => { build.closureStatus = 'complete'; });
  rejectsMutation(build => { build.callback = 'override'; });
});
test('copied or missing companion keys fail before filesystem or native-query access', async () => {
  const input = operator();
  for (const copied of [null, {}, Object.freeze({})]) {
    await assert.rejects(captureNumericCompanion(input, input.root, resolve('output'), 'after', copied), /same live collector/u);
  }
});
