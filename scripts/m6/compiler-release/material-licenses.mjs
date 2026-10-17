import { requireValue, sha256 } from '../../distribution/canonical.mjs';
import { captureTarGzipMembers } from '../../distribution/npm-materials.mjs';

const UPSTREAM = 'https://raw.githubusercontent.com/bytecodealliance/wasmtime/'
  + '7bac2c2775808aaec5d4aa5627a5e447b51102cf/LICENSE';

// Descriptor-only helper; production callers select its records from independently pinned tables.
export function captureCompilerLicenses(pin, archive, upstreamLicense) {
  const identity = `${pin.name}-${pin.version}`;
  requireValue(Array.isArray(pin.files) && pin.files.length > 0 && pin.files.length <= 16,
    'compiler nonempty bounded crate licenses');
  const files = [];
  const local = pin.files.filter(file => file.origin !== UPSTREAM);
  requireValue(local.every(file => file.origin
    === `https://static.crates.io/crates/${pin.name}/${identity}.crate`), 'compiler material official origin');
  if (local.length) files.push(...captureTarGzipMembers(archive, {
    root: identity, size: archive.length, sha256: pin.crateSha256,
    files: local.map(file => ({ sourcePath: file.sourcePath, path: file.path, mode: 0o644,
      size: file.size, sha256: file.sha256 })),
  }));
  for (const license of pin.files.filter(file => file.origin === UPSTREAM)) {
    requireValue(Buffer.isBuffer(upstreamLicense) && upstreamLicense.length === license.size
      && sha256(upstreamLicense) === license.sha256, 'compiler approved upstream license');
    files.push({ path: license.path, mode: 0o644, data: Buffer.from(upstreamLicense) });
  }
  return files;
}
