// Synthetic ZIP32 construction independent of the production reader.
import { crc32, deflateRawSync } from 'node:zlib';
export function archiveFixture(entries, { method = 0, descriptor = false, trailing = false } = {}) {
  const locals = [], centrals = []; let offset = 0;
  for (const [path, input] of entries) {
    const name = Buffer.from(path), bytes = Buffer.from(input);
    const crc = crc32(bytes);
    const encoded = method === 8 ? deflateRawSync(bytes) : bytes;
    const content = trailing ? Buffer.concat([encoded, Buffer.from('extra')]) : encoded;
    const local = Buffer.alloc(30), central = Buffer.alloc(46), suffix = Buffer.alloc(descriptor ? 16 : 0);
    local.writeUInt32LE(0x04034b50); local.writeUInt16LE(20, 4);
    local.writeUInt16LE(descriptor ? 8 : 0, 6); local.writeUInt16LE(method, 8);
    if (!descriptor) { local.writeUInt32LE(crc, 14); local.writeUInt32LE(content.length, 18); local.writeUInt32LE(bytes.length, 22); }
    local.writeUInt16LE(name.length, 26);
    central.writeUInt32LE(0x02014b50); central.writeUInt16LE(20, 4); central.writeUInt16LE(20, 6);
    central.writeUInt16LE(descriptor ? 8 : 0, 8); central.writeUInt16LE(method, 10);
    central.writeUInt32LE(crc, 16); central.writeUInt32LE(content.length, 20); central.writeUInt32LE(bytes.length, 24);
    central.writeUInt16LE(name.length, 28); central.writeUInt32LE(offset, 42);
    if (descriptor) { suffix.writeUInt32LE(0x08074b50); suffix.writeUInt32LE(crc, 4);
      suffix.writeUInt32LE(content.length, 8); suffix.writeUInt32LE(bytes.length, 12); }
    locals.push(local, name, content, suffix); centrals.push(central, name);
    offset += 30 + name.length + content.length + suffix.length;
  }
  const local = Buffer.concat(locals), central = Buffer.concat(centrals), end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50); end.writeUInt16LE(entries.length, 8); end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(central.length, 12); end.writeUInt32LE(local.length, 16);
  return Buffer.concat([local, central, end]);
}
