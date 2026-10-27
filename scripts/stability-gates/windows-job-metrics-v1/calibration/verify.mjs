import assert from 'node:assert/strict';
import { join } from 'node:path';
import { flat, helperKeys, ownerKeys, helperTypes, uint, u64, boolean } from './protocol.mjs';
import { readBounded, raw } from './files.mjs';
export function verify(directory, expected, index, processResult) {
  assert.ifError(processResult.error); assert.equal(processResult.signal, null); assert.equal(processResult.status, 0, 'C owner completed protocol');
  const o = flat(join(directory, 'owner.json'), 8192, ownerKeys);
  const h = flat(join(directory, 'sample/result.json'), 4096, helperKeys); helperTypes(h);
  assert.equal(o.format, 'zryna.windows-job-calibration-owner.v1');
  for (const key of ['caseId', 'slots', 'errorStage', 'errorWin32', 'cleanupErrorWin32', 'observedMask', 'readyMask', 'outerBeforeActive']) uint(o[key]);
  for (const key of ['helperExitKnown', 'mainExitedBeforeRelease', 'helperExitedWhileHeld', 'peakSnapshot', 'outerBeforeKnown', 'cleanupForced', 'cleanupConfirmed']) boolean(o[key]);
  assert.equal(o.caseId, index + 1); assert.equal(o.slots, expected.slots);
  assert.equal(o.errorStage, 0); assert.equal(o.errorWin32, 0); assert.equal(o.cleanupErrorWin32, 0);
  assert.equal(o.observedMask, (1 << o.slots) - 1); assert.equal(o.readyMask, (1 << o.slots) - 1);
  assert.equal(o.outerBeforeKnown, true); assert.equal(o.outerBeforeActive, 0);
  assert.equal(o.cleanupForced, false, 'outer cleanup must not manufacture expected owned-tree closure'); assert.equal(o.cleanupConfirmed, true);
  assert.equal(o.helperExitKnown, true); assert.equal(u64(o.helperExit), expected.measurementValid ? 0n : 1n);
  assert.equal(h.timeoutMs, expected.helperTimeoutMs); assert.equal(h.cleanupReserveMs, 10000); assert.equal(h.combinedStreamLimitBytes, 16777216);
  assert.equal(h.childArgc, expected.name === 'success' ? 9 : 4); assert(h.commandUtf16Units > 0 && h.commandUtf16Units < 32767);
  assert.equal(h.childCreated, true); assert.equal(h.mainExitKnown, true); assert.equal(h.childPid, o.pid0);
  assert.equal(h.mainExitCode, o.exit0, 'original retained main handle status binding'); assert(u64(h.mainExitCode) <= 0xffffffffn);
  assert.equal(h.cleanupStage, 0); assert.equal(h.cleanupWin32Error, 0); assert.equal(h.cleanupConfirmed, true); assert.equal(h.jobEmptyConfirmed, true);
  assert(h.stdoutBytes + h.stderrBytes <= 16777216); assert(u64(h.qpcFrequency) > 0n); assert.equal(h.qpcFrequency, o.qpcFrequency);
  const start = u64(h.qpcStart), frequency = u64(h.qpcFrequency), markers = [];
  for (let i = 0; i < 3; ++i) {
    for (const key of ['pid', 'creationLow', 'creationHigh', 'requestBytes']) uint(o[`${key}${i}`]);
    for (const key of ['bound', 'goneBeforeCleanup', 'exitKnown']) boolean(o[`${key}${i}`]);
    if (i < o.slots) {
      assert(o[`pid${i}`] > 0 && (o[`creationLow${i}`] !== 0 || o[`creationHigh${i}`] !== 0));
      assert.equal(o[`bound${i}`], true); assert.equal(o[`goneBeforeCleanup${i}`], true); assert.equal(o[`exitKnown${i}`], true);
      const ready = u64(o[`readyQpc${i}`]); assert(ready >= start);
      assert((ready - start) * 1000n < BigInt(h.timeoutMs) * frequency, 'genuine READY before helper expiry');
      assert(u64(o[`exit${i}`]) <= 0xffffffffn);
      if (o[`endQpc${i}`] !== null) markers.push(u64(o[`endQpc${i}`]));
      if (expected.name === 'timeout') assert.equal(u64(o[`exit${i}`]), 0xe4180001n, 'same Job timeout termination status');
      else if (expected.name !== 'first-extra') assert.equal(u64(o[`exit${i}`]), expected.name === 'exit17' ? 17n : 0n);
    } else {
      for (const key of ['pid', 'creationLow', 'creationHigh', 'requestBytes']) assert.equal(o[`${key}${i}`], 0);
      for (const key of ['bound', 'goneBeforeCleanup', 'exitKnown']) assert.equal(o[`${key}${i}`], false);
      for (const key of ['exit', 'readyQpc', 'endQpc', 'peakCommitBytes']) assert.equal(o[`${key}${i}`], null);
    }
  }
  if (expected.measurementValid) {
    assert.equal(h.measurementStatus, 'complete'); assert.equal(h.mainExitCode, '0');
    for (const key of ['stdoutEof', 'stderrEof', 'peakQuerySucceeded']) assert.equal(h[key], true);
    assert.equal(h.forcedTermination, false); assert.equal(h.primaryStage, 0); assert.equal(h.primaryWin32Error, 0);
    const end = u64(h.qpcEnd); assert(end >= start); assert.equal(u64(h.wallTicks), end - start); assert(u64(h.peakJobMemoryUsedBytes) > 0n);
    for (const marker of markers) assert(end >= marker, 'owned drain occurs after original process marker');
  } else {
    assert.equal(h.measurementStatus, 'invalid'); assert.equal(h.primaryStage, expected.primaryStage); assert.equal(h.forcedTermination, true);
    assert.equal(h.primaryWin32Error, expected.name === 'timeout' ? 258 : expected.name === 'first-extra' ? 111 : 0);
    assert.equal(h.peakQuerySucceeded, false); assert.equal(h.peakJobMemoryUsedBytes, null); assert.equal(h.qpcEnd, null); assert.equal(h.wallTicks, null);
    if (expected.name === 'exit17') assert.equal(h.mainExitCode, '17');
  }
  if (expected.mainExitedBeforeRelease) {
    assert.equal(o.mainExitedBeforeRelease, true); assert.equal(o.helperExitedWhileHeld, false);
    const release = u64(o.releaseQpc); assert(u64(o.endQpc0) <= release); assert(u64(o.endQpc1) >= release); assert(u64(h.qpcEnd) >= u64(o.endQpc1));
  }
  if (expected.explicitReadyReleaseOverlap) {
    assert.equal(o.peakSnapshot, true); const release = u64(o.releaseQpc), total = BigInt(expected.minimumJobCommitBytes);
    for (let i = 0; i < 3; ++i) {
      assert.equal(o[`requestBytes${i}`], expected.committedBytes[i]);
      assert(u64(o[`readyQpc${i}`]) <= release); assert(u64(o[`endQpc${i}`]) >= release);
      const peak = u64(o[`peakCommitBytes${i}`]); assert(peak > 0n && peak >= BigInt(expected.committedBytes[i]));
      assert(peak < total, 'calibrator process peak must distinguish the96MiB aggregate'); assert(u64(h.peakJobMemoryUsedBytes) > peak);
    }
    assert(u64(h.peakJobMemoryUsedBytes) >= total, 'actual concurrent32+64MiB Job COMMIT');
  } else assert.equal(o.peakSnapshot, false);
  const stdout = raw(join(directory, 'sample/stdout.raw'), h.stdoutBytes, expected.stdout);
  const stderr = raw(join(directory, 'sample/stderr.raw'), h.stderrBytes, expected.stderr);
  assert.equal(readBounded(join(directory, 'helper.stdout.raw'), 4096).length, 0);
  const helperError = readBounded(join(directory, 'helper.stderr.raw'), 4096);
  const diagnostic = 'm418 Windows Job metrics failed; retain owned output and tool exit status\n';
  const messages = [Buffer.from(diagnostic), Buffer.from(diagnostic.replace('\n', '\r\n'))];
  assert(expected.measurementValid ? helperError.length === 0 : messages.some(message =>
    helperError.length <= message.length && message.subarray(0, helperError.length).equals(helperError)), 'bounded diagnostic only, never status inference');
  return { owner: o, helper: h, raw: { stdout, stderr }, measurementUsable: expected.measurementValid };
}
