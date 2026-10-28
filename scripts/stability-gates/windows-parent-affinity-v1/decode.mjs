// Pure declaration decoder. Actual binary/process/source linkage is a separate collector owner.
import assert from 'node:assert/strict';
import { exact } from '../input.mjs';
export const RECORD_BYTES = 2047; // C buffer2048 includes its terminator; no output-cap increase.
const KEYS = ['format', 'status', 'stage', 'error', 'cleanupError', 'childPid', 'parentPid',
  'childCreated', 'parentCreated', 'parentCreatedAfter', 'parentLiveBefore', 'parentLiveAfter',
  'snapshotsMatch', 'snapshotClosed', 'parentHandleClosed', 'groupCount', 'groupId',
  'activeProcessorCount', 'processMask', 'systemMask', 'maskBitCount', 'afterGroupCount',
  'afterGroupId', 'afterActiveProcessorCount', 'afterProcessMask', 'afterSystemMask', 'elapsedMs'];
function uint(value, maximum = 0xffffffff) {
  assert(Number.isSafeInteger(value) && value >= 0 && value <= maximum, 'bounded unsigned integer');
  return value;
}
function u64(value) {
  assert(typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/u.test(value), 'canonical decimal u64');
  const integer = BigInt(value);
  assert(integer > 0n && integer <= 0xffffffffffffffffn, 'positive u64');
  return integer;
}
function bits(value) {
  let count = 0;
  while (value) { count += Number(value & 1n); value >>= 1n; }
  return count;
}
function flat(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= RECORD_BYTES, 'bounded original record');
  assert(bytes.every(byte => byte < 128), 'ASCII record');
  const text = bytes.toString('ascii');
  assert(text.endsWith('\n'), 'complete record newline');
  const member = '"[A-Za-z][A-Za-z0-9]*"\\s*:\\s*(?:"[A-Za-z0-9_. -]*"|true|false|null|0|[1-9][0-9]*)';
  assert(new RegExp(`^\\s*\\{\\s*${member}(?:\\s*,\\s*${member})*\\s*\\}\\s*$`, 'u').test(text), 'flat primitive record');
  const record = JSON.parse(text);
  assert.equal([...text.matchAll(/"[A-Za-z][A-Za-z0-9]*"\s*:/gu)].length,
    Object.keys(record).length, 'duplicate record key');
  exact(record, KEYS, 'fixed affinity record');
  const emitter = `${JSON.stringify(Object.fromEntries(KEYS.map(key => [key, record[key]])))}\n`;
  assert(bytes.equals(Buffer.from(emitter, 'ascii')), 'exact emitter bytes and one LF, no CRT conversion');
  return record;
}
export function decodeObservedParentAffinity(result, expected) {
  exact(result, ['stdout', 'stderr', 'exitCode', 'signal', 'error', 'elapsedMs'], 'original execution result');
  exact(expected, ['parentPid', 'parentCreated'], 'live collector expected identity');
  assert(uint(expected.parentPid) > 0);
  if (expected.parentCreated !== null) u64(expected.parentCreated);
  assert.equal(result.exitCode, 0); assert.equal(result.signal, null); assert.equal(result.error, null);
  assert(Buffer.isBuffer(result.stderr) && result.stderr.length === 0, 'complete empty probe stderr');
  assert(Number.isFinite(result.elapsedMs) && result.elapsedMs >= 0 && result.elapsedMs <= 10000,
    'unchanged outer observation budget');
  const record = flat(result.stdout);
  assert.equal(record.format, 'zryna.windows-parent-affinity.v1'); assert.equal(record.status, 'observed');
  for (const key of ['stage', 'error', 'cleanupError']) assert.equal(record[key], 0, key);
  for (const key of ['parentLiveBefore', 'parentLiveAfter', 'snapshotsMatch', 'snapshotClosed', 'parentHandleClosed'])
    assert.equal(record[key], true, key);
  assert.equal(uint(record.parentPid), expected.parentPid); assert(uint(record.childPid) > 0);
  assert.notEqual(record.childPid, record.parentPid);
  const created = u64(record.parentCreated);
  assert(created < u64(record.childCreated), 'original parent predates probe, not a reused parent PID');
  assert.equal(record.parentCreatedAfter, record.parentCreated, 'same retained process handle identity');
  if (expected.parentCreated !== null) assert.equal(record.parentCreated, expected.parentCreated, 'same collector as prior phase');
  assert.equal(record.groupCount, 1); assert.equal(record.afterGroupCount, 1, 'single-group scope only');
  uint(record.groupId, 65534); // 65535 means ALL_PROCESSOR_GROUPS, never a single-group identity.
  assert.equal(record.afterGroupId, record.groupId);
  assert(uint(record.activeProcessorCount, 64) > 0);
  assert.equal(record.afterActiveProcessorCount, record.activeProcessorCount);
  const mask = u64(record.processMask), system = u64(record.systemMask);
  assert.equal(mask & ~system, 0n, 'process mask is within returned system mask of the SAME group');
  assert.equal(record.afterProcessMask, record.processMask); assert.equal(record.afterSystemMask, record.systemMask);
  assert.equal(uint(record.maskBitCount, 64), bits(mask));
  assert(bits(system) <= record.activeProcessorCount, 'no cross-group active-count inference');
  uint(record.elapsedMs, 9999);
  return { format: 'zryna.windows-parent-affinity-projection.v1', status: 'observed-unqualified',
    scope: 'one-original-parent-one-group-at-two-snapshots', parent: { pid: record.parentPid, created: record.parentCreated },
    affinity: { groupId: record.groupId, processMask: record.processMask, systemMask: record.systemMask,
      maskBitCount: record.maskBitCount, activeProcessorCount: record.activeProcessorCount },
    originalRecord: record, sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    hostAdmitted: false, closureStatus: 'blocked', baselineAuthority: null, thresholdAuthority: null, thresholds: null,
    unobserved: ['outer CPU/memory quotas', 'physical exclusivity', 'power policy', 'OS kernel version', 'CPU-set/scheduling restrictions outside process affinity'] };
}
