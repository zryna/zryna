import { orderedPaths, requireValue, sha256 } from './canonical.mjs';
import { captureTarGzipMembers } from './npm-materials.mjs';
import { rustMaterials } from './rust-materials.mjs';

const MAX_CRATE = 64 * 1024 * 1024;
const UPSTREAM_WASMTIME =
  'https://raw.githubusercontent.com/bytecodealliance/wasmtime/7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';

export function captureRustMaterials(target, captures, upstreamLicense) {
  const records = rustMaterials(target);
  requireValue(Array.isArray(captures) && captures.length === records.length
    && Buffer.isBuffer(upstreamLicense), 'Rust material capture set');
  const files = [];
  for (let index = 0; index < records.length; index++) {
    const record = records[index];
    const identity = `${record.name}-${record.version}`;
    const capture = captures[index];
    requireValue(capture?.identity === identity && Buffer.isBuffer(capture.archive)
      && capture.archive.length >= 1 && capture.archive.length <= MAX_CRATE
      && sha256(capture.archive) === record.crateSha256, 'Rust crate identity');
    const selected = record.files.filter(file => file.origin !== UPSTREAM_WASMTIME);
    for (const file of selected) {
      requireValue(file.origin
        === `https://static.crates.io/crates/${record.name}/${identity}.crate`,
      'Rust crate origin');
    }
    if (selected.length > 0) {
      files.push(...captureTarGzipMembers(capture.archive, {
        root: identity, size: capture.archive.length, sha256: record.crateSha256,
        files: selected.map(({ sourcePath, path, size, sha256: digest }) => ({
          sourcePath, path, mode: 0o644, size, sha256: digest,
        })),
      }));
    }
    for (const expected of record.files.filter(file => file.origin === UPSTREAM_WASMTIME)) {
      requireValue(upstreamLicense.length === expected.size
        && sha256(upstreamLicense) === expected.sha256, 'Rust upstream license identity');
      files.push({ path: expected.path, mode: 0o644, data: Buffer.from(upstreamLicense) });
    }
  }
  files.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  orderedPaths(files.map(file => file.path));
  return files;
}
