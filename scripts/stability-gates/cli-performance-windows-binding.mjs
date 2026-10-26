// Operator byte bindings only: no self-issued source, build, host, baseline or threshold authority.
import assert from 'node:assert/strict';
import { lstatSync } from 'node:fs';
import { isAbsolute, parse as parsePath, relative, resolve, win32 } from 'node:path';
import { canonical, exact, MAX_LOG, parse, readSafe, sha256 } from './input.mjs';
export const HELPER_DIRECTORY = 'scripts/stability-gates/windows-job-metrics-v1';
export const HELPER_SOURCE_PINS = `${HELPER_DIRECTORY}/calibration/source-pins.json`;
export const HELPER_SOURCE_SHA256 = 'ea0a832ad83b3d6473c99a359da96fb5f7a1587d1c76254f12e4a9dbeb188e3a';
export function localAbsolute(path) {
  assert(typeof path === 'string' && path.length < 4096 && /^[A-Za-z]:[\\/]/u.test(path), 'absolute local Windows path');
  assert.equal(win32.normalize(path), path, 'normalized Windows path');
  assert(!/[\x00-\x1f]/u.test(path) && !path.slice(2).includes(':'), 'no alternate device/stream path');
  return path;
}
export function validateWindowsCapture(document, capture) {
  canonical(capture);
  assert.equal(capture.format, 'zryna.cli-performance-windows-capture.draft.v1');
  assert.equal(capture.catalogueSha256, sha256(canonical(document)));
  assert.equal(capture.host.platform, 'windows-x86_64');
  assert(Object.keys(capture.host).length >= 2, 'actual host metadata retained');
  assert(['io-qualification', 'baseline-round-1', 'baseline-round-2', 'held-out-candidate'].includes(capture.round));
  assert(['debug', 'release'].includes(capture.cliBuildClass));
  assert(capture.cliBuildClass === 'release' || capture.round === 'io-qualification', 'debug-class CLI only qualifies IO');
  assert(Number.isSafeInteger(capture.hostWallGuardMs) && capture.hostWallGuardMs > 0 && capture.hostWallGuardMs <= 900000);
  assert.equal(capture.cleanupGuardMs, 10000); assert.equal(capture.combinedOutputBytes, MAX_LOG);
  for (const field of ['sourceCommit', 'sourceTree']) assert.match(capture[field], /^[a-f0-9]{40}$/u);
  for (const field of ['reviewRecordSha256', 'artifactOracleSha256']) assert.match(capture[field], /^[a-f0-9]{64}$/u);
  exact(capture.tools, ['helper', 'cli', 'node'], 'fixed tools');
  for (const pin of [...Object.values(capture.tools), capture.operatorAdmission]) {
    exact(pin, ['path', 'bytes', 'sha256'], 'operator file binding'); localAbsolute(pin.path);
    assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= 256 * 1024 * 1024);
    assert.match(pin.sha256, /^[a-f0-9]{64}$/u);
  }
}
function snapshot(pin) {
  const base = parsePath(pin.path).root;
  const before = lstatSync(pin.path);
  const bytes = readSafe(base, relative(base, pin.path).split('\\').join('/'), pin.bytes);
  const identity = stat => Object.fromEntries(['dev', 'ino', 'size', 'mode', 'nlink', 'mtimeMs', 'ctimeMs'].map(key => [key, stat[key]]));
  assert.deepEqual(identity(lstatSync(pin.path)), identity(before), 'stable operator-bound file identity');
  assert.equal(before.nlink, 1, 'private single-link operator input');
  assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256);
  return { identity: identity(before), bytes };
}
export function validateWindowsAdmission(capture, bytes) {
  assert.equal(bytes.length, capture.operatorAdmission.bytes);
  assert.equal(sha256(bytes), capture.operatorAdmission.sha256, 'exact operator admission bytes');
  const record = parse(bytes);
  exact(record, ['format', 'tools', 'helperBuiltFromSha256', 'sourceCommit', 'sourceTree',
    'catalogueSha256', 'artifactOracleSha256', 'reviewRecordSha256'], 'finite operator admission');
  assert.equal(record.format, 'zryna.cli-performance-windows-operator-binding.v1');
  assert.deepEqual(record.tools, capture.tools);
  assert.equal(record.helperBuiltFromSha256, HELPER_SOURCE_SHA256, 'helper binary/source operator binding');
  for (const key of ['sourceCommit', 'sourceTree', 'catalogueSha256', 'artifactOracleSha256', 'reviewRecordSha256']) assert.equal(record[key], capture[key]);
  return record;
}
export function openWindowsBinding(root, capture) {
  assert(isAbsolute(root) && root === resolve(root));
  const admission = snapshot(capture.operatorAdmission);
  validateWindowsAdmission(capture, admission.bytes);
  const sourceBytes = readSafe(root, HELPER_SOURCE_PINS);
  assert.equal(sha256(sourceBytes), HELPER_SOURCE_SHA256, 'reviewed helper source pin record');
  const pins = JSON.parse(sourceBytes);
  assert.equal(pins.format, 'zryna.windows-job-calibration-source-pins.v1'); assert.equal(pins.files.length, 29);
  const selected = pins.files.map(pin => ({ path: resolve(root, HELPER_DIRECTORY, pin.path), bytes: pin.bytes, sha256: pin.sha256 }));
  const bound = [capture.operatorAdmission, ...Object.values(capture.tools),
    { path: resolve(root, HELPER_SOURCE_PINS), bytes: sourceBytes.length, sha256: HELPER_SOURCE_SHA256 }, ...selected];
  const first = bound.map(snapshot);
  assert.equal(capture.tools.node.path.toLowerCase(), process.execPath.toLowerCase(), 'the bound Node runs this collector');
  const check = () => bound.forEach((pin, index) => assert.deepEqual(snapshot(pin).identity, first[index].identity, 'operator inputs unchanged since admission'));
  return { check, admissionBytes: admission.bytes };
}
