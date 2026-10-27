// Separate M7 ZIP32 profile. Existing M4 archive schemas and limits are unchanged.
import assert from 'node:assert/strict';
import { inflateRawSync } from 'node:zlib';
import { portable, sha256 } from './input.mjs';

const MiB = 1024 * 1024;
const profiles = Object.freeze({
  primary: Object.freeze({ archive: 128 * MiB, expanded: 128 * MiB, entry: 64 * MiB, entries: 4096 }),
  'linux-provider': Object.freeze({ archive: MiB, expanded: MiB, entry: 256 * 1024, entries: 256 }),
});
const opened = new WeakMap();
const fail = message => { throw new Error(`S418 performance archive: ${message}`); };
function privateCopy(bytes) {
  const copy = Buffer.alloc(bytes.length); // Never share Node's exposed small-buffer pool.
  bytes.copy(copy); return copy;
}
const table = Uint32Array.from({ length: 256 }, (_, index) => {
  let value = index;
  for (let bit = 0; bit < 8; bit++) value = (value >>> 1) ^ ((value & 1) ? 0xedb88320 : 0);
  return value;
});
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const byte of bytes) crc = (crc >>> 8) ^ table[(crc ^ byte) & 255];
  return (crc ^ 0xffffffff) >>> 0;
}
function range(bytes, offset, size) {
  if (!Number.isSafeInteger(offset) || !Number.isSafeInteger(size) || offset < 0 || size < 0
    || offset + size > bytes.length) fail('truncated ZIP');
  return bytes.subarray(offset, offset + size);
}
function path(bytes) {
  const value = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  assert(Buffer.from(value).equals(bytes), 'S418 performance archive: canonical UTF-8 name');
  return portable(value);
}
function scan(bytes, limits) {
  const end = bytes.length - 22;
  if (bytes.readUInt32LE(end) !== 0x06054b50 || bytes.readUInt16LE(end + 20)
    || bytes.readUInt16LE(end + 4) || bytes.readUInt16LE(end + 6)) fail('unsupported ZIP end');
  const count = bytes.readUInt16LE(end + 10);
  if (!count || count > limits.entries || count !== bytes.readUInt16LE(end + 8)) fail('entry count');
  const centralStart = bytes.readUInt32LE(end + 16), centralSize = bytes.readUInt32LE(end + 12);
  if (centralStart + centralSize !== end) fail('central directory layout');
  let cursor = centralStart, nextLocal = 0, expanded = 0;
  const entries = new Map(), names = new Set();
  for (let index = 0; index < count; index++) {
    const header = range(bytes, cursor, 46);
    if (header.readUInt32LE(0) !== 0x02014b50) fail('missing central header');
    const flags = header.readUInt16LE(8), method = header.readUInt16LE(10);
    if ((flags & ~0x808) || ![0, 8].includes(method)) fail('flags or compression');
    const checksum = header.readUInt32LE(16), compressedSize = header.readUInt32LE(20), size = header.readUInt32LE(24);
    const nameSize = header.readUInt16LE(28), extraSize = header.readUInt16LE(30), commentSize = header.readUInt16LE(32);
    const attributes = header.readUInt32LE(38), offset = header.readUInt32LE(42), type = (attributes >>> 16) & 0xf000;
    if (header.readUInt16LE(34) || ![0, 0x8000].includes(type) || (attributes & 0x10)
      || size === 0xffffffff || compressedSize === 0xffffffff || offset !== nextLocal) fail('entry identity');
    const name = path(range(bytes, cursor + 46, nameSize));
    if (names.has(name.toLowerCase())) fail('duplicate or case-colliding path');
    names.add(name.toLowerCase());
    cursor += 46 + nameSize + extraSize + commentSize;
    if (cursor > end || extraSize || commentSize) fail('unsupported entry metadata');
    const local = range(bytes, offset, 30);
    if (local.readUInt32LE(0) !== 0x04034b50 || local.readUInt16LE(6) !== flags
      || local.readUInt16LE(8) !== method || local.readUInt16LE(26) !== nameSize || local.readUInt16LE(28)
      || path(range(bytes, offset + 30, nameSize)) !== name) fail('local/central mismatch');
    expanded += size;
    if (expanded > limits.expanded || size > limits.entry) fail('expanded byte bound');
    const start = offset + 30 + nameSize, compressed = range(bytes, start, compressedSize);
    const inflated = method === 0 ? null : inflateRawSync(compressed, { maxOutputLength: size + 1, info: true });
    if (inflated && inflated.engine.bytesWritten !== compressedSize) fail('unaccounted compressed bytes');
    const content = inflated ? inflated.buffer : compressed;
    if (content.length !== size || crc32(content) !== checksum) fail('size or CRC mismatch');
    nextLocal = start + compressedSize;
    if (flags & 8) {
      const descriptor = range(bytes, nextLocal, 16);
      if (descriptor.readUInt32LE(0) !== 0x08074b50 || descriptor.readUInt32LE(4) !== checksum
        || descriptor.readUInt32LE(8) !== compressedSize || descriptor.readUInt32LE(12) !== size) fail('invalid descriptor');
      for (const [at, value] of [[14, checksum], [18, compressedSize], [22, size]])
        if (![0, value].includes(local.readUInt32LE(at))) fail('local descriptor fields disagree');
      nextLocal += 16;
    } else if (local.readUInt32LE(14) !== checksum || local.readUInt32LE(18) !== compressedSize
      || local.readUInt32LE(22) !== size) fail('local size or CRC mismatch');
    if (nextLocal > centralStart) fail('entry overlaps central directory');
    entries.set(name, { metadata: { path: name, bytes: size, sha256: sha256(content) },
      content: inflated ? privateCopy(content) : content });
  }
  if (cursor !== end || nextLocal !== centralStart) fail('unaccounted ZIP bytes');
  return { entries, expanded };
}
function selection(key) {
  const value = opened.get(key); assert(value, 'S418 performance archive: live validated selection required'); return value;
}
export function openPerformanceArchive(bytes, expected, role = 'primary') {
  assert(Object.hasOwn(profiles, role), 'S418 performance archive: closed archive role');
  const limits = profiles[role];
  assert(Buffer.isBuffer(bytes) && bytes.length >= 22 && bytes.length <= limits.archive);
  const expectedBytes = expected?.bytes, digest = expected?.sha256;
  assert(Number.isSafeInteger(expectedBytes) && expectedBytes === bytes.length);
  assert(typeof digest === 'string' && /^[a-f0-9]{64}$/u.test(digest));
  const image = privateCopy(bytes); // Caller mutations cannot rewrite a validated entry.
  assert.equal(sha256(image), digest, 'S418 performance archive: reviewed archive digest');
  const { entries, expanded } = scan(image, limits);
  const key = Object.freeze({});
  opened.set(key, { entries, expanded, role, bytes: image.length, sha256: digest });
  return key;
}
export function inspectPerformanceArchive(key) {
  const value = selection(key);
  return { format: 'zryna.cli-performance-archive.v1', role: value.role, bytes: value.bytes, sha256: value.sha256,
    entries: value.entries.size, expandedBytes: value.expanded, archiveIntegrityVerified: true,
    sourceAuthenticated: false, performanceQualified: false, thresholds: null };
}
export function listPerformanceArchive(key) {
  return [...selection(key).entries.values()].map(row => ({ ...row.metadata }));
}
export function readPerformanceArchiveEntry(key, name) {
  portable(name);
  const row = selection(key).entries.get(name); assert(row, 'S418 performance archive: exact entry required');
  return privateCopy(row.content);
}
export function closePerformanceArchive(key) {
  selection(key); opened.delete(key);
}
