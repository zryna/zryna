// Synthetic host controls plus tiny private Git source fixtures; no native helper or workload runs.
import assert from 'node:assert/strict';
import test from 'node:test';
import { campaignArchiveFixture } from './cli-performance-campaign-archive-fixture.mjs';
import { reconcileNumericCampaignArchive, closeNumericCampaignArchive }
  from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { canonical, MAX_DOCUMENT, sha256 } from '../scripts/stability-gates/input.mjs';
import { producerDocument }
  from '../scripts/stability-gates/cli-performance-campaign-windows-host-v1.mjs';
import { reconcileNumericCampaignHost, inspectNumericCampaignHost }
  from '../scripts/stability-gates/cli-performance-campaign-host-v1.mjs';
import { decodeObservedParentAffinity } from '../scripts/stability-gates/windows-parent-affinity-v1/decode.mjs';
import { fixture, run, mutateDocument, PREFIX, DIRECTORY } from './cli-performance-campaign-windows-host-fixture.mjs';
import { addHosts } from './cli-performance-campaign-host-fixture.mjs';
test('self-consistently pinned helpers still require original bounded PE AMD64 headers', () => {
  for (const mutate of [bytes => bytes.writeUInt32LE(0, 60), bytes => bytes.writeUInt32LE(0xffffffff, 60),
    bytes => bytes.fill(0, 64, 68), bytes => bytes.writeUInt16LE(0x14c, 68)])
    assert.throws(() => run(fixture(null, mutate)), /helper PE/u);
});
test('stable positive hardlink observations remain observations while zero and changing identities reject', () => {
  const f = fixture();
  const paths = [...f.files.keys()].filter(path => /-materials-(before|after)\.json$/u.test(path));
  for (const path of paths) mutateDocument(f, path, value => {
    for (const row of value.observed) row.identity.nlink = '2';
  });
  assert.equal(run(f).decoded.length, 2);
  for (const invalid of ['0', '00', '-1', '1'.repeat(33)]) {
    const g = fixture();
    for (const path of paths) mutateDocument(g, path, value => { for (const row of value.observed) row.identity.nlink = invalid; });
    assert.throws(() => run(g));
  }
  mutateDocument(f, paths[0], value => { value.observed[0].identity.nlink = '3'; });
  assert.throws(() => run(f), /stable/u);
});
test('synthetic original Windows records bind all materials and re-decode both native phases without authority', () => {
  const f = fixture(), result = run(f);
  assert.equal(result.parent.pid, 123); assert.equal(result.parent.created, '133000000000000000');
  assert.equal(result.decoded.length, 2); assert.equal(result.decoded[0].hostAdmitted, false);
  assert.equal(result.toolInputs.sourceFiles.length, 6);
  const changed = fixture(); changed.companions[0].native.decoded.affinity.maskBitCount = 1;
  assert.throws(() => run(changed), /ORIGINAL/u);
});
test('original C process records, helper bytes, source bytes and complete material bindings cannot be replaced by copied decoded reports', () => {
  const cases = [f => { f.files.set(PREFIX + 'numeric-host-before-affinity.stdout.raw', Buffer.from('{}\n')); },
    f => { f.files.get('windows-parent-affinity-observation/build/windows-parent-affinity-v1.exe')[0] = 0; },
    f => { f.originals.set(`${DIRECTORY}/probe.c`, Buffer.from('changed original\n')); },
    f => mutateDocument(f, PREFIX + 'numeric-host-after-affinity-materials-after.json', row => { row.observed[0].sha256 = 'f'.repeat(64); }),
    f => mutateDocument(f, PREFIX + 'numeric-host-before-affinity.command.json', row => { row.args[1] = '124'; }),
    f => mutateDocument(f, PREFIX + 'numeric-host-after-affinity.result.json', row => { row.error = 'CLEANUP'; }),
    f => mutateDocument(f, 'windows-job-metrics-calibration/toolchain-before-build.json', row => { row.compiler.sha256 = 'f'.repeat(64); }),
    f => { f.tools.buildAfter.cc = { ...f.tools.buildAfter.cc, sha256: 'f'.repeat(64) }; }];
  for (const change of cases) { const f = fixture(); change(f); assert.throws(() => run(f)); }
});
test('native parent birth must remain identical even if after metadata and copied projection are consistently rewritten', () => {
  const f = fixture(), path = PREFIX + 'numeric-host-after-affinity.stdout.raw';
  const row = JSON.parse(f.files.get(path)); row.parentCreated = row.parentCreatedAfter = '133000000000000003'; row.childCreated = '133000000000000004';
  const stdout = Buffer.from(JSON.stringify(row) + '\n'); f.files.set(path, stdout);
  mutateDocument(f, PREFIX + 'numeric-host-after-affinity.result.json', value => { value.streams.stdout = { bytes: stdout.length, sha256: sha256(stdout) }; });
  f.companions[1].native.decoded = decodeObservedParentAffinity({ stdout, stderr: Buffer.alloc(0), exitCode: 0, signal: null, error: null, elapsedMs: 10 }, { parentPid: 123, parentCreated: null });
  f.companions[1].native.capture.stdoutSha256 = sha256(stdout);
  assert.throws(() => run(f), /same collector as prior phase/u);
});
test('producer JSON is bounded and lossless, and copied phase/source/raw/host keys cannot mint a host observation', () => {
  for (const raw of [Buffer.alloc(0), Buffer.alloc(MAX_DOCUMENT + 1), Buffer.from('{"a":1,"a":1}\n')]) assert.throws(() => producerDocument(raw));
  for (const key of [{}, { hostFactBytesVerified: true }, Object.freeze({})]) {
    assert.throws(() => reconcileNumericCampaignHost(key, {}, {}));
    assert.throws(() => inspectNumericCampaignHost({}, {}, {}, key));
  }
  assert.throws(() => reconcileNumericCampaignHost({}, {}, {}, () => {}), /fixed live/u);
});

