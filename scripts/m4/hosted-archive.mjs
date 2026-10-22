import { inflateRawSync } from 'node:zlib';
import { join } from 'node:path';
import { digest, inventory, portablePath, readStableFile } from './registry-files.mjs';

// One retained lane: original 256 MiB material bounds plus bounded complete gate logs.
const MAX_ARCHIVE = 768 * 1024 * 1024;
const MAX_EXPANDED = 768 * 1024 * 1024;
const MAX_ENTRY = 256 * 1024 * 1024;
const fail = message => { throw new Error(`M4 hosted archive: ${message}`); };
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const value of bytes) crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ value) & 255];
  return (crc ^ 0xffffffff) >>> 0;
}
const CRC_TABLE = Uint32Array.from({ length: 256 }, (_, index) => {
  let crc = index;
  for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  return crc;
});
function range(bytes, offset, size) {
  if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(size) || offset < 0
      || size < 0 || offset + size > bytes.length) fail('truncated ZIP');
  return bytes.subarray(offset, offset + size);
}
function name(bytes) {
  const value = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  portablePath(value);
  return value;
}

// The ZIP is externally SHA-pinned. Its retained contents are checked against this same archive,
// rather than trusting an independently mutable extraction directory.
export function verifyRetainedArchive(archivePath, directory, expected) {
  const bytes = readStableFile(archivePath, MAX_ARCHIVE);
  if (bytes.length !== expected.bytes || digest(bytes) !== expected.sha256) fail('archive identity differs');
  const entries = zipEntries(bytes);
  const actual = inventory(directory, 'evidence');
  const wanted = entries.map(entry => `evidence/${entry.path}`).sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) fail('retained inventory differs');
  for (const entry of entries) {
    const retained = readStableFile(join(directory, 'evidence', entry.path), MAX_ENTRY);
    if (retained.length !== entry.size || digest(retained) !== entry.sha256) fail('retained bytes differ');
  }
  return entries.map(entry => entry.path);
}

export function zipEntries(bytes, consume = undefined) {
  if (!Buffer.isBuffer(bytes) || bytes.length > MAX_ARCHIVE || bytes.length < 22) fail('invalid archive');
  // Artifact producer requests compression-level: 0 and emits ordinary bounded ZIP32 records.
  const end = bytes.length - 22;
  if (bytes.readUInt32LE(end) !== 0x06054b50 || bytes.readUInt16LE(end + 20) !== 0
      || bytes.readUInt16LE(end + 4) !== 0 || bytes.readUInt16LE(end + 6) !== 0) fail('unsupported ZIP end');
  const count = bytes.readUInt16LE(end + 10);
  if (!count || count > 256 || count !== bytes.readUInt16LE(end + 8)) fail('invalid ZIP count');
  const centralSize = bytes.readUInt32LE(end + 12);
  const centralStart = bytes.readUInt32LE(end + 16);
  if (centralStart + centralSize !== end) fail('central directory layout differs');
  let cursor = centralStart;
  let nextLocal = 0;
  let expanded = 0;
  const entries = [];
  const names = new Set();
  for (let index = 0; index < count; index++) {
    const header = range(bytes, cursor, 46);
    if (header.readUInt32LE(0) !== 0x02014b50) fail('missing central header');
    const flags = header.readUInt16LE(8);
    const method = header.readUInt16LE(10);
    if ((flags & ~0x808) || ![0, 8].includes(method)) fail('unsupported flags or compression');
    const checksum = header.readUInt32LE(16);
    const compressedSize = header.readUInt32LE(20);
    const size = header.readUInt32LE(24);
    const nameSize = header.readUInt16LE(28);
    const extraSize = header.readUInt16LE(30);
    const commentSize = header.readUInt16LE(32);
    const attributes = header.readUInt32LE(38);
    const offset = header.readUInt32LE(42);
    const fileType = (attributes >>> 16) & 0xf000;
    if (header.readUInt16LE(34) || ![0, 0x8000].includes(fileType) || (attributes & 0x10)
        || size === 0xffffffff || compressedSize === 0xffffffff || offset !== nextLocal) fail('invalid entry identity');
    const path = name(range(bytes, cursor + 46, nameSize));
    if (names.has(path.toLowerCase())) fail('duplicate or case-colliding entry');
    names.add(path.toLowerCase());
    cursor += 46 + nameSize + extraSize + commentSize;
    if (cursor > end || extraSize || commentSize) fail('unsupported ZIP entry metadata');
    const local = range(bytes, offset, 30);
    if (local.readUInt32LE(0) !== 0x04034b50 || local.readUInt16LE(6) !== flags
        || local.readUInt16LE(8) !== method || local.readUInt16LE(26) !== nameSize
        || local.readUInt16LE(28) !== 0
        || name(range(bytes, offset + 30, nameSize)) !== path) fail('local/central identities differ');
    expanded += size;
    if (expanded > MAX_EXPANDED || size > MAX_ENTRY) fail('expanded byte bound exceeded');
    const start = offset + 30 + nameSize;
    const compressed = range(bytes, start, compressedSize);
    const inflated = method === 0 ? null : inflateRawSync(compressed, { maxOutputLength: size + 1, info: true });
    if (inflated && inflated.engine.bytesWritten !== compressedSize) fail('unaccounted compressed bytes');
    const content = inflated ? inflated.buffer : compressed;
    if (content.length !== size || crc32(content) !== checksum) fail('size or CRC mismatch');
    nextLocal = start + compressedSize;
    if (flags & 8) {
      const descriptor = range(bytes, nextLocal, 16);
      if (descriptor.readUInt32LE(0) !== 0x08074b50 || descriptor.readUInt32LE(4) !== checksum
          || descriptor.readUInt32LE(8) !== compressedSize || descriptor.readUInt32LE(12) !== size) fail('invalid descriptor');
      nextLocal += 16;
    } else if (local.readUInt32LE(14) !== checksum || local.readUInt32LE(18) !== compressedSize
        || local.readUInt32LE(22) !== size) fail('local size differs');
    if (nextLocal > centralStart) fail('entry overlaps central directory');
    const entry = { path, size, sha256: digest(content) };
    if (consume) consume(entry, content);
    entries.push(entry);
  }
  if (cursor !== end || nextLocal !== centralStart) fail('unaccounted ZIP bytes');
  return entries;
}
