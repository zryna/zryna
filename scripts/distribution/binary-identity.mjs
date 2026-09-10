import { requireValue } from './canonical.mjs';
import { digestValue } from './inventory.mjs';

export function verifyCompiledIdentity(binary, digest, target) {
  digestValue(digest);
  requireValue(Buffer.isBuffer(binary) && binary.length >= 64, 'compiled binary header');
  if (target === 'x86_64-unknown-linux-gnu') {
    requireValue(binary.subarray(0, 7).equals(Buffer.from([127, 69, 76, 70, 2, 1, 1]))
      && [2, 3].includes(binary.readUInt16LE(16)) && binary.readUInt16LE(18) === 62,
    'compiled CLI is not an x86-64 ELF executable');
  } else {
    requireValue(target === 'x86_64-pc-windows-msvc' && binary.subarray(0, 2).toString('ascii') === 'MZ',
      'compiled CLI is not a Windows executable');
    const pe = binary.readUInt32LE(60);
    requireValue(pe >= 64 && pe + 26 <= binary.length && binary.readUInt32LE(pe) === 0x4550
      && binary.readUInt16LE(pe + 4) === 0x8664 && binary.readUInt16LE(pe + 24) === 0x20b
      && (binary.readUInt16LE(pe + 22) & 0x2002) === 2, 'compiled CLI PE target or executable flags');
  }
  const marker = Buffer.from(`ZRYNA-DISTRIBUTION-V1\0${digest}\0`, 'ascii');
  const first = binary.indexOf(marker);
  requireValue(first >= 0 && binary.indexOf(marker, first + 1) < 0,
    'compiled distribution identity missing or ambiguous');
}
