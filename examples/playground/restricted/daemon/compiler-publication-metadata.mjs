// Pure metadata relationships; none of these functions mint publication or execution authority.
import { parseCanonical } from '../../../../scripts/distribution/canonical.mjs';
import { LIMITS, bytes, equal, exactKeys, hash,
  positive, product, productVersion, requireValue, target, validateSource } from './compiler-publication-constants.mjs';

export function compilerBuildReceipt(document) {
  const { provenance } = document;
  return { format: 'zryna.playground-compiler-build.v1', version: 1, source: document.source,
    product, productVersion, target, binary: document.binary,
    workflow: provenance.workflow, toolchain: provenance.toolchain, inputs: provenance.inputs,
    build: provenance.build, requiredCi: provenance.requiredCi, reproduction: provenance.reproduction };
}

export function validateCompilerBuildReceipt(input, document) {
  const expected = bytes(compilerBuildReceipt(document));
  requireValue(Buffer.isBuffer(input) && input.equals(expected), 'compiler publication actual build receipt');
  return parseCanonical(input);
}

export function validateCompilerRustMaterials(input, document) {
  requireValue(Buffer.isBuffer(input) && input.length > 0 && input.length <= LIMITS.text,
    'compiler publication Rust material byte bound');
  // The established record parser's 256KiB limit is stricter than the outer metadata ceiling.
  const materials = parseCanonical(input);
  exactKeys(materials, ['format', 'version', 'source', 'target', 'cargoLockSha256', 'packages']);
  validateSource(materials.source);
  requireValue(materials.format === 'zryna.playground-compiler-rust-materials.v1' && materials.version === 1 &&
    equal(materials.source, document.source) && materials.target === target &&
    materials.cargoLockSha256 === document.provenance.inputs.cargoLock.sha256 &&
    Array.isArray(materials.packages) && materials.packages.length > 0 && materials.packages.length <= 256,
  'compiler publication approved Rust material identity');
  let previous = '';
  const licensePaths = new Set();
  for (const entry of materials.packages) {
    exactKeys(entry, ['name', 'version', 'crateSha256', 'licenses']);
    requireValue(typeof entry.name === 'string' && /^[a-z0-9][a-z0-9_-]{0,63}$/.test(entry.name) &&
      typeof entry.version === 'string' && entry.version.length <= 96 &&
      /^[0-9]+\.[0-9]+\.[0-9]+(?:[A-Za-z0-9.+-]*)$/.test(entry.version) && hash(entry.crateSha256),
    'compiler publication crate identity');
    const identity = `${entry.name}-${entry.version}`;
    const key = `${entry.name}\0${entry.version}`;
    requireValue(key > previous && Array.isArray(entry.licenses) && entry.licenses.length > 0 &&
      entry.licenses.length <= 16, 'compiler publication crate closure order');
    previous = key;
    let previousLicense = '';
    for (const license of entry.licenses) {
      exactKeys(license, ['path', 'bytes', 'sha256']);
      requireValue(typeof license.path === 'string' &&
        license.path.startsWith(`licenses/rust/${identity}/`) && license.path > previousLicense &&
        !licensePaths.has(license.path) && positive(license.bytes, LIMITS.text) && hash(license.sha256),
      'compiler publication crate license identity');
      previousLicense = license.path; licensePaths.add(license.path);
    }
  }
  return materials;
}

export function validateCompilerElf(binary) {
  requireValue(Buffer.isBuffer(binary) && binary.length >= 64 &&
    binary.subarray(0, 7).equals(Buffer.from([0x7f, 69, 76, 70, 2, 1, 1])) &&
    [2, 3].includes(binary.readUInt16LE(16)) && binary.readUInt16LE(18) === 62,
  'compiler publication ELF target');
  const offset = Number(binary.readBigUInt64LE(32));
  const width = binary.readUInt16LE(54), count = binary.readUInt16LE(56);
  requireValue(Number.isSafeInteger(offset) && offset >= 64 && width === 56 && count > 0 && count <= 256 &&
    offset + width * count <= binary.length, 'compiler publication ELF program inventory');
  let executable = false;
  for (let index = 0; index < count; index++) {
    const position = offset + index * width;
    const type = binary.readUInt32LE(position), flags = binary.readUInt32LE(position + 4);
    const fileOffset = Number(binary.readBigUInt64LE(position + 8));
    const size = Number(binary.readBigUInt64LE(position + 32));
    requireValue(Number.isSafeInteger(fileOffset) && Number.isSafeInteger(size) &&
      fileOffset >= 0 && size >= 0 && fileOffset <= binary.length - size,
    'compiler publication ELF segment bounds');
    if (type === 1) {
      requireValue((flags & 3) !== 3, 'compiler publication writable executable segment');
      executable ||= (flags & 1) !== 0;
    }
    if (type === 0x6474e551) requireValue((flags & 1) === 0, 'compiler publication executable stack');
  }
  requireValue(executable, 'compiler publication executable load segment');
}
