// Inert synthetic wire controls only: no Windows process, compiler, collector, network or clock calls.
import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeObservedParentAffinity, RECORD_BYTES } from './decode.mjs';
const expected = () => ({ parentPid: 123, parentCreated: null });
const record = () => ({ format: 'zryna.windows-parent-affinity.v1', status: 'observed',
  stage: 0, error: 0, cleanupError: 0, childPid: 456, parentPid: 123,
  childCreated: '133000000000000001', parentCreated: '133000000000000000', parentCreatedAfter: '133000000000000000',
  parentLiveBefore: true, parentLiveAfter: true, snapshotsMatch: true, snapshotClosed: true, parentHandleClosed: true,
  groupCount: 1, groupId: 0, activeProcessorCount: 4, processMask: '15', systemMask: '15', maskBitCount: 4,
  afterGroupCount: 1, afterGroupId: 0, afterActiveProcessorCount: 4, afterProcessMask: '15', afterSystemMask: '15', elapsedMs: 5 });
const result = row => ({ stdout: Buffer.from(`${JSON.stringify(row)}\n`), stderr: Buffer.alloc(0),
  exitCode: 0, signal: null, error: null, elapsedMs: 10 });
function rejectRow(change, pattern) {
  const row = record(); change(row);
  assert.throws(() => decodeObservedParentAffinity(result(row), expected()), pattern);
  assert.equal(decodeObservedParentAffinity(result(record()), expected()).status, 'observed-unqualified', 'pristine retry');
}
test('synthetic current-parent projection retains native identity/masks without admission', () => {
  const row = record(), decoded = decodeObservedParentAffinity(result(row), expected());
  assert.deepEqual(decoded.originalRecord, row); assert.deepEqual(decoded.parent, { pid: 123, created: row.parentCreated });
  assert.deepEqual(decoded.affinity, { groupId: 0, processMask: '15', systemMask: '15', maskBitCount: 4, activeProcessorCount: 4 });
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted']) assert.equal(decoded[key], false);
  assert.equal(decoded.closureStatus, 'blocked'); assert.equal(decoded.thresholdAuthority, null);
  assert.equal(decoded.baselineAuthority, null); assert.equal(decoded.thresholds, null);
});
test('kernel FILETIME and 64-bit masks stay exact beyond Number precision', () => {
  const row = record();
  row.processMask = row.systemMask = row.afterProcessMask = row.afterSystemMask = '18446744073709551615';
  row.maskBitCount = row.activeProcessorCount = row.afterActiveProcessorCount = 64;
  const decoded = decodeObservedParentAffinity(result(row), { parentPid: 123, parentCreated: row.parentCreated });
  assert.equal(decoded.affinity.processMask, '18446744073709551615');
  rejectRow(r => { r.parentCreatedAfter = '133000000000000001'; });
});
test('wrong parent, PID reuse/equal birth, or changed prior-phase identity rejects', () => {
  rejectRow(r => { r.parentPid = 124; }); rejectRow(r => { r.childPid = r.parentPid; });
  rejectRow(r => { r.childCreated = r.parentCreated; });
  rejectRow(r => { r.parentCreated = r.parentCreatedAfter = '133000000000000002'; });
  assert.throws(() => decodeObservedParentAffinity(result(record()), { parentPid: 123, parentCreated: '133000000000000001' }));
});
test('failed native calls, live checks or handle cleanup never become observed', () => {
  for (const key of ['stage', 'error', 'cleanupError']) rejectRow(r => { r[key] = 5; });
  for (const key of ['parentLiveBefore', 'parentLiveAfter', 'snapshotsMatch', 'snapshotClosed', 'parentHandleClosed'])
    rejectRow(r => { r[key] = false; });
  rejectRow(r => { r.status = 'unavailable'; });
});
test('multi-group, changed group, zero/outside masks or cross-group count guesses reject', () => {
  for (const key of ['groupCount', 'afterGroupCount']) rejectRow(r => { r[key] = 2; });
  rejectRow(r => { r.afterGroupId = 1; });
  for (const group of [65535, 65536]) rejectRow(r => { r.groupId = r.afterGroupId = group; });
  rejectRow(r => { r.processMask = r.afterProcessMask = '0'; });
  rejectRow(r => { r.processMask = r.afterProcessMask = '16'; r.maskBitCount = 1; });
  rejectRow(r => { r.activeProcessorCount = r.afterActiveProcessorCount = 3; });
  rejectRow(r => { r.maskBitCount = 3; }); rejectRow(r => { r.afterSystemMask = '31'; });
  rejectRow(r => { r.afterProcessMask = '7'; }); rejectRow(r => { r.afterActiveProcessorCount = 5; });
});
test('exact decimal unsigned fields reject truncation, overflow and alternate spellings', () => {
  for (const value of ['01', '0xF', '-1', '1e3', '18446744073709551616', 15])
    rejectRow(r => { r.processMask = r.afterProcessMask = value; });
  rejectRow(r => { r.parentPid = 0; }); rejectRow(r => { r.childPid = 0x100000000; });
  rejectRow(r => { r.elapsedMs = 10000; }); rejectRow(r => { r.elapsedMs = 1.5; });
});
test('timeout, signal, nonzero, stderr or missing execution facts cannot validate a raw record', () => {
  for (const delta of [{ exitCode: 1 }, { error: 'ETIMEDOUT' }, { signal: 'SIGTERM' },
    { stderr: Buffer.from('failed') }, { elapsedMs: 10001 }, { elapsedMs: NaN }])
    assert.throws(() => decodeObservedParentAffinity({ ...result(record()), ...delta }, expected()));
  const missing = result(record()); delete missing.error;
  assert.throws(() => decodeObservedParentAffinity(missing, expected()));
});
test('truncated, duplicate, extra, escaped and non-ASCII wire fields fail closed', () => {
  const wire = result(record()).stdout.toString('ascii');
  const invalid = [wire.trimEnd(), wire.slice(0, -3), wire.replace('"stage":0', '"stage":0,"stage":0'),
    wire.replace('"stage":0', '"stage":0,"waiver":true'), wire.replace('observed', 'observ\\u0065d'),
    wire.replace('observed', 'observé'), wire.replace('\n', '\r\n'), wire.replace('{', '{ '), `${wire}{}`];
  for (const text of invalid) assert.throws(() => decodeObservedParentAffinity({ ...result(record()), stdout: Buffer.from(text) }, expected()));
});
test('record first-extra is rejected before text/JSON work and copied reports get no authority', () => {
  const huge = Buffer.alloc(RECORD_BYTES + 1, 32);
  assert.throws(() => decodeObservedParentAffinity({ ...result(record()), stdout: huge }, expected()), /bounded original record/u);
  assert.throws(() => decodeObservedParentAffinity(result({ ...record(), sourceAuthenticated: true }), expected()));
  const decoded = decodeObservedParentAffinity(result(record()), expected());
  assert.throws(() => decodeObservedParentAffinity(result(decoded), expected()));
  assert.equal(decodeObservedParentAffinity(result(record()), expected()).sourceAuthenticated, false);
});
