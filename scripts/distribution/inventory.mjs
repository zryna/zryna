import { LIMITS, bytes, exactKeys, orderedPaths, requireValue, sha256 } from './canonical.mjs';

export const PROVIDERS = Object.freeze([
  'limits-v3.mjs', 'limits-v4.mjs',
  'node_modules/@typescript/old/lib/typescript.js',
  'node_modules/@typescript/old/package.json',
  'node_modules/@typescript/typescript6/lib/typescript.js',
  'node_modules/@typescript/typescript6/package.json',
  'worker-v3.mjs', 'worker-v4.mjs', 'worker.mjs',
].map(path => `lib/zryna/bootstrap/${path}`));

export const METADATA = Object.freeze([
  'metadata/architecture-receipt.json', 'metadata/checksums.sha256',
  'metadata/distribution.json', 'metadata/inventory.json', 'metadata/materials.json',
]);

const TEXT = Object.freeze([
  'LICENSE', 'NOTICE', 'README.md', 'SUPPORT.md', 'VERSION',
  'licenses/node-LICENSE', 'licenses/typescript-LICENSE.txt',
  'licenses/typescript-ThirdPartyNoticeText.txt', 'licenses/typescript6-LICENSE.txt',
]);

export function targetPaths(target) {
  requireValue(['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'].includes(target), 'target');
  return target === 'x86_64-unknown-linux-gnu'
    ? { cli: 'bin/zryna', node: 'runtime/node/bin/node', format: 'tar-gzip', extension: 'tar.gz' }
    : { cli: 'bin/zryna.exe', node: 'runtime/node/node.exe', format: 'zip', extension: 'zip' };
}

export function digestValue(value) {
  requireValue(typeof value === 'string' && /^[0-9a-f]{64}$/.test(value), 'SHA-256 encoding');
}

export function roleFor(path, target) {
  const paths = targetPaths(target);
  if (path === paths.cli) return 'cli';
  if (path === paths.node) return 'runtime';
  if (PROVIDERS.includes(path)) return 'provider';
  if (METADATA.includes(path)) return 'metadata';
  if (TEXT.includes(path)) return path.startsWith('licenses/') || path === 'LICENSE' ? 'license' : 'notice';
  if (/^licenses\/rust\/[a-z0-9][a-z0-9_-]*-[0-9]+\.[0-9]+\.[0-9]+(?:[A-Za-z0-9.+-]*)\/[A-Za-z0-9][A-Za-z0-9._-]*$/.test(path)) {
    return 'license';
  }
  requireValue(false, 'path has no distribution role');
}

export function validateTuples(entries, target, { complete = false } = {}) {
  orderedPaths(entries.map(entry => entry.path));
  let total = 0;
  for (const entry of entries) {
    exactKeys(entry, ['path', 'size', 'sha256', 'role', 'mode', 'material', 'licenses']);
    const role = roleFor(entry.path, target);
    requireValue(entry.role === role, 'file role mismatch');
    const executable = role === 'cli' || role === 'runtime';
    requireValue(entry.mode === (executable ? 0o755 : 0o644), 'file mode mismatch');
    const limit = executable ? LIMITS.binary : role === 'provider' ? LIMITS.provider : LIMITS.text;
    requireValue(Number.isSafeInteger(entry.size) && entry.size >= 1 && entry.size <= limit,
      'role byte budget');
    total += entry.size;
    requireValue(total <= LIMITS.expanded, 'inventory expansion budget');
    digestValue(entry.sha256);
    requireValue(typeof entry.material === 'string'
      && /^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$/.test(entry.material), 'origin material reference');
    requireValue(Array.isArray(entry.licenses) && entry.licenses.length >= 1
      && entry.licenses.length <= LIMITS.files, 'license reference count');
    orderedPaths(entry.licenses);
  }
  const paths = new Set(entries.map(entry => entry.path));
  for (const entry of entries) {
    for (const license of entry.licenses) {
      requireValue(paths.has(license) && roleFor(license, target) === 'license', 'missing license reference');
    }
  }
  if (complete) {
    for (const path of [...PROVIDERS, ...TEXT, targetPaths(target).node,
      'metadata/materials.json', 'metadata/architecture-receipt.json']) {
      requireValue(paths.has(path), 'missing required payload');
    }
    requireValue(entries.some(entry => entry.path.startsWith('licenses/rust/')), 'missing Rust notices');
  }
}

export function authenticateFiles(files, entries, target) {
  validateTuples(entries, target);
  orderedPaths(files.map(file => file.path));
  requireValue(files.length === entries.length, 'file inventory count mismatch');
  for (let index = 0; index < entries.length; index++) {
    const file = files[index];
    const expected = entries[index];
    requireValue(file.path === expected.path && file.mode === expected.mode
      && Buffer.isBuffer(file.data) && file.data.length === expected.size
      && sha256(file.data) === expected.sha256, 'file inventory digest mismatch');
  }
}

export function inventoryBytes(entries, target) {
  validateTuples(entries, target);
  const record = bytes({ format: 'zryna.distribution-inventory.v1', files: entries });
  requireValue(record.length <= LIMITS.record, 'inventory record byte budget');
  return record;
}

export function checksumBytes(files) {
  orderedPaths(files.map(file => file.path));
  return Buffer.from(files.map(file => `${sha256(file.data)}  ${file.path}\n`).join(''), 'ascii');
}
