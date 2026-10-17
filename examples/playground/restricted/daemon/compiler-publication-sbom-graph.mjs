import { createHash } from 'node:crypto';
import {
  bytes, canonical, exactKeys, orderedPaths, portablePath, sha256,
} from '../../../../scripts/distribution/canonical.mjs';

export const PRODUCT = 'zryna-playground-compiler';
export const VERSION = '0.1.0';
export const TARGET = 'x86_64-unknown-linux-gnu';
export const SPDX_PATH = 'metadata/compiler.spdx.json';
export const SOURCE_KEYS = [
  'repository', 'ref', 'commit', 'tree', 'sourceDateEpoch', 'tagObject', 'tagType',
];

export function requireSpdx(condition, message) {
  if (!condition) throw new Error(`PLAYGROUND-COMPILER-SPDX: ${message}`);
}

export function sha1(data) {
  return createHash('sha1').update(data).digest('hex');
}

function digest(value) {
  return typeof value === 'string' && /^[0-9a-f]{64}$/.test(value);
}

function sourceIdentity(source) {
  exactKeys(source, SOURCE_KEYS);
  requireSpdx(source.repository === 'https://github.com/zryna/zryna'
    && source.ref === 'refs/tags/v0.1.0-playground-compiler.1'
    && source.tagType === 'annotated'
    && ['commit', 'tree', 'tagObject'].every(key => /^[0-9a-f]{40}$/.test(source[key]))
    && source.commit !== source.tagObject
    && Number.isSafeInteger(source.sourceDateEpoch)
    && source.sourceDateEpoch >= 0 && source.sourceDateEpoch <= 0xffffffff,
  'source identity differs');
}

function fileGraph(files, envelope) {
  requireSpdx(Array.isArray(files) && files.length > 0 && files.length <= 512,
    'file count budget');
  const sorted = [...files].sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  orderedPaths(sorted.map(file => file.path));
  let expanded = 0;
  const graph = sorted.map(file => {
    exactKeys(file, ['path', 'data', 'mode']);
    requireSpdx(Buffer.isBuffer(file.data) && file.data.length > 0
      && file.data.length <= 256 * 1024 * 1024, 'file byte budget');
    expanded += file.data.length;
    const binary = file.path === 'bin/zryna-playground-compiler';
    requireSpdx(file.mode === (binary ? 0o755 : 0o644), 'file mode differs');
    requireSpdx(binary || ['LICENSE', 'NOTICE', 'README.md',
      'metadata/build-receipt.json', 'metadata/rust-materials.json', SPDX_PATH].includes(file.path)
      || file.path.startsWith('licenses/'), 'unexpected compiler archive path');
    requireSpdx(binary || file.data.length <= 2 * 1024 * 1024, 'text file byte budget');
    return { path: file.path, data: file.data, mode: file.mode,
      size: file.data.length, sha1: sha1(file.data), sha256: sha256(file.data) };
  }).filter(file => file.path !== SPDX_PATH);
  requireSpdx(expanded <= 512 * 1024 * 1024, 'expanded file byte budget');
  const byPath = new Map(graph.map(file => [file.path, file]));
  for (const path of ['LICENSE', 'NOTICE', 'README.md', 'metadata/build-receipt.json',
    'metadata/rust-materials.json', 'bin/zryna-playground-compiler']) {
    requireSpdx(byPath.has(path), `required file missing: ${path}`);
  }
  exactKeys(envelope.binary, ['path', 'bytes', 'sha256']);
  const binary = byPath.get(envelope.binary.path);
  requireSpdx(envelope.binary.path === 'bin/zryna-playground-compiler'
    && Number.isSafeInteger(envelope.binary.bytes) && envelope.binary.bytes > 0
    && digest(envelope.binary.sha256)
    && binary?.size === envelope.binary.bytes && binary.sha256 === envelope.binary.sha256,
  'binary identity differs');
  return { graph, byPath };
}

function materialGraph(materials, envelope, byPath) {
  exactKeys(materials, ['format', 'version', 'source', 'target', 'cargoLockSha256', 'packages']);
  sourceIdentity(materials.source);
  requireSpdx(materials.format === 'zryna.playground-compiler-rust-materials.v1'
    && materials.version === 1 && materials.target === TARGET
    && canonical(materials.source) === canonical(envelope.source)
    && digest(materials.cargoLockSha256), 'materials identity differs');
  requireSpdx(Array.isArray(materials.packages)
    && materials.packages.length > 0 && materials.packages.length <= 256,
  'material package count budget');
  const licenseOwners = new Map();
  let previous = '';
  for (const pkg of materials.packages) {
    exactKeys(pkg, ['name', 'version', 'crateSha256', 'licenses']);
    requireSpdx(typeof pkg.name === 'string' && pkg.name.length <= 128
      && /^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(pkg.name)
      && typeof pkg.version === 'string' && pkg.version.length <= 128
      && /^[0-9][A-Za-z0-9+._-]*$/.test(pkg.version) && digest(pkg.crateSha256),
    'material package identity differs');
    const key = `${pkg.name}\0${pkg.version}`;
    requireSpdx(key > previous, 'unordered or duplicate material packages');
    previous = key;
    requireSpdx(Array.isArray(pkg.licenses) && pkg.licenses.length > 0
      && pkg.licenses.length <= 512, 'material license count budget');
    orderedPaths(pkg.licenses.map(license => license.path));
    for (const license of pkg.licenses) {
      exactKeys(license, ['path', 'bytes', 'sha256']);
      portablePath(license.path);
      const actual = byPath.get(license.path);
      requireSpdx(license.path.startsWith(`licenses/rust/${pkg.name}-${pkg.version}/`)
        && Number.isSafeInteger(license.bytes) && license.bytes > 0 && digest(license.sha256)
        && actual?.size === license.bytes && actual.sha256 === license.sha256
        && !licenseOwners.has(license.path), 'material license bytes or ownership differ');
      licenseOwners.set(license.path, key);
    }
  }
  requireSpdx([...byPath.keys()].filter(path => path.startsWith('licenses/rust/'))
    .every(path => licenseOwners.has(path)), 'unaccounted Rust license file');
  requireSpdx(byPath.get('metadata/rust-materials.json').data.equals(bytes(materials)),
    'materials bytes differ from archive');
  return licenseOwners;
}

export function compilerGraph({ files, envelope, materials }) {
  requireSpdx(envelope?.product === PRODUCT && envelope.productVersion === VERSION
    && envelope.target === TARGET, 'compiler product identity differs');
  sourceIdentity(envelope.source);
  const { graph, byPath } = fileGraph(files, envelope);
  const licenseOwners = materialGraph(materials, envelope, byPath);
  return { graph, licenseOwners };
}
