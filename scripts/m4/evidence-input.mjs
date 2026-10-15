import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, parseCanonical, portablePath, readStableFile } from './registry-files.mjs';

export const MAX_RECEIPT = 2 * 1024 * 1024;
export const MAX_LOG = 128 * 1024 * 1024;
export const MAX_ARTIFACT = 256 * 1024 * 1024;
export const reject = message => { throw new Error(`M4 evidence: ${message}`); };
export function exact(value, keys, label) {
  if (!value || typeof value !== 'object' || Array.isArray(value)) reject(`${label} object required`);
  assert.deepEqual(Object.keys(value).sort(), [...keys].sort(), `${label} closed keys`);
}
export function text(value, label, maximum = 512) {
  if (typeof value !== 'string' || !value || value.length > maximum || /[\x00-\x1f\x7f]/.test(value)) {
    reject(`${label} bounded text required`);
  }
  return value;
}
export function integer(value, minimum, maximum, label) {
  if (!Number.isSafeInteger(value) || value < minimum || value > maximum) reject(`${label} integer bound`);
}
export function sha(value, label = 'digest') {
  if (!/^[a-f0-9]{64}$/.test(value ?? '')) reject(`${label} SHA-256 required`);
}
export function oid(value) {
  if (!/^[a-f0-9]{40}$/.test(value ?? '')) reject('full Git object ID required');
}
export function array(value, maximum, label) {
  if (!Array.isArray(value) || value.length > maximum) reject(`${label} bounded array required`);
}
export function utf8(bytes) {
  try { return new TextDecoder('utf-8', { fatal: true }).decode(bytes); }
  catch { reject('log is not strict UTF-8'); }
}
export function retained(descriptor, root, maximum = MAX_LOG, expectedPath) {
  exact(descriptor, ['path', 'bytes', 'sha256'], 'retained descriptor');
  portablePath(descriptor.path);
  if (expectedPath !== undefined) assert.equal(descriptor.path, expectedPath, 'retained path binding');
  integer(descriptor.bytes, 0, maximum, 'retained bytes');
  sha(descriptor.sha256);
  const bytes = readStableFile(path.join(root, descriptor.path), maximum, descriptor.path);
  assert.equal(bytes.length, descriptor.bytes, 'retained byte count');
  assert.equal(digest(bytes), descriptor.sha256, 'retained digest');
  return bytes;
}
export function retainedJson(descriptor, root, maximum = MAX_RECEIPT) {
  return parseCanonical(retained(descriptor, root, maximum), descriptor.path);
}
export function attachmentMap(result) {
  array(result.attachments, 96, 'attachments');
  const map = new Map();
  const folded = new Set();
  for (const item of result.attachments) {
    portablePath(item.path);
    if (folded.has(item.path.toLowerCase())) reject('attachment duplicate or case collision');
    folded.add(item.path.toLowerCase());
    map.set(item.path, item);
  }
  return map;
}
export function processResult(result, root, prefix) {
  exact(result, ['id', 'argv', 'launchArgv', 'exitCode', 'signal', 'error', 'elapsedMs', 'stdout', 'stderr',
    'attachments'], 'result');
  array(result.argv, 48, 'argv');
  result.argv.forEach(arg => text(arg, 'argument', 2048));
  integer(result.elapsedMs, 1, 6 * 60 * 60 * 1000, 'elapsedMs');
  if (result.exitCode !== 0 || result.signal !== null || result.error !== null) reject(`${result.id} process failed`);
  return {
    stdout: utf8(retained(result.stdout, root, MAX_LOG, `${prefix}.stdout.log`)).replace(/\r\n/g, '\n'),
    stderr: utf8(retained(result.stderr, root, MAX_LOG, `${prefix}.stderr.log`)).replace(/\r\n/g, '\n'),
  };
}
