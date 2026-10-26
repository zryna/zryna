import assert from 'node:assert/strict';
import { readBounded } from './files.mjs';
export const helperKeys = ['format', 'metric', 'measurementStatus', 'timeoutMs', 'cleanupReserveMs', 'combinedStreamLimitBytes',
  'childArgc', 'commandUtf16Units', 'childCreated', 'childPid', 'mainExitKnown', 'mainExitCode', 'stdoutBytes', 'stderrBytes',
  'stdoutEof', 'stderrEof', 'jobEmptyConfirmed', 'forcedTermination', 'cleanupConfirmed', 'peakQuerySucceeded',
  'peakJobMemoryUsedBytes', 'qpcFrequency', 'qpcStart', 'qpcEnd', 'wallTicks', 'primaryStage', 'primaryWin32Error', 'cleanupStage', 'cleanupWin32Error'];
const slotKeys = ['pid', 'creationLow', 'creationHigh', 'bound', 'goneBeforeCleanup', 'exitKnown', 'exit', 'readyQpc', 'endQpc', 'requestBytes', 'peakCommitBytes'];
export const ownerKeys = ['format', 'caseId', 'slots', 'errorStage', 'errorWin32', 'cleanupErrorWin32', 'observedMask', 'readyMask',
  'helperExitKnown', 'helperExit', 'mainExitedBeforeRelease', 'helperExitedWhileHeld', 'peakSnapshot', 'outerBeforeKnown',
  'outerBeforeActive', 'cleanupForced', 'cleanupConfirmed', 'qpcFrequency', 'releaseQpc',
  ...Array.from({ length: 3 }, (_, i) => slotKeys.map(key => `${key}${i}`)).flat()];
// Deliberately flat protocol: C emits only ASCII primitive fields, no escaped keys/nested data.
export function flat(path, maximum, keys) {
  const bytes = readBounded(path, maximum); assert(bytes.every(byte => byte < 128), 'ASCII fixed protocol');
  const text = bytes.toString('ascii'); assert(text.endsWith('\n'), 'complete metadata newline');
  let at = 0; const object = Object.create(null);
  const ws = () => { while (/[ \t\r\n]/u.test(text[at] ?? 'x')) ++at; };
  const literal = value => { ws(); assert.equal(text[at++], value, 'flat JSON grammar'); };
  literal('{');
  for (;;) {
    ws(); if (text[at] === '}') { ++at; break; }
    const key = /^"([A-Za-z][A-Za-z0-9]*)"/u.exec(text.slice(at)); assert(key, 'fixed metadata key'); at += key[0].length;
    assert(!Object.hasOwn(object, key[1]), 'duplicate metadata key'); literal(':'); ws();
    const value = /^(?:"[A-Za-z0-9_. -]*"|true|false|null|0|[1-9][0-9]*)/u.exec(text.slice(at));
    assert(value, 'flat primitive metadata value'); at += value[0].length; object[key[1]] = JSON.parse(value[0]); ws();
    if (text[at] === '}') { ++at; break; } literal(',');
    ws(); assert.notEqual(text[at], '}', 'trailing metadata comma');
  }
  ws(); assert.equal(at, text.length, 'no extra/truncated metadata');
  assert.deepEqual(Object.keys(object).sort(), [...keys].sort(), 'closed metadata inventory'); return object;
}
export function uint(value, maximum = 0xffffffff) { assert(Number.isSafeInteger(value) && value >= 0 && value <= maximum); return value; }
export function u64(value) {
  assert(typeof value === 'string' && /^(0|[1-9][0-9]{0,19})$/u.test(value), 'canonical exact unsigned64 string');
  const n = BigInt(value); assert(n <= 0xffffffffffffffffn); return n;
}
export function boolean(value) { assert.equal(typeof value, 'boolean'); return value; }
export function helperTypes(h) {
  assert.equal(h.format, 'zryna.stability.windows-job-metrics.v1'); assert.equal(h.metric, 'job-peak-committed-bytes.v1');
  for (const key of ['timeoutMs', 'cleanupReserveMs', 'combinedStreamLimitBytes', 'childArgc', 'commandUtf16Units', 'childPid',
    'stdoutBytes', 'stderrBytes', 'primaryStage', 'primaryWin32Error', 'cleanupStage', 'cleanupWin32Error']) uint(h[key]);
  for (const key of ['childCreated', 'mainExitKnown', 'stdoutEof', 'stderrEof', 'jobEmptyConfirmed', 'forcedTermination', 'cleanupConfirmed', 'peakQuerySucceeded']) boolean(h[key]);
  for (const key of ['mainExitCode', 'peakJobMemoryUsedBytes', 'qpcFrequency', 'qpcStart', 'qpcEnd', 'wallTicks']) if (h[key] !== null) u64(h[key]);
  assert(['complete', 'invalid'].includes(h.measurementStatus));
}
