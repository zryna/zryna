// Synthetic bytes and API responses only; no host/guest/measurement/authentication qualification.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { inspectPerformancePlatformLink } from '../scripts/stability-gates/cli-performance-platform-link.mjs';
import { reconcileHostedRawMetrics, inspectHostedRawMetrics } from '../scripts/stability-gates/cli-performance-linked-metrics-v1.mjs';
import { exactWindows, pair } from './cli-performance-linked-metrics-fixture.mjs';
const issued = f => reconcileHostedRawMetrics(f.selection, f.artifactId, f.bytes);
const inspected = f => inspectHostedRawMetrics(f.selection, issued(f));
function flags(value) {
  for (const key of ['rawAuthenticated', 'sourceAuthenticated', 'hostAuthenticated', 'proofQualified',
    'performanceQualified', 'baselineAdmitted']) assert.equal(value[key], false, key);
  assert.equal(value.baselineAuthority, null);
  assert.equal(value.thresholdAuthority, null);
  assert.equal(value.thresholds, null);
  assert.equal(value.closureStatus, 'blocked');
}
test('Linux original six-attempt order and checked native units survive without summary substitution', async () => {
  const f = await pair(), key = issued(f), value = inspectHostedRawMetrics(f.selection, key);
  assert(Object.isFrozen(key)); assert.equal(value.cellCount, 24);
  assert.equal(value.attemptCount, 144); assert.equal(value.warmupCount, 24); assert.equal(value.measuredCount, 120);
  assert.equal(value.memoryMethod, 'gnu-time-wait4-maxrss-kib.v1');
  assert.equal(value.wallEncoding, 'original-wall-ns-decimal');
  assert.equal(value.capture.sha256, sha256(Buffer.from(canonical(f.raw.capture))));
  assert.equal(value.session.sha256, sha256(Buffer.from(canonical(f.raw.session))));
  assert.equal(value.artifact.id, f.artifactId); assert.equal(value.source.commit, f.raw.capture.sourceCommit);
  for (const [i, cell] of value.cells.entries()) {
    assert.equal(cell.selector, f.raw.session.cells[i].selector);
    assert.deepEqual(cell.attempts.map(row => row.wallNs), ['14000000000', '100', '900', '300', '500', '700']);
    assert.deepEqual(cell.attempts.map(row => row.peakRssKiB), ['99999', '11', '19', '13', '17', '15']);
    assert.deepEqual(cell.attempts.map(row => row.index), [0, 1, 2, 3, 4, 5]);
    assert.deepEqual(cell.attempts.map(row => row.role), ['warmup', ...Array(5).fill('measured')]);
    assert.deepEqual(cell.attempts.map(row => row.id), f.raw.session.cells[i].attempts.map(row => row.id));
    assert(cell.attempts.every(row => !Object.hasOwn(row, 'outcome') && !Object.hasOwn(row, 'summary')));
  }
  for (const key of ['classBefore', 'classAfter', 'build', 'startedAt', 'endedAt']) assert(!Object.hasOwn(value, key));
  flags(value);
});
test('Windows all six original QPC/COMMIT strings stay distinct from Linux nanoseconds/RSS', async () => {
  const f = await pair('windows-x86_64'), value = inspected(f);
  assert.equal(value.cellCount, 20); assert.equal(value.attemptCount, 120);
  assert.equal(value.warmupCount, 20); assert.equal(value.measuredCount, 100);
  assert.equal(value.memoryMethod, 'job-peak-committed-bytes.v1');
  assert.equal(value.wallEncoding, 'original-qpc-ticks-and-frequency-decimal');
  for (const cell of value.cells) {
    assert.deepEqual(cell.attempts.map(row => row.wallTicks), ['1000', '5', '1', '3', '2', '4']);
    assert.deepEqual(cell.attempts.map(row => row.peakJobMemoryUsedBytes), ['999', '13', '17', '11', '19', '15']);
    assert(cell.attempts.every(row => row.qpcStart === '9007199254740993000' && row.qpcFrequency === '1000000000'));
    assert(cell.attempts.every(row => !Object.hasOwn(row, 'wallNs') && !Object.hasOwn(row, 'peakRssKiB')));
  }
  assert.equal(value.capture.sha256, sha256(Buffer.from(canonical(f.raw.capture)))); flags(value);
});
test('replayed native values above Number precision never round through a median or float', async () => {
  const f = await pair('windows-x86_64', { mutate: exactWindows }), value = inspected(f);
  for (const cell of value.cells) for (const [index, row] of cell.attempts.entries()) {
    assert.equal(row.wallTicks, String(9007199254740992n + BigInt(index)));
    assert.equal(row.qpcFrequency, '18014398509481984');
    assert.equal(row.peakJobMemoryUsedBytes, String(9007199254741000n - BigInt(index)));
    assert.equal(BigInt(row.qpcEnd) - BigInt(row.qpcStart), BigInt(row.wallTicks));
  }
  assert.notEqual(value.cells[0].attempts[0].wallTicks, value.cells[0].attempts[1].wallTicks); flags(value);
});
test('only actual issued platform/metrics keys bind; copied reports and a different live selection reject', async () => {
  const f = await pair(), key = issued(f), other = await pair();
  for (const fake of [{}, { ...f.selection }, inspectPerformancePlatformLink(f.selection)])
    assert.throws(() => reconcileHostedRawMetrics(fake, f.artifactId, f.bytes));
  for (const fake of [{}, { ...key }, inspectHostedRawMetrics(f.selection, key)])
    assert.throws(() => inspectHostedRawMetrics(f.selection, fake));
  assert.throws(() => inspectHostedRawMetrics(other.selection, key), /SAME live platform selection/u);
  flags(inspectHostedRawMetrics(f.selection, key));
});
test('caller ZIP/projection mutations cannot change a reconciled key; altered bytes cannot issue another key', async () => {
  const f = await pair(), key = issued(f), pristine = inspectHostedRawMetrics(f.selection, key);
  const copy = inspectHostedRawMetrics(f.selection, key);
  copy.cells[0].attempts[1].wallNs = '999999'; copy.source.commit = 'f'.repeat(40); copy.artifact.id = 0;
  copy.performanceQualified = true; copy.pending.length = 0;
  f.bytes.fill(0);
  assert.deepEqual(inspectHostedRawMetrics(f.selection, key), pristine);
  assert.throws(() => issued(f));
  assert.deepEqual(inspectHostedRawMetrics(f.selection, key), pristine);
});
test('exact primary selection excludes provider/unknown IDs, truncation and metric/read callbacks', async () => {
  const f = await pair();
  for (const id of [0, '301', 999999, f.platform.value.jobs[0].provider.id])
    assert.throws(() => reconcileHostedRawMetrics(f.selection, id, f.bytes));
  assert.throws(() => reconcileHostedRawMetrics(f.selection, f.artifactId, f.bytes.subarray(0, -1)));
  const changed = Buffer.from(f.bytes); changed[30] ^= 1;
  assert.throws(() => reconcileHostedRawMetrics(f.selection, f.artifactId, changed));
  let called = false;
  assert.throws(() => reconcileHostedRawMetrics(f.selection, f.artifactId, f.bytes, () => { called = true; }));
  assert.equal(called, false); flags(inspected(f));
});
test('platform-pinned source/tree/host/round relabels still fail the existing raw replayer', async () => {
  for (const options of [{ jobHost: 'windows-x86_64' }, { round: 'baseline-round-2' },
    { source: { commit: 'f'.repeat(40), tree: 'e'.repeat(40) } },
    { source: { commit: 'a'.repeat(40), tree: 'e'.repeat(40) } }]) {
    const f = await pair('linux-x86_64', options);
    assert.throws(() => issued(f), /selected (source|round|host)/u);
  }
});
test('even newly platform-pinned archives cannot replace original values with summaries, omit attempts or swap units', async () => {
  for (const mutate of [raw => { raw.session.cells[0].summary.wallNs.median = 123; },
    raw => { raw.session.cells[0].attempts.pop(); },
    raw => { raw.session.memoryMethod = 'job-peak-committed-bytes.v1'; },
    raw => { raw.session.cells[0].attempts[1].wallNs = raw.session.cells[0].summary.wallNs.median; }]) {
    const f = await pair('linux-x86_64', { mutate });
    assert.throws(() => issued(f));
  }
});
test('original Linux raw decimals must agree with session before a key exists; pristine input still reconciles', async () => {
  const f = await pair('linux-x86_64', { mutate(raw) {
    const path = `round/${raw.session.cells[0].attempts[1].id}.json`;
    const row = JSON.parse(raw.files.get(path)); row.wallNs = '123';
    raw.files.set(path, Buffer.from(canonical(row)));
  } });
  assert.throws(() => issued(f));
  const pristine = await pair(); flags(inspected(pristine));
});
