// Pure expectations for one qualified installer transformation; no filesystem authority.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { isAbsolute, resolve } from 'node:path';
import { vsixEntries } from '../../portable-setup/vsix.mjs';
const digest = bytes => createHash('sha256').update(bytes).digest('hex');
const pin = bytes => ({ bytes: bytes.length, sha256: digest(bytes) });
function closed(value, names) {
  assert(value && typeof value === 'object' && !Array.isArray(value));
  assert.deepEqual(Object.keys(value).sort(), [...names].sort());
}
function bodyPin(value) {
  closed(value, ['bytes', 'sha256']);
  assert(Number.isSafeInteger(value.bytes) && value.bytes >= 0 && value.bytes <= 1024 * 1024);
  assert(typeof value.sha256 === 'string' && /^[a-f0-9]{64}$/.test(value.sha256));
}
function payloadPins(payload) {
  assert(payload && typeof payload === 'object' && !Array.isArray(payload));
  assert.equal(Object.keys(payload).length, 12);
  assert(Object.hasOwn(payload, 'package.json') && !Object.hasOwn(payload, '.vsixmanifest'));
  for (const [path, value] of Object.entries(payload)) {
    assert(/^[A-Za-z0-9_./-]+$/.test(path) && !path.startsWith('/')
      && path.split('/').every(part => part && part !== '.' && part !== '..'));
    bodyPin(value);
  }
}
function sourcePin(value) {
  closed(value, ['path', 'bytes', 'sha256', 'identity']);
  assert(typeof value.path === 'string' && value.path.length <= 32768
    && isAbsolute(value.path) && resolve(value.path) === value.path);
  assert(Number.isSafeInteger(value.bytes) && value.bytes > 0 && value.bytes <= 4 * 1024 * 1024);
  assert(typeof value.sha256 === 'string' && /^[a-f0-9]{64}$/.test(value.sha256));
  closed(value.identity, ['dev', 'ino', 'bytes', 'links', 'mode', 'modifiedNs', 'changedNs']);
  for (const name of ['dev', 'ino', 'bytes', 'links', 'mode']) {
    assert(typeof value.identity[name] === 'string' && /^(0|[1-9][0-9]{0,31})$/.test(value.identity[name]));
  }
  for (const name of ['modifiedNs', 'changedNs']) {
    assert(typeof value.identity[name] === 'string' && /^-?(0|[1-9][0-9]{0,31})$/.test(value.identity[name]));
  }
  assert.equal(value.identity.bytes, String(value.bytes));
  assert.equal(value.identity.links, '1');
  assert.notEqual(value.identity.ino, '0');
}
function packageJson(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= 1024 * 1024);
  const value = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
  assert(value && typeof value === 'object' && !Array.isArray(value));
  return value;
}
function freeze(value) {
  if (value && typeof value === 'object') {
    for (const item of Object.values(value)) freeze(item);
    Object.freeze(value);
  }
  return value;
}
export function validateInstallerIdentity(identity, expected) {
  closed(identity, ['format', 'vsix', 'payload', 'packageJsonBase64', 'manifest']);
  assert.equal(identity.format, 'zryna.installed-editor-vsix-source.v1');
  sourcePin(identity.vsix);
  payloadPins(identity.payload);
  payloadPins(expected);
  assert.deepEqual(identity.payload, expected);
  bodyPin(identity.manifest);
  assert(identity.manifest.bytes > 0);
  assert(typeof identity.packageJsonBase64 === 'string' && identity.packageJsonBase64.length <= 1398104);
  const raw = Buffer.from(identity.packageJsonBase64, 'base64');
  assert.equal(raw.toString('base64'), identity.packageJsonBase64);
  assert.deepEqual(pin(raw), expected['package.json']);
  const original = packageJson(raw);
  assert(!Object.hasOwn(original, '__metadata'), 'Original authenticated package metadata cannot be overwritten.');
  return original;
}
export function deriveInstallerIdentity(bytes, expected, vsix) {
  assert(Buffer.isBuffer(bytes));
  sourcePin(vsix);
  assert.equal(bytes.length, vsix.bytes);
  assert.equal(digest(bytes), vsix.sha256);
  const entries = vsixEntries(bytes);
  const payload = Object.fromEntries(entries.filter(entry => entry.name.startsWith('extension/'))
    .map(entry => [entry.name.slice(10), pin(entry.data)]));
  payloadPins(payload);
  assert.equal(entries.length, 14);
  const manifest = entries.find(entry => entry.name === 'extension.vsixmanifest');
  assert(manifest && entries.some(entry => entry.name === '[Content_Types].xml'));
  const original = entries.find(entry => entry.name === 'extension/package.json');
  assert(original);
  const identity = { format: 'zryna.installed-editor-vsix-source.v1', vsix: structuredClone(vsix),
    payload, packageJsonBase64: original.data.toString('base64'), manifest: pin(manifest.data) };
  validateInstallerIdentity(identity, expected);
  return freeze(identity);
}
function windowBounds(window, timestamp) {
  closed(window, ['before', 'after']);
  assert(Number.isSafeInteger(window.before) && Number.isSafeInteger(window.after)
    && window.before > 0 && window.before <= window.after && window.after - window.before <= 600000);
  assert(Number.isSafeInteger(timestamp) && timestamp >= window.before && timestamp <= window.after);
}
function metadata(identity, timestamp) {
  const size = Object.values(identity.payload).reduce((total, value) => total + value.bytes,
    identity.manifest.bytes);
  assert(Number.isSafeInteger(size) && size > 0 && size <= 13 * 1024 * 1024);
  return { installedTimestamp: timestamp, targetPlatform: 'undefined', size };
}
function serialized(original, fields) {
  return Buffer.from(JSON.stringify({ ...original, __metadata: fields }, null, '\t'));
}
export function bindInstaller(identity, expected, window, installedPackage) {
  const original = validateInstallerIdentity(identity, expected);
  const actual = packageJson(installedPackage);
  const fields = actual.__metadata;
  closed(fields, ['installedTimestamp', 'targetPlatform', 'size']);
  assert.deepEqual(Object.keys(fields), ['installedTimestamp', 'targetPlatform', 'size']);
  windowBounds(window, fields.installedTimestamp);
  assert.deepEqual(fields, metadata(identity, fields.installedTimestamp));
  assert(installedPackage.equals(serialized(original, fields)), 'Exact qualified package serialization required.');
  return freeze({ format: 'zryna.installed-editor-installer.v1', identity: structuredClone(identity),
    window: structuredClone(window), installedTimestamp: fields.installedTimestamp });
}
export function installedPayload(expected, installer) {
  closed(installer, ['format', 'identity', 'window', 'installedTimestamp']);
  assert.equal(installer.format, 'zryna.installed-editor-installer.v1');
  const original = validateInstallerIdentity(installer.identity, expected);
  windowBounds(installer.window, installer.installedTimestamp);
  const bytes = serialized(original, metadata(installer.identity, installer.installedTimestamp));
  return { ...expected, 'package.json': pin(bytes), '.vsixmanifest': { ...installer.identity.manifest } };
}
