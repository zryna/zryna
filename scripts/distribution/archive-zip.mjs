import { LIMITS, requireValue } from './canonical.mjs';
import { archiveEntries, decodedFiles } from './archive-entries.mjs';

const CRC_TABLE = Array.from({ length: 256 }, (_, value) => {
  let crc = value;
  for (let bit = 0; bit < 8; bit++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  return crc >>> 0;
});

export function crc32(data) {
  let crc = 0xffffffff;
  for (const byte of data) crc = (crc >>> 8) ^ CRC_TABLE[(crc ^ byte) & 255];
  return (crc ^ 0xffffffff) >>> 0;
}

function localHeader(entry, crc) {
  const name = Buffer.from(entry.path, 'ascii');
  const result = Buffer.alloc(30 + name.length);
  result.writeUInt32LE(0x04034b50, 0);
  result.writeUInt16LE(20, 4);
  result.writeUInt16LE(0x21, 12); // Fixed 1980-01-01, 00:00:00, no local timezone.
  result.writeUInt32LE(crc, 14);
  result.writeUInt32LE(entry.data.length, 18);
  result.writeUInt32LE(entry.data.length, 22);
  result.writeUInt16LE(name.length, 26);
  name.copy(result, 30);
  return result;
}

function centralHeader(entry, crc, offset) {
  const name = Buffer.from(entry.path, 'ascii');
  const result = Buffer.alloc(46 + name.length);
  result.writeUInt32LE(0x02014b50, 0);
  result.writeUInt16LE(0x0314, 4); // Unix creator, version 2.0.
  result.writeUInt16LE(20, 6);
  result.writeUInt16LE(0x21, 14);
  result.writeUInt32LE(crc, 16);
  result.writeUInt32LE(entry.data.length, 20);
  result.writeUInt32LE(entry.data.length, 24);
  result.writeUInt16LE(name.length, 28);
  const kind = entry.directory ? 0o040000 : 0o100000;
  result.writeUInt32LE((((kind | entry.mode) << 16) | (entry.directory ? 16 : 0)) >>> 0, 38);
  result.writeUInt32LE(offset, 42);
  name.copy(result, 46);
  return result;
}

function zipChunks(root, files) {
  const local = [];
  const central = [];
  let offset = 0;
  for (const entry of archiveEntries(root, files)) {
    const crc = crc32(entry.data);
    const header = localHeader(entry, crc);
    local.push(header, entry.data);
    central.push(centralHeader(entry, crc, offset));
    offset += header.length + entry.data.length;
  }
  const centralSize = central.reduce((total, chunk) => total + chunk.length, 0);
  const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50, 0);
  end.writeUInt16LE(central.length, 8);
  end.writeUInt16LE(central.length, 10);
  end.writeUInt32LE(centralSize, 12);
  end.writeUInt32LE(offset, 16);
  requireValue(offset + centralSize + end.length <= LIMITS.archive, 'archive wire budget');
  return [...local, ...central, end];
}

export function encodeZip(root, files) {
  return Buffer.concat(zipChunks(root, files));
}

export function decodeZip(input, root, executablePaths) {
  requireValue(Buffer.isBuffer(input) && input.length <= LIMITS.archive, 'archive wire budget');
  const entries = [];
  let offset = 0;
  let total = 0;
  while (offset + 4 <= input.length && input.readUInt32LE(offset) === 0x04034b50) {
    requireValue(offset + 30 <= input.length, 'truncated zip header');
    requireValue(input.readUInt16LE(offset + 6) === 0
      && input.readUInt16LE(offset + 8) === 0, 'zip flags or compression method');
    const size = input.readUInt32LE(offset + 18);
    const length = input.readUInt16LE(offset + 26);
    const extra = input.readUInt16LE(offset + 28);
    const start = offset + 30 + length + extra;
    requireValue(size <= LIMITS.binary && start + size <= input.length, 'zip member byte budget');
    requireValue(size === input.readUInt32LE(offset + 22) && extra === 0, 'zip size or extra field');
    const name = input.subarray(offset + 30, offset + 30 + length);
    requireValue(name.every(byte => byte >= 32 && byte < 127), 'zip path encoding');
    const path = name.toString('ascii');
    const directory = path.endsWith('/');
    total += size;
    requireValue(total <= LIMITS.expanded, 'zip expansion budget');
    entries.push({
      path,
      directory,
      mode: directory || executablePaths.includes(path.slice(root.length + 1)) ? 0o755 : 0o644,
      data: input.subarray(start, start + size),
    });
    requireValue(entries.length <= LIMITS.files * LIMITS.depth + 1, 'zip entry count');
    offset = start + size;
  }
  const files = decodedFiles(root, entries);
  // Compare incrementally without allocating a second full uncompressed archive.
  offset = 0;
  for (const chunk of zipChunks(root, files)) {
    requireValue(chunk.equals(input.subarray(offset, offset + chunk.length)), 'noncanonical zip archive');
    offset += chunk.length;
  }
  requireValue(offset === input.length, 'zip trailing bytes');
  return files;
}
