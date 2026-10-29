// Synthetic declarations and the real Linux before-session reader only; no fake anchor or owner issuance.
import assert from 'node:assert/strict';
import test from 'node:test';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import { canonical } from '../scripts/stability-gates/input.mjs';
import { projectNumericHostFacts } from '../scripts/stability-gates/cli-performance-campaign-host-facts-v1.mjs';
import { compareHeldoutHost, compareHeldoutTools, HELDOUT_FLAGS } from '../scripts/stability-gates/cli-performance-heldout-comparison-v1.mjs';
import { beginNumericHeldoutGate, finishNumericHeldoutGate, closeNumericHeldoutGate }
  from '../scripts/stability-gates/cli-performance-heldout-gate-v1.mjs';
import { captureNumericCompanion, inspectNumericCompanionBefore } from '../scripts/stability-gates/cli-performance-numeric-host.mjs';
import { providerAcquisitionEnvironment } from '../scripts/stability-gates/cli-performance-linux-provision-provider.mjs';
import { fixture as windowsFixture } from './cli-performance-campaign-windows-host-fixture.mjs';
const source = { commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
const flags = { ...HELDOUT_FLAGS, sourceFileBytesVerified: true, tableOriginalBytesVerified: true, tableTemporalOrderVerified: true };
function sample(windows = false) {
  const platform = windows ? 'windows-x86_64' : 'linux-x86_64';
  const host = { format: 'zryna.cli-performance-host-observation.v1', platform, phase: 'before',
    sourceAuthenticated: false, performanceQualified: false,
    kernel: { type: windows ? 'Windows_NT' : 'Linux', release: 'fixture', version: 'synthetic', machine: 'x86_64' },
    image: Object.fromEntries(Object.entries({ RUNNER_ARCH: 'X64', RUNNER_OS: windows ? 'Windows' : 'Linux',
      ImageOS: windows ? 'win22' : 'ubuntu24', ImageVersion: 'synthetic-v1' }).map(([key, value]) => [key, { status: 'observed-environment', value }])),
    cpu: { models: ['synthetic CPU'], logicalCount: 4, availableParallelism: 4 },
    affinity: { status: 'observed', text: 'Pid:\t123\nTgid:\t123\nCpus_allowed:\tf\nCpus_allowed_list:\t0-3\n' } };
  const companion = { platform, kernel: structuredClone(host.kernel), source: { ...source }, collectorNodePid: 123,
    ...(windows ? { native: windowsFixture().companions[0].native } : {}) };
  const projected = projectNumericHostFacts(platform, host, companion.native?.decoded ?? null,
    { status: 'observed', value: 'github-actions-hosted-vm' });
  const tools = { host: platform, release: [{ role: 'node', bytes: 1, sha256: 'c'.repeat(64) }],
    sdk: [], native: [], versions: { synthetic: 'v1' } };
  const anchor = { ...flags, current: { host: platform, source: { ...source, ref: 'refs/heads/fix/fixture' } },
    baselineSource: { ...source }, tools: [structuredClone(tools)],
    profile: { sources: { reference: { ...source }, candidate: { ...source } }, collectorSha256: 'e'.repeat(64),
      hosts: [{ host: platform, ...projected, candidateBuild: { class: 'release', cliSha256: 'd'.repeat(64) } }] } };
  return { anchor, observation: { host, companion }, tools, platform };
}
test('pure Linux and Windows premeasurement comparisons preserve all false qualification flags', () => {
  for (const windows of [false, true]) {
    const f = sample(windows), host = compareHeldoutHost(f.anchor, f.observation, source);
    const tools = compareHeldoutTools(f.anchor, f.platform, source, f.tools, 'd'.repeat(64), 'e'.repeat(64));
    for (const report of [host, tools]) for (const [key, value] of Object.entries(HELDOUT_FLAGS)) assert.equal(report[key], value);
    assert.equal(host.host, f.platform); assert.match(tools.semanticToolsSha256, /^[a-f0-9]{64}$/u);
  }
});
test('observed CPU, image, kernel and affinity drift rejects before timing on both hosts', () => {
  for (const windows of [false, true]) for (const mutate of [
    f => { f.observation.host.cpu.models = ['other CPU']; },
    f => { f.observation.host.image.ImageVersion.value = 'other-image'; },
    f => { f.observation.host.kernel.release = 'other-kernel'; f.observation.companion.kernel.release = 'other-kernel'; },
    f => {
      f.observation.host.cpu.availableParallelism = 2;
      if (windows) Object.assign(f.observation.companion.native.decoded.affinity, { processMask: '3', maskBitCount: 2 });
      else f.observation.host.affinity.text = 'Pid:\t123\nTgid:\t123\nCpus_allowed:\t3\nCpus_allowed_list:\t0-1\n';
    }]) {
    const f = sample(windows); mutate(f); assert.throws(() => compareHeldoutHost(f.anchor, f.observation, source));
  }
  const linux = sample(); linux.observation.host.affinity.text = linux.observation.host.affinity.text.replace('Pid:\t123', 'Pid:\t124');
  assert.throws(() => compareHeldoutHost(linux.anchor, linux.observation, source));
});
test('source, scoped anchor proof, semantic tools, measured CLI and collector drift are independent failures', () => {
  for (const windows of [false, true]) {
    for (const mutate of [f => { f.anchor.current.source.commit = 'f'.repeat(40); },
      f => { f.anchor.baselineSource.tree = 'f'.repeat(40); },
      f => { f.anchor.profile.sources.candidate.commit = 'f'.repeat(40); },
      f => { f.anchor.tableTemporalOrderVerified = false; }]) {
      const f = sample(windows); mutate(f); assert.throws(() => compareHeldoutHost(f.anchor, f.observation, source));
    }
    for (const mutate of [f => { f.tools.release[0].sha256 = 'f'.repeat(64); },
      f => { f.tools.sdk.push({ role: 'unexpected', bytes: 1, sha256: 'f'.repeat(64) }); },
      f => { f.tools.versions.synthetic = 'v2'; }]) {
      const f = sample(windows); mutate(f);
      assert.throws(() => compareHeldoutTools(f.anchor, f.platform, source, f.tools, 'd'.repeat(64), 'e'.repeat(64)));
    }
    const f = sample(windows);
    assert.throws(() => compareHeldoutTools(f.anchor, f.platform, source, f.tools, 'f'.repeat(64), 'e'.repeat(64)));
    assert.throws(() => compareHeldoutTools(f.anchor, f.platform, source, f.tools, 'd'.repeat(64), 'f'.repeat(64)));
  }
});
function operator(root) {
  return { root, expectedHead: source.commit, expectedTree: source.tree, round: 'held-out-candidate',
    calibrationDirectory: resolve('unused-calibration'), tools: {}, environment: {} };
}
test('copied gate and companion keys reject before filesystem, API, native query or measurement', async () => {
  const input = operator(resolve('unused-root')), output = resolve('unused-output');
  for (const key of [{}, Object.freeze({}), { sourceFileBytesVerified: true }]) {
    assert.throws(() => inspectNumericCompanionBefore(key, input, input.root, output), /live before/u);
    await assert.rejects(beginNumericHeldoutGate(key, input, input.root, output), /live before/u);
    await assert.rejects(finishNumericHeldoutGate(key, key, input, input.root, output, {}, {}, {}), /unused held-out gate/u);
    assert.throws(() => closeNumericHeldoutGate(key), /own held-out gate/u);
  }
});
test('real private Linux before keys bind original bytes, input and lifetime without executing a helper', {
  skip: process.platform !== 'linux',
}, async t => {
  const directory = mkdtempSync(join(tmpdir(), 'heldout-before-')); t.after(() => rmSync(directory, { recursive: true }));
  const output = join(directory, 'capture'); mkdirSync(output); const input = operator(directory), f = sample();
  const host = f.observation.host; host.affinity.text = host.affinity.text.replaceAll('123', String(process.pid));
  writeFileSync(join(output, 'host-before.json'), canonical(host));
  const key = await captureNumericCompanion(input, directory, output, 'before');
  const observed = inspectNumericCompanionBefore(key, input, directory, output);
  assert.equal(observed.companion.collectorNodePid, process.pid); assert.equal(observed.nativeBinding, null);
  observed.host.kernel.release = 'mutated-copy';
  assert.equal(inspectNumericCompanionBefore(key, input, directory, output).host.kernel.release, 'fixture');
  assert.throws(() => inspectNumericCompanionBefore(key, input, directory, directory), /live before/u);
  assert.throws(() => inspectNumericCompanionBefore(key, { ...input, round: 'baseline-round-1' }, directory, output), /same original/u);
  const changed = { ...host, kernel: { ...host.kernel, release: 'changed-original' } };
  writeFileSync(join(output, 'host-before.json'), canonical(changed));
  assert.throws(() => inspectNumericCompanionBefore(key, input, directory, output), /host bytes unchanged/u);
  writeFileSync(join(output, 'host-before.json'), canonical(host));
  const companionPath = join(output, 'numeric-host-before.json');
  writeFileSync(companionPath, '{}\n');
  assert.throws(() => inspectNumericCompanionBefore(key, input, directory, output), /companion bytes unchanged/u);
  // Separate real before/after session proves completion consumes inspector lifetime without reopening it.
  const second = join(directory, 'second'); mkdirSync(second); writeFileSync(join(second, 'host-before.json'), canonical(host));
  const next = await captureNumericCompanion(input, directory, second, 'before');
  writeFileSync(join(second, 'host-after.json'), canonical({ ...host, phase: 'after' }));
  await captureNumericCompanion(input, directory, second, 'after', next);
  assert.throws(() => inspectNumericCompanionBefore(next, input, directory, second), /live before/u);
});

test('fixed acquisition environment removes only API tokens and preserves managed proxy and CA context', () => {
  const original = { GH_TOKEN: 'synthetic-api-token', GITHUB_TOKEN: 'synthetic-fallback-token', PATH: '/synthetic/bin',
    HTTPS_PROXY: 'https://synthetic-proxy', HTTP_PROXY: 'http://synthetic-proxy', NODE_EXTRA_CA_CERTS: '/synthetic/ca.pem',
    SSL_CERT_FILE: '/synthetic/ca.pem', CUSTOM_RUNTIME_HEADER: 'synthetic-injected-context', LC_ALL: 'synthetic-locale' };
  const expected = { ...original }; delete expected.GH_TOKEN; delete expected.GITHUB_TOKEN;
  const result = providerAcquisitionEnvironment(original); assert.deepEqual(result, expected);
  assert.equal(original.GH_TOKEN, 'synthetic-api-token'); result.PATH = 'copy'; assert.equal(original.PATH, '/synthetic/bin');
  assert.throws(() => providerAcquisitionEnvironment(original, ['CUSTOM_RUNTIME_HEADER']));
});
