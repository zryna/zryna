// Exact native Job COMMIT/QPC observations; never substitute RSS or a process sum.
import assert from 'node:assert/strict';
import { helperKeys, helperTypes, u64 } from './windows-job-metrics-v1/calibration/protocol.mjs';
import { MAX_LOG } from './input.mjs';
export const MEMORY_METHOD = 'job-peak-committed-bytes.v1';
export function parseWindowsHelperMetadata(bytes) {
  assert(bytes.length <= 4096 && bytes.every(byte => byte < 128), 'bounded ASCII helper protocol');
  const text = bytes.toString('ascii');
  assert(text.endsWith('\n'), 'complete helper metadata newline');
  const member = '"[A-Za-z][A-Za-z0-9]*"\\s*:\\s*(?:"[A-Za-z0-9_. -]*"|true|false|null|0|[1-9][0-9]*)';
  assert(new RegExp(`^\\s*\\{\\s*${member}(?:\\s*,\\s*${member})*\\s*\\}\\s*$`, 'u').test(text), 'flat primitive helper grammar');
  const h = JSON.parse(text);
  assert.equal([...text.matchAll(/"[A-Za-z][A-Za-z0-9]*"\s*:/gu)].length, Object.keys(h).length, 'no duplicate helper keys');
  return h; // checkedWindowsMetrics applies the existing closed keys/types and exact-u64 semantics.
}
export function checkedWindowsMetrics(h, capture, childArgc, stdout, stderr) {
  assert(Buffer.isBuffer(stdout) && Buffer.isBuffer(stderr));
  assert.deepEqual(Object.keys(h).sort(), [...helperKeys].sort(), 'closed helper metadata');
  helperTypes(h);
  assert.equal(h.measurementStatus, 'complete', 'only complete helper measurements');
  assert.equal(h.timeoutMs, capture.hostWallGuardMs);
  assert.equal(h.cleanupReserveMs, 10000); assert.equal(h.combinedStreamLimitBytes, MAX_LOG);
  assert.equal(h.childArgc, childArgc);
  assert(h.commandUtf16Units > 0 && h.commandUtf16Units < 32767);
  assert(h.childPid > 0);
  for (const key of ['childCreated', 'mainExitKnown', 'stdoutEof', 'stderrEof',
    'jobEmptyConfirmed', 'cleanupConfirmed', 'peakQuerySucceeded']) assert.equal(h[key], true, key);
  assert.equal(h.forcedTermination, false); assert.equal(h.mainExitCode, '0');
  for (const key of ['primaryStage', 'primaryWin32Error', 'cleanupStage', 'cleanupWin32Error']) assert.equal(h[key], 0, key);
  assert.equal(h.stdoutBytes, stdout.length); assert.equal(h.stderrBytes, stderr.length);
  assert(stdout.length + stderr.length <= MAX_LOG, 'same measured tree stream budget');
  const start = u64(h.qpcStart), end = u64(h.qpcEnd), ticks = u64(h.wallTicks);
  const frequency = u64(h.qpcFrequency), peak = u64(h.peakJobMemoryUsedBytes);
  assert(frequency > 0n && ticks > 0n && peak > 0n);
  assert(end > start && end - start === ticks, 'exact QPC delta');
  assert(ticks * 1000n <= BigInt(capture.hostWallGuardMs) * frequency, 'native QPC within guard');
  return { qpcStart: h.qpcStart, qpcEnd: h.qpcEnd, wallTicks: h.wallTicks,
    qpcFrequency: h.qpcFrequency, peakJobMemoryUsedBytes: h.peakJobMemoryUsedBytes };
}
function summary(values) {
  const ordered = values.map(u64).sort((a, b) => a < b ? -1 : a > b ? 1 : 0);
  assert(ordered.every(value => value > 0n));
  return { median: String(ordered[2]), maximum: String(ordered[4]),
    spread: { numerator: String(ordered[4] - ordered[0]), denominator: String(ordered[2]) } };
}
export function summarizeWindowsFive(samples) {
  assert.equal(samples.length, 5, 'all five measured attempts required');
  assert(samples.every(row => row.qpcFrequency === samples[0].qpcFrequency), 'one QPC frequency');
  assert(u64(samples[0].qpcFrequency) > 0n);
  return { wallTicks: { ...summary(samples.map(row => row.wallTicks)), qpcFrequency: samples[0].qpcFrequency },
    peakJobMemoryUsedBytes: summary(samples.map(row => row.peakJobMemoryUsedBytes)) };
}
