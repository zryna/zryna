import { inflateRawSync } from 'node:zlib';

// VSIX metadata timestamps are not part of the extension's behavior. Canonicalize only those
// fields, retaining the package's exact compressed bytes, checksums and content inventory.
export function canonicalVsix(input) {
  const bytes = Buffer.from(input);
  const entries = vsixEntries(bytes);
  for (const entry of entries) {
    bytes.writeUInt16LE(0, entry.local + 10);
    bytes.writeUInt16LE(33, entry.local + 12);
    bytes.writeUInt16LE(0, entry.central + 12);
    bytes.writeUInt16LE(33, entry.central + 14);
  }
  return bytes;
}

export function vsixEntries(bytes) {
  if (!Buffer.isBuffer(bytes) || bytes.length < 22 || bytes.length > 4 * 1024 * 1024) throw new Error('VSIX size.');
  const end = bytes.length - 22;
  if (bytes.readUInt32LE(end) !== 0x06054b50 || bytes.readUInt16LE(end + 20) !== 0
    || bytes.readUInt16LE(end + 4) !== 0 || bytes.readUInt16LE(end + 6) !== 0) throw new Error('VSIX ZIP end.');
  let cursor = bytes.readUInt32LE(end + 16);
  const count = bytes.readUInt16LE(end + 10);
  if (count > 64 || count !== bytes.readUInt16LE(end + 8)) throw new Error('VSIX count.');
  const result = [];
  for (let i = 0; i < count; i++) {
    if (cursor + 46 > end || bytes.readUInt32LE(cursor) !== 0x02014b50) throw new Error('VSIX directory.');
    const nameSize = bytes.readUInt16LE(cursor + 28);
    const extra = bytes.readUInt16LE(cursor + 30);
    const comment = bytes.readUInt16LE(cursor + 32);
    const local = bytes.readUInt32LE(cursor + 42);
    const size = bytes.readUInt32LE(cursor + 20);
    const expanded = bytes.readUInt32LE(cursor + 24);
    const method = bytes.readUInt16LE(cursor + 10);
    if (expanded > 1024 * 1024 || local + 30 > cursor || bytes.readUInt32LE(local) !== 0x04034b50
      || ![0, 8].includes(method) || (bytes.readUInt16LE(cursor + 8) & 1)) throw new Error('VSIX entry.');
    const name = bytes.subarray(cursor + 46, cursor + 46 + nameSize).toString('utf8');
    const start = local + 30 + bytes.readUInt16LE(local + 26) + bytes.readUInt16LE(local + 28);
    if (start + size > cursor || !/^[A-Za-z0-9[\]_./-]+$/.test(name)
      || name.split('/').includes('..') || result.some(entry => entry.name === name)) throw new Error('VSIX path.');
    const compressed = bytes.subarray(start, start + size);
    const data = method === 8 ? inflateRawSync(compressed, { maxOutputLength: 1024 * 1024 }) : compressed;
    if (data.length !== expanded) throw new Error('VSIX expansion.');
    result.push({ name, data, local, central: cursor });
    cursor += 46 + nameSize + extra + comment;
  }
  if (cursor !== end) throw new Error('VSIX trailing directory bytes.');
  return result;
}
