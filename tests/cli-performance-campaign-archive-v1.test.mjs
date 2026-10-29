// Synthetic original raw records only; no source/host/campaign/application qualification.
import assert from 'node:assert/strict';
import test from 'node:test';
import { retainNumericArchiveSamples } from '../scripts/stability-gates/cli-performance-campaign-samples-v1.mjs';
import { reconcileNumericCampaignArchive, inspectNumericCampaignArchive,
  readNumericCampaignArchiveEntry, closeNumericCampaignArchive } from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { replayLinuxRound } from '../scripts/stability-gates/cli-performance-linux-replay.mjs';
import { replayWindowsRound } from '../scripts/stability-gates/cli-performance-windows-replay.mjs';
import { linuxReplayFixture } from './cli-performance-linux-replay-fixture.mjs';
import { windowsReplayFixture } from './cli-performance-windows-replay-fixture.mjs';
import { exactWindows } from './cli-performance-linked-metrics-fixture.mjs';

test('Linux original raw replay yields all six exact native attempts without summary substitution', () => {
  const f = linuxReplayFixture(); assert.equal(replayLinuxRound(f).rawMetricsReconciled, true);
  const samples = retainNumericArchiveSamples('linux-x86_64', f.session);
  assert.equal(samples.length, 24); assert.equal(samples.reduce((n, cell) => n + cell.attempts.length, 0), 144);
  for (let cell = 0; cell < samples.length; cell++) for (let i = 0; i < 6; i++) {
    const row = samples[cell].attempts[i], original = f.session.cells[cell].attempts[i];
    assert.equal(row.wallNs, String(original.wallNs)); assert.equal(row.peakRssKiB, String(original.peakRssKiB));
    assert.equal(row.id, original.id); assert.equal(row.index, i); assert.equal(row.role, i === 0 ? 'warmup' : 'measured');
  }
  // This projection helper is not a validator/issuer: it reads original attempts, never summaries.
  const copy = structuredClone(f.session); copy.cells[0].summary = { ignored: 'not a sample' };
  assert.deepEqual(retainNumericArchiveSamples('linux-x86_64', copy), samples);
});
test('Windows original QPC and Job COMMIT strings above safe Number precision remain distinct', async () => {
  const f = await windowsReplayFixture(); exactWindows(f);
  assert.equal(replayWindowsRound(f).rawMetricsReconciled, true);
  const samples = retainNumericArchiveSamples('windows-x86_64', f.session);
  assert.equal(samples.length, 20); assert.equal(samples.reduce((n, cell) => n + cell.attempts.length, 0), 120);
  const rows = samples[0].attempts;
  assert.equal(rows[0].wallTicks, '9007199254740992'); assert.equal(rows[1].wallTicks, '9007199254740993');
  assert.equal(rows[0].qpcFrequency, '18014398509481984'); assert.equal(rows[0].peakJobMemoryUsedBytes, '9007199254741000');
  assert.notEqual(rows[0].wallTicks, rows[1].wallTicks);
  for (const [index, row] of rows.entries()) for (const key of ['qpcStart', 'qpcEnd', 'wallTicks', 'qpcFrequency', 'peakJobMemoryUsedBytes']) {
    assert.equal(typeof row[key], 'string'); assert.equal(row[key], f.session.cells[0].attempts[index][key]);
  }
});
test('sample projections own their primitive output and confer no opaque or normalized authority', () => {
  const f = linuxReplayFixture(), samples = retainNumericArchiveSamples('linux-x86_64', f.session);
  const wall = samples[0].attempts[0].wallNs;
  f.session.cells[0].attempts[0].wallNs++; assert.equal(samples[0].attempts[0].wallNs, wall);
  samples[0].attempts[0].wallNs = '0';
  assert.notEqual(retainNumericArchiveSamples('linux-x86_64', f.session)[0].attempts[0].wallNs, '0');
  for (const key of ['sourceAuthenticated', 'performanceQualified', 'classId', 'buildId', 'collectedAt', 'outcome']) {
    assert.equal(Object.hasOwn(samples[0].attempts[0], key), false);
  }
});
test('native projection rejects missing attempts, role replacement and nonnative numeric encodings', async () => {
  for (const mutate of [s => { s.cells.pop(); }, s => { s.cells[0].attempts.pop(); },
    s => { s.cells[0].attempts[0].role = 'measured'; }, s => { s.cells[0].attempts[1].wallNs = 0.5; },
    s => { s.cells[0].attempts[1].peakRssKiB = 1000000000000000; }]) {
    const s = structuredClone(linuxReplayFixture().session); mutate(s);
    assert.throws(() => retainNumericArchiveSamples('linux-x86_64', s));
  }
  const f = await windowsReplayFixture(); f.session.cells[0].attempts[0].wallTicks = 1;
  assert.throws(() => retainNumericArchiveSamples('windows-x86_64', f.session));
});
test('fixed archive API rejects caller reader/source/metric overrides before any callback can run', () => {
  assert.equal(reconcileNumericCampaignArchive.length, 4);
  let called = 0;
  assert.throws(() => reconcileNumericCampaignArchive({}, {}, 1, Buffer.alloc(22), () => { called++; }), /fixed archive owner/u);
  assert.equal(called, 0);
  for (const bytes of [null, {}, new Uint8Array(22), Buffer.alloc(0), Buffer.alloc(21)]) {
    assert.throws(() => reconcileNumericCampaignArchive({}, {}, 1, bytes), /bounded original primary ZIP/u);
  }
});
test('serialized platform/source claims cannot open an archive or mint a successful raw key', () => {
  const copied = { format: 'zryna.cli-performance-campaign-platform-link.v1', plan: {}, jobs: [], sourceAuthenticated: false };
  for (const selection of [{}, Object.freeze({}), copied, structuredClone(copied)]) {
    assert.throws(() => reconcileNumericCampaignArchive(selection, {}, 1, Buffer.alloc(22)), /live numeric campaign selection/u);
  }
});
test('copied raw projections cannot inspect, read or close retained original archive bytes', () => {
  assert.equal(inspectNumericCampaignArchive.length, 3); assert.equal(readNumericCampaignArchiveEntry.length, 5);
  assert.equal(closeNumericCampaignArchive.length, 3);
  for (const key of [{}, Object.freeze({}), { format: 'zryna.cli-performance-campaign-archive.v1', rawMetrics: true }]) {
    assert.throws(() => inspectNumericCampaignArchive({}, {}, key), /own archive key/u);
    assert.throws(() => readNumericCampaignArchiveEntry({}, {}, key, 'm418-hosted-observations/capture/host-before.json', 262144), /own archive key/u);
    assert.throws(() => closeNumericCampaignArchive({}, {}, key), /own archive key/u);
  }
});
