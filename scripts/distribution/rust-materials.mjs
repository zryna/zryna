import { readFileSync } from 'node:fs';
import { bytes, parseCanonical, requireValue, sha256 } from './canonical.mjs';

const LOCK = readFileSync(new URL('./materials-rust-v1.json', import.meta.url));
requireValue(sha256(LOCK) === 'bf0e12868f54bccdd47030fcc6fd24bce7b7dca7e0033dbfbf581c9c1a24e722',
  'Rust material recipe identity');
const RECORD = parseCanonical(LOCK);

export function rustMaterials(target) {
  requireValue(Object.hasOwn(RECORD.targets, target), 'Rust material target');
  const selected = new Set(RECORD.targets[target]);
  return RECORD.packages.filter(record => selected.has(`${record.name}-${record.version}`));
}

export function validateRustMaterials(entries, target, architectureReceipt) {
  const lock = architectureReceipt.inputs.find(input => input.logicalPath === 'Cargo.lock');
  requireValue(lock?.sha256 === RECORD.cargoLockSha256, 'Rust material lockfile identity');
  const expected = rustMaterials(target).flatMap(record => record.files)
    .map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest }))
    .sort((a, b) => a.path < b.path ? -1 : a.path > b.path ? 1 : 0);
  const actual = entries.filter(entry => entry.path.startsWith('licenses/rust/'))
    .map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest }));
  requireValue(bytes(actual).equals(bytes(expected)), 'Rust notice inventory differs from approved recipe');
}
