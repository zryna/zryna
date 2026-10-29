// Complete synthetic heldout ZIP plus real tiny Git/source/raw/host owners; no API/native/workload execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { campaignFixture } from './cli-performance-campaign-fixture.mjs';
import { campaignArchiveFixture, campaignArchiveSource } from './cli-performance-campaign-archive-fixture.mjs';
import { windowsReplayFixture } from './cli-performance-windows-replay-fixture.mjs';
import { fixture as nativeFixture } from './cli-performance-campaign-windows-host-fixture.mjs';
import { addHosts } from './cli-performance-campaign-host-fixture.mjs';
import { reconcileNumericCampaignArchive, inspectNumericCampaignArchive, closeNumericCampaignArchive }
  from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { reconcileNumericCampaignHost, inspectNumericCampaignHost } from '../scripts/stability-gates/cli-performance-campaign-host-v1.mjs';
import { numericCampaignCollectorIdentity } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { producerDocument } from '../scripts/stability-gates/cli-performance-campaign-windows-host-v1.mjs';
import { numericSemanticTools } from '../scripts/stability-gates/cli-performance-campaign-normalization-projection-v1.mjs';
import { compareHeldoutHost, compareHeldoutTools, HELDOUT_FLAGS } from '../scripts/stability-gates/cli-performance-heldout-comparison-v1.mjs';
import { decodeObservedParentAffinity } from '../scripts/stability-gates/windows-parent-affinity-v1/decode.mjs';
import { replayNumericHeldoutOriginals } from '../scripts/stability-gates/cli-performance-heldout-replay-v1.mjs';
const P = 'm418-hosted-observations/capture/', Q = P + 'numeric-premeasurement/';
const get = (f, path) => JSON.parse(f.files.get(path));
const put = (f, path, value) => f.files.set(path, Buffer.from(canonical(value)));
function rewriteC(f, prefix, companion, mutate) {
  const record = get(f, prefix + '.stdout.raw'); mutate(record);
  const bytes = Buffer.from(JSON.stringify(record) + '\n'); f.files.set(prefix + '.stdout.raw', bytes);
  const result = get(f, prefix + '.result.json'); result.streams.stdout = { bytes: bytes.length, sha256: sha256(bytes) }; put(f, prefix + '.result.json', result);
  const value = get(f, companion);
  value.native.decoded = decodeObservedParentAffinity({ stdout: bytes, stderr: Buffer.alloc(0), exitCode: 0, signal: null, error: null, elapsedMs: result.elapsedMs },
    { parentPid: value.collectorNodePid, parentCreated: null });
  value.native.capture.stdoutSha256 = sha256(bytes); put(f, companion, value);
}
function open(f) {
  const rawKey = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
  try {
    const hostKey = reconcileNumericCampaignHost(f.selection, f.sourceKey, rawKey);
    return { rawKey, hostKey, raw: inspectNumericCampaignArchive(f.selection, f.sourceKey, rawKey),
      host: inspectNumericCampaignHost(f.selection, f.sourceKey, rawKey, hostKey) };
  } catch (error) { closeNumericCampaignArchive(f.selection, f.sourceKey, rawKey); throw error; }
}
const close = (f, keys) => closeNumericCampaignArchive(f.selection, f.sourceKey, keys.rawKey);
function replay(f, anchor) {
  const keys = open(f);
  try { return replayNumericHeldoutOriginals(f.selection, f.sourceKey, keys.rawKey, keys.hostKey, null, anchor); }
  finally { close(f, keys); }
}
async function fixture(t) {
  const document = (await windowsReplayFixture()).document;
  const sharedSource = campaignArchiveSource(t, campaignFixture('heldout'), document, true);
  const f = await campaignArchiveFixture(t, { windows: true, phase: 'heldout', hostEvidence: true, sharedSource });
  const native = nativeFixture(f); for (const [path, bytes] of native.files) f.files.set(path, bytes); addHosts(f, () => {}, native);
  for (const [path, bytes] of [...f.files]) if (path.startsWith(P + 'numeric-host-before-affinity') || [P + 'host-before.json', P + 'numeric-host-before.json'].includes(path))
    f.files.set(Q + path.slice(P.length), Buffer.from(bytes));
  rewriteC(f, P + 'numeric-host-after-affinity', P + 'numeric-host-after.json', value => { value.childCreated = String(BigInt(value.childCreated) + 2n); });
  rewriteC(f, Q + 'numeric-host-before-affinity', Q + 'numeric-host-before.json', value => { value.childPid += 2; value.childCreated = String(BigInt(value.childCreated) + 1n); });
  await f.reseal(); const keys = open(f), { raw, host } = keys, source = { commit: f.source.commit, tree: f.source.tree };
  let anchor;
  try {
    const tools = numericSemanticTools({ raw, host, provider: null,
      calibration: producerDocument(f.files.get('windows-job-metrics-calibration/toolchain-before-build.json')) });
    const collector = numericCampaignCollectorIdentity(f.selection, f.sourceKey);
    anchor = { ...HELDOUT_FLAGS, sourceFileBytesVerified: true, tableOriginalBytesVerified: true, tableTemporalOrderVerified: true,
      current: { host: raw.host, source: f.source, runId: raw.run.id, jobId: raw.jobId }, baselineSource: source,
      anchor: { runId: 10, jobId: 20, artifact: { id: 30, sha256: 'f'.repeat(64) } }, tools: [tools], collector,
      profile: { sources: { reference: { ...source }, candidate: { ...source } }, collectorSha256: collector.collectorSha256,
        hosts: [{ host: raw.host, classId: host.classId, facts: host.facts, candidateBuild: { class: 'release', cliSha256: raw.image.sha256 } }] } };
    for (const [index, name] of ['numeric-gate-before.json', 'numeric-gate-premeasurement.json'].entries()) {
      const directory = index ? Q : P, observation = { host: get(f, directory + 'host-before.json'), companion: get(f, directory + 'numeric-host-before.json') };
      put(f, P + name, { format: 'zryna.cli-performance-heldout-premeasurement.v1', phase: index ? 'immediately-before-timed-round' : 'before-release-build',
        host: compareHeldoutHost(anchor, observation, source), ...(index ? { tools: compareHeldoutTools(anchor, raw.host, source, tools, raw.image.sha256, collector.collectorSha256) } : {}),
        source, anchor: anchor.anchor, profileSha256: sha256(canonical(anchor.profile)), observedClassMatched: true,
        ...(index ? { observedToolsMatched: true } : {}), ...HELDOUT_FLAGS });
    }
  } finally { close(f, keys); }
  await f.reseal(); return { f, anchor };
}
test('complete original Windows premeasurement bytes join actual source/raw/host keys without qualifying the anchor', async t => {
  const { f, anchor } = await fixture(t), result = replay(f, anchor);
  assert.equal(result.premeasurementOriginalsVerified, true);
  assert.deepEqual(result.observationPhases, ['original-before', 'premeasurement-before']);
  assert(result.originalBindings.some(row => row.path === Q + 'numeric-host-before-affinity.stdout.raw'));
  assert(!result.originalBindings.some(row => row.path === P + 'numeric-host-after-affinity.stdout.raw'));
  for (const [key, value] of Object.entries(HELDOUT_FLAGS)) assert.equal(result[key], value);
  const keys = open(f); close(f, keys);
  assert.throws(() => replayNumericHeldoutOriginals(f.selection, f.sourceKey, keys.rawKey, keys.hostKey, null, anchor));
  assert.throws(() => replayNumericHeldoutOriginals({}, {}, {}, {}, null, anchor));
  assert.throws(() => replayNumericHeldoutOriginals(f.selection, f.sourceKey, {}, {}, null, anchor, () => {}), /fixed original/u);
});
test('missing/tampered original phase, profile, witness, source, host and C bytes remain failures after ZIP re-sealing', async t => {
  const { f, anchor } = await fixture(t), pristine = [...f.files].map(([path, bytes]) => [path, Buffer.from(bytes)]);
  const change = (path, mutate) => { const value = get(f, path); mutate(value); put(f, path, value); };
  const mutants = [() => f.files.delete(P + 'numeric-gate-before.json'), () => f.files.delete(Q + 'numeric-host-before-affinity.stderr.raw'),
    () => change(P + 'numeric-gate-premeasurement.json', value => { value.profileSha256 = '0'.repeat(64); }),
    () => change(P + 'numeric-gate-before.json', value => { value.sourceAuthenticated = true; }),
    () => change(P + 'numeric-gate-premeasurement.json', value => { value.tools.cliSha256 = '0'.repeat(64); }),
    () => change(P + 'numeric-gate-before.json', value => { value.anchor.runId++; }),
    () => change(Q + 'numeric-host-before.json', value => { value.source.commit = '0'.repeat(40); }),
    () => change(Q + 'numeric-host-before.json', value => { value.collectorNodePid++; }),
    () => change(Q + 'numeric-host-before.json', value => { value.phase = 'after'; }),
    () => {
      change(Q + 'host-before.json', value => { value.cpu.models = ['other CPU']; });
      const bytes = f.files.get(Q + 'host-before.json'); change(Q + 'numeric-host-before.json', value => { value.original.bytes = bytes.length; value.original.sha256 = sha256(bytes); });
    },
    () => rewriteC(f, Q + 'numeric-host-before-affinity', Q + 'numeric-host-before.json', value => { value.parentCreated = value.parentCreatedAfter = String(BigInt(value.parentCreated) - 1n); }),
    () => {
      for (const [path, bytes] of pristine) if (path.startsWith(P + 'numeric-host-before-affinity')) f.files.set(Q + path.slice(P.length), Buffer.from(bytes));
      put(f, Q + 'numeric-host-before.json', get(f, P + 'numeric-host-before.json'));
    },
    () => change(Q + 'numeric-host-before-affinity-materials-after.json', value => { value.expected.helper.sha256 = '0'.repeat(64); })];
  for (const mutate of mutants) {
    f.files.clear(); for (const [path, bytes] of pristine) f.files.set(path, Buffer.from(bytes)); mutate(); await f.reseal();
    assert.throws(() => replay(f, anchor));
  }
});
test('Linux original clock consistency rejects coherent copied and out-of-order pairs before provider admission', async t => {
  const f = await campaignArchiveFixture(t, { phase: 'heldout' });
  addHosts(f, (host, phase) => { host.observedAt = `2026-10-28T00:00:0${phase === 'before' ? 0 : 2}.000Z`; });
  for (const second of [0, 3, 1]) {
    const host = get(f, P + 'host-before.json'); host.observedAt = `2026-10-28T00:00:0${second}.000Z`;
    put(f, Q + 'host-before.json', host);
    const companion = get(f, P + 'numeric-host-before.json'), bytes = f.files.get(Q + 'host-before.json');
    companion.original = { path: 'host-before.json', bytes: bytes.length, sha256: sha256(bytes) };
    put(f, Q + 'numeric-host-before.json', companion); await f.reseal();
    const keys = open(f);
    try {
      const anchor = { current: { source: f.source, runId: keys.raw.run.id, jobId: keys.raw.jobId } };
      assert.throws(() => replayNumericHeldoutOriginals(f.selection, f.sourceKey, keys.rawKey, keys.hostKey, null, anchor),
        second === 1 ? /own provider key/u : /original Linux host clocks bracket/u);
    } finally { close(f, keys); }
  }
});