test('actual private source/archive owner keys join synthetic Linux host bytes; copied and crossed keys remain powerless', async t => {
  const f = await campaignArchiveFixture(t); addHosts(f); await f.reseal();
  const rawKey = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
  try {
    const key = reconcileNumericCampaignHost(f.selection, f.sourceKey, rawKey);
    const value = inspectNumericCampaignHost(f.selection, f.sourceKey, rawKey, key);
    assert.equal(value.hostFactBytesVerified, true); assert.equal(value.nativeParentBindingVerified, false);
    assert.equal(value.hostAdmitted, false); assert.equal(value.performanceQualified, false);
    assert.equal(value.facts.visibleCpuCount.value, '2'); assert.equal(value.acceptedUnknownFacts.length, 4);
    for (const args of [[f.selection, f.sourceKey, rawKey, value], [{}, f.sourceKey, rawKey, key],
      [f.selection, {}, rawKey, key], [f.selection, f.sourceKey, {}, key]])
      assert.throws(() => inspectNumericCampaignHost(...args));
    value.facts.visibleCpuCount.value = '999';
    assert.equal(inspectNumericCampaignHost(f.selection, f.sourceKey, rawKey, key).facts.visibleCpuCount.value, '2');
    closeNumericCampaignArchive(f.selection, f.sourceKey, rawKey);
    assert.throws(() => inspectNumericCampaignHost(f.selection, f.sourceKey, rawKey, key));
  } finally { /* The raw key is deliberately closed by the test before the final assertion. */ }
});
test('self-consistent original host metadata cannot change PID, phase image, schema or stable kernel identity', async t => {
  const f = await campaignArchiveFixture(t);
  for (const change of [host => { host.affinity.text = host.affinity.text.replace('Pid:\t123', 'Pid:\t124'); },
    host => { host.image.GITHUB_RUN_ID.value = '999'; },
    host => { delete host.memory; },
    (host, phase) => { if (phase === 'after') host.kernel.release = 'changed'; }]) {
    addHosts(f, change); await f.reseal();
    const key = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
    try { assert.throws(() => reconcileNumericCampaignHost(f.selection, f.sourceKey, key)); }
    finally { closeNumericCampaignArchive(f.selection, f.sourceKey, key); }
  }
});

test('Windows public SAME-key route joins original source/build/PE/material/native bytes into one opaque host key', async t => {
  const f = await campaignArchiveFixture(t, { windows: true, hostEvidence: true }), native = fixture(f);
  for (const [path, bytes] of native.files) f.files.set(path, bytes);
  addHosts(f, () => {}, native); await f.reseal();
  const raw = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
  try {
    const key = reconcileNumericCampaignHost(f.selection, f.sourceKey, raw);
    const value = inspectNumericCampaignHost(f.selection, f.sourceKey, raw, key);
    assert.equal(value.host, 'windows-x86_64'); assert.equal(value.nativeParentBindingVerified, true);
    assert.equal(value.parent.created, '133000000000000000'); assert.equal(value.facts.visibleCpuCount.value, '4');
    assert.equal(value.hostAdmitted, false); assert.equal(value.sourceAuthenticated, false); assert.equal(value.proofQualified, false);
    assert.equal(value.performanceQualified, false); assert.equal(value.baselineAdmitted, false);
  } finally { closeNumericCampaignArchive(f.selection, f.sourceKey, raw); }
  // Coherently copied companion/kernel/original hash cannot replace the original C process output.
  f.files.set(PREFIX + 'numeric-host-after-affinity.stdout.raw', Buffer.from('{}\n')); await f.reseal();
  const changed = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
  try { assert.throws(() => reconcileNumericCampaignHost(f.selection, f.sourceKey, changed)); }
  finally { closeNumericCampaignArchive(f.selection, f.sourceKey, changed); }
});
