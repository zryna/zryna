// Binding-only provider admission. No source, host, release, or baseline authority.
import assert from 'node:assert/strict';
import { lstatSync, realpathSync } from 'node:fs';
import { dirname, join, posix } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical, exact, parse, readSafe, sha256 } from './input.mjs';
export const PROVIDER_SHA256 = '94e1aea6a73b5b2f1021678c8d8d655cc81f052cf7c0cb0ad1dd46dfc89a2506';
export const GNU_TIME = Object.freeze({ bytes: 27384,
  sha256: 'efc0d1112e36ec76c14bf7508e287935cf1631d9d0d91fb532802d07aef51cea' });
export const PACKAGE = Object.freeze({ bytes: 50776,
  sha256: '10257c6fe888e6578550b1f20705d282dd630b76f8df0c006aec3031fefe5e23' });
export const DOWNLOAD_URL = 'https://deb.debian.org/debian/pool/main/t/time/time_1.9-0.2_amd64.deb';
export const ROOT_PREFIX = '/tmp/m418-linux-provider-';
export const RECEIPT_NAME = 'runtime-receipt.json';
const RECORD = fileURLToPath(new URL('./provider-inputs/linux-gnu-time-debian-trixie-v2.json', import.meta.url));
export function providerRoot(root) {
  assert(typeof root === 'string' && new RegExp(`^${ROOT_PREFIX}[A-Za-z0-9]{6}$`, 'u').test(root),
    'fresh fixed-recipe private provider root');
  return root;
}
export function timeExecutable(root) {
  return { path: posix.join(providerRoot(root), 'extracted/usr/bin/time'), ...GNU_TIME };
}
export function validateUbuntuRelease(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length <= 16384);
  const text = bytes.toString('utf8');
  assert(/^ID=ubuntu$/mu.test(text) && /^VERSION_ID="24\.04"$/mu.test(text), 'Ubuntu 24.04 only');
}
export function validateProviderRecord(bytes) {
  const record = parse(bytes);
  assert.equal(sha256(bytes), PROVIDER_SHA256, 'fixed operator recipe identity');
  assert.equal(record.sourceAuthenticated, false); assert.equal(record.performanceQualified, false);
  assert.equal(record.baselineAuthority, null); assert.equal(record.thresholds, null);
  assert.equal(record.historicalAnchor.currentHostVerification, false);
  return record;
}
export function validateProviderBinding(capture) {
  assert.equal(capture.providerRecordSha256, PROVIDER_SHA256);
  providerRoot(capture.providerRoot);
  assert.match(capture.providerReceiptSha256, /^[a-f0-9]{64}$/u);
  assert.deepEqual(capture.tools.time, timeExecutable(capture.providerRoot));
  assert(!Object.hasOwn(capture, 'providerOriginSha256'), 'historical origin is not current receipt');
}
const identityKeys = ['dev', 'ino', 'size', 'mode', 'nlink', 'uid', 'gid', 'mtimeMs', 'ctimeMs'];
export const fileIdentity = stat => Object.fromEntries(identityKeys.map(key => [key, stat[key]]));
export function validateObservedFile(pin, { path, bytes, sha256: digest, executable = false } = {}) {
  exact(pin, ['path', 'bytes', 'sha256', 'identity'], 'observed file');
  assert(posix.isAbsolute(pin.path) && posix.resolve(pin.path) === pin.path && pin.path.length < 4096);
  if (path !== undefined) assert.equal(pin.path, path);
  assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= 32 * 1024 * 1024);
  assert.match(pin.sha256, /^[a-f0-9]{64}$/u);
  if (bytes !== undefined) assert.equal(pin.bytes, bytes);
  if (digest !== undefined) assert.equal(pin.sha256, digest);
  exact(pin.identity, identityKeys, 'observed identity');
  for (const key of identityKeys) assert(Number.isSafeInteger(pin.identity[key]) ||
    (['mtimeMs', 'ctimeMs'].includes(key) && Number.isFinite(pin.identity[key])));
  assert.equal(pin.identity.size, pin.bytes);
  assert.equal(pin.identity.nlink, 1, 'single-link observed input');
  assert.equal(pin.identity.mode & 0o170000, 0o100000, 'regular file identity');
  assert.equal(pin.identity.mode & 0o022, 0, 'no group/other writable input');
  if (executable) assert(pin.identity.mode & 0o111, 'executable admitted input');
  return pin;
}
export function validateRuntimeReceipt(bytes, root) {
  providerRoot(root);
  const receipt = parse(bytes);
  exact(receipt, ['format', 'providerRoot', 'providerRecordSha256', 'acquisition', 'host', 'package', 'tool',
    'runtime', 'provenance', 'sourceAuthenticated', 'performanceQualified', 'baselineAuthority',
    'thresholds'], 'runtime receipt');
  assert.equal(receipt.format, 'zryna.gnu-time-runtime-receipt.v1');
  assert.equal(receipt.providerRoot, root); assert.equal(receipt.providerRecordSha256, PROVIDER_SHA256);
  exact(receipt.acquisition, ['curl', 'version', 'receipt'], 'acquisition tools');
  validateObservedFile(receipt.acquisition.curl, { path: '/usr/bin/curl', executable: true });
  validateObservedFile(receipt.acquisition.receipt, { path: posix.join(root, 'acquisition.json') });
  assert.equal(receipt.acquisition.receipt.identity.mode & 0o7777, 0o600);
  assert(typeof receipt.acquisition.version === 'string' && receipt.acquisition.version.length > 0 &&
    Buffer.byteLength(receipt.acquisition.version) <= 16384);
  exact(receipt.host, ['platform', 'arch', 'osRelease'], 'host');
  assert.equal(receipt.host.platform, 'linux'); assert.equal(receipt.host.arch, 'x64');
  validateObservedFile(receipt.host.osRelease, { path: '/usr/lib/os-release' });
  validateObservedFile(receipt.package, { path: posix.join(root, 'time.deb'), ...PACKAGE });
  assert.equal(receipt.package.identity.mode & 0o7777, 0o600);
  validateObservedFile(receipt.tool.file, { ...timeExecutable(root), executable: true });
  assert.equal(receipt.tool.file.identity.mode & 0o7777, 0o755);
  exact(receipt.tool, ['file', 'version'], 'GNU time');
  assert.equal(receipt.tool.version.split('\n')[0], 'time (GNU Time) UNKNOWN');
  exact(receipt.runtime, ['dpkgDeb', 'dpkgQuery', 'loader', 'libc', 'sharedProviders', 'versions'], 'runtime');
  for (const [key, path] of [['dpkgDeb', '/usr/bin/dpkg-deb'], ['dpkgQuery', '/usr/bin/dpkg-query']])
    validateObservedFile(receipt.runtime[key], { path, executable: true });
  for (const key of ['loader', 'libc']) validateObservedFile(receipt.runtime[key], { executable: true });
  assert.match(receipt.runtime.loader.path, /^\/usr\/lib\/x86_64-linux-gnu\/ld-linux-x86-64\.so\.2$/u);
  assert.equal(receipt.runtime.libc.path, '/usr/lib/x86_64-linux-gnu/libc.so.6');
  assert(Array.isArray(receipt.runtime.sharedProviders) && receipt.runtime.sharedProviders.length > 0 &&
    receipt.runtime.sharedProviders.length <= 24);
  const paths = receipt.runtime.sharedProviders.map(pin => {
    validateObservedFile(pin);
    assert.match(pin.path, /^\/usr\/lib\/x86_64-linux-gnu\/[A-Za-z0-9_.+-]+$/u);
    return pin.path;
  });
  assert.deepEqual(paths, [...new Set(paths)].sort(), 'bounded unique sorted runtime closure');
  assert(paths.includes(receipt.runtime.loader.path) && paths.includes(receipt.runtime.libc.path));
  exact(receipt.runtime.versions, ['dpkgDeb', 'libc', 'loader'], 'actual versions');
  for (const value of Object.values(receipt.runtime.versions)) assert(typeof value === 'string' &&
    value.length > 0 && Buffer.byteLength(value) <= 16384);
  exact(receipt.provenance, ['packageAcquisition', 'hostPackages', 'historicalAnchor'], 'provenance');
  assert.equal(receipt.provenance.packageAcquisition, 'fixed-url-exact-hash-this-host-no-fresh-gpg');
  assert.equal(receipt.provenance.historicalAnchor, 'historical-only-a719cdffdae1f744c92b2d93f65103a1e4cfc3429d2fc55cdb85fe3ffbf8470a');
  assert(typeof receipt.provenance.hostPackages === 'string' && receipt.provenance.hostPackages.length > 0 &&
    Buffer.byteLength(receipt.provenance.hostPackages) <= 16384);
  assert.equal(receipt.sourceAuthenticated, false); assert.equal(receipt.performanceQualified, false);
  assert.equal(receipt.baselineAuthority, null); assert.equal(receipt.thresholds, null);
  return receipt;
}
function trustedAncestors(path, root) {
  let current = dirname(path);
  while (true) {
    const stat = lstatSync(current);
    assert(stat.isDirectory() && !stat.isSymbolicLink(), 'real provider ancestors');
    assert(stat.uid === 0 || stat.uid === process.getuid(), 'trusted ancestor owner');
    if (current === '/tmp') assert(stat.uid === 0 && (stat.mode & 0o1000), 'trusted sticky /tmp');
    else assert.equal(stat.mode & 0o022, 0, 'trusted provider ancestors');
    if (current === root) assert(stat.uid === process.getuid() && (stat.mode & 0o777) === 0o700);
    if (current === '/') break;
    current = dirname(current);
  }
}
export function snapshotProviderFile(path, limit = 32 * 1024 * 1024) {
  assert.equal(realpathSync(path), path, 'canonical provider file, no symlink aliases');
  const before = fileIdentity(lstatSync(path));
  const bytes = readSafe('/', path.slice(1), limit);
  assert.deepEqual(fileIdentity(lstatSync(path)), before, 'stable provider file identity');
  const pin = { path, bytes: bytes.length, sha256: sha256(bytes), identity: before };
  validateObservedFile(pin);
  return { pin, bytes };
}
// Explicit root chooses a fresh instance of this one recipe, never an executable, hash, or grant.
export function openLinuxTimeProvider({ providerRoot: root }) {
  providerRoot(root);
  trustedAncestors(join(root, 'extracted/usr/bin/time'), root);
  const rootIdentity = fileIdentity(lstatSync(root));
  const admission = snapshotProviderFile(RECORD, 16384);
  validateProviderRecord(admission.bytes);
  const observed = snapshotProviderFile(join(root, RECEIPT_NAME), 65536);
  assert.equal(observed.pin.identity.uid, process.getuid());
  assert.equal(observed.pin.identity.mode & 0o7777, 0o600);
  const receipt = validateRuntimeReceipt(observed.bytes, root);
  for (const pin of [receipt.package, receipt.tool.file]) assert.equal(pin.identity.uid, process.getuid());
  validateUbuntuRelease(snapshotProviderFile(receipt.host.osRelease.path).bytes);
  const pins = [admission.pin, observed.pin, receipt.host.osRelease, receipt.package, receipt.tool.file,
    receipt.acquisition.curl, receipt.acquisition.receipt,
    receipt.runtime.dpkgDeb, receipt.runtime.dpkgQuery, receipt.runtime.loader, receipt.runtime.libc,
    ...receipt.runtime.sharedProviders];
  const check = () => {
    assert.deepEqual(fileIdentity(lstatSync(root)), rootIdentity, 'same fresh private provider root');
    for (const pin of pins) {
      trustedAncestors(pin.path, root);
      assert.deepEqual(snapshotProviderFile(pin.path).pin, pin, 'provider unchanged since fresh admission');
    }
  };
  check();
  return Object.freeze({ check, admissionBytes: Buffer.from(admission.bytes),
    receiptBytes: Buffer.from(observed.bytes), binding: Object.freeze({ providerRoot: root,
      providerRecordSha256: PROVIDER_SHA256, providerReceiptSha256: sha256(observed.bytes),
      executable: timeExecutable(root), authority: 'binding-only-no-source-or-baseline-authority' }) });
}
