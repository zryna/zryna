import assert from 'node:assert/strict';
import test from 'node:test';
import { sha256 } from '../scripts/stability-gates/input.mjs';
import { zipEntries as legacyM4Entries } from '../scripts/m4/hosted-archive.mjs';
import { openPerformanceArchive, inspectPerformanceArchive, listPerformanceArchive,
  readPerformanceArchiveEntry, closePerformanceArchive } from '../scripts/stability-gates/cli-performance-archive.mjs';
import { archiveFixture } from './cli-performance-archive-fixture.mjs';
const expectation = bytes => ({ bytes: bytes.length, sha256: sha256(bytes) });
const open = (bytes, role = 'primary') => openPerformanceArchive(bytes, expectation(bytes), role);
const central = bytes => bytes.readUInt32LE(bytes.length - 6);
const sample = options => archiveFixture([['receipt.json', '{}\n'], ['raw/output.bin', Buffer.from([0, 255, 13, 10])]], options);

test('independent stored/deflated/descriptor ZIP32 archives preserve exact bytes and false qualification', () => {
  for (const method of [0, 8]) for (const descriptor of [false, true]) {
    const bytes = sample({ method, descriptor }), key = open(bytes), report = inspectPerformanceArchive(key);
    assert.equal(report.entries, 2); assert.equal(report.expandedBytes, 7); assert.equal(report.sha256, sha256(bytes));
    assert.equal(report.archiveIntegrityVerified, true);
    assert.deepEqual([report.sourceAuthenticated, report.performanceQualified, report.thresholds], [false, false, null]);
    assert.deepEqual(listPerformanceArchive(key), [
      { path: 'receipt.json', bytes: 3, sha256: sha256(Buffer.from('{}\n')) },
      { path: 'raw/output.bin', bytes: 4, sha256: sha256(Buffer.from([0, 255, 13, 10])) }]);
    assert(readPerformanceArchiveEntry(key, 'raw/output.bin').equals(Buffer.from([0, 255, 13, 10])));
    closePerformanceArchive(key);
  }
});
test('caller mutations, report copies and fabricated keys cannot rewrite or substitute the validated selection', () => {
  const bytes = sample(), expected = expectation(bytes), key = openPerformanceArchive(bytes, expected);
  bytes.fill(0); expected.sha256 = '9'.repeat(64);
  const content = readPerformanceArchiveEntry(key, 'receipt.json'); content.fill(0);
  const rows = listPerformanceArchive(key); rows[0].sha256 = '9'.repeat(64);
  assert.equal(readPerformanceArchiveEntry(key, 'receipt.json').toString(), '{}\n');
  assert.notEqual(listPerformanceArchive(key)[0].sha256, rows[0].sha256);
  for (const fake of [{}, { ...key }, inspectPerformanceArchive(key)]) assert.throws(() => listPerformanceArchive(fake));
  assert.throws(() => readPerformanceArchiveEntry(key, 'missing.json'));
  closePerformanceArchive(key); assert.throws(() => inspectPerformanceArchive(key));
});
test('reviewed complete archive size/digest and closed role are required before parsing', () => {
  const bytes = sample(), expected = expectation(bytes);
  for (const descriptor of [{ ...expected, bytes: bytes.length + 1 }, { ...expected, sha256: '0'.repeat(64) },
    { ...expected, sha256: undefined }, {}]) assert.throws(() => openPerformanceArchive(bytes, descriptor));
  assert.throws(() => open(bytes, 'caller-profile'));
  assert.throws(() => openPerformanceArchive(Buffer.alloc(128 * 1024 * 1024 + 1), { bytes: 1, sha256: 'a'.repeat(64) }));
  assert.throws(() => openPerformanceArchive(Buffer.alloc(1024 * 1024 + 1), { bytes: 1, sha256: 'a'.repeat(64) }, 'linux-provider'));
});
test('traversal, absolute/device paths, aliases, invalid UTF8 and duplicate case-folded names reject', () => {
  for (const path of ['../escape', '/absolute', 'C:/absolute', 'a\\b', 'a//b', 'a/./b', 'a/NUL.txt', 'a/CON', 'trailing.',
    Buffer.from([0xff]), Buffer.from([0xef, 0xbb, 0xbf, 0x61])]) assert.throws(() => open(archiveFixture([[path, 'x']])));
  for (const entries of [[['a', 'x'], ['a', 'y']], [['A', 'x'], ['a', 'y']]]) assert.throws(() => open(archiveFixture(entries)));
});
test('ZIP end records, missing bytes, central offsets, disk numbers and unaccounted bytes reject', () => {
  const original = sample();
  for (const bytes of [original.subarray(1), original.subarray(0, -1), Buffer.concat([original, Buffer.from('x')]),
    Buffer.alloc(21), archiveFixture([])]) assert.throws(() => open(bytes));
  for (const mutate of [b => b.writeUInt16LE(1, b.length - 18), b => b.writeUInt16LE(1, b.length - 2),
    b => b.writeUInt32LE(central(b) + 1, b.length - 6), b => b.writeUInt16LE(1, b.length - 14),
    b => b.writeUInt32LE(1, central(b) + 42), b => b.writeUInt16LE(1, central(b) + 34)]) {
    const bytes = Buffer.from(original); mutate(bytes); assert.throws(() => open(bytes));
  }
});
test('local/central substitutions, directories, symlinks, unsupported flags/methods and ZIP64 metadata reject', () => {
  for (const mutate of [b => { b[30] ^= 32; }, b => b.writeUInt16LE(1, 6), b => b.writeUInt16LE(1, 28),
    b => b.writeUInt16LE(1, central(b) + 8), b => b.writeUInt16LE(99, central(b) + 10),
    b => b.writeUInt32LE(0x10, central(b) + 38), b => b.writeUInt32LE(0xa0000000, central(b) + 38),
    b => b.writeUInt16LE(1, central(b) + 30), b => b.writeUInt16LE(1, central(b) + 32),
    b => b.writeUInt32LE(0xffffffff, central(b) + 24)]) {
    const bytes = sample(); mutate(bytes); assert.throws(() => open(bytes));
  }
});
test('corrupt CRC, compressed trailing input and descriptor/local inconsistencies fail before any key is returned', () => {
  const corrupt = sample(); corrupt[30 + Buffer.byteLength('receipt.json')] ^= 1; assert.throws(() => open(corrupt));
  assert.throws(() => open(sample({ method: 8, trailing: true })));
  for (const mutate of [b => { b[30 + Buffer.byteLength('receipt.json') + 3 + 4] ^= 1; },
    b => b.writeUInt32LE(12345, 14), b => b.writeUInt32LE(12345, 18), b => b.writeUInt32LE(12345, 22)]) {
    const bytes = sample({ descriptor: true }); mutate(bytes); assert.throws(() => open(bytes));
  }
});
test('M7 supports its fixed larger inventory while original M4 still rejects more than 256 entries', () => {
  const bytes = archiveFixture(Array.from({ length: 4096 }, (_, n) => [`entry-${n}.bin`, '']));
  const key = open(bytes); assert.equal(inspectPerformanceArchive(key).entries, 4096); closePerformanceArchive(key);
  assert.throws(() => legacyM4Entries(bytes));
  assert.throws(() => open(archiveFixture(Array.from({ length: 4097 }, (_, n) => [`entry-${n}.bin`, '']))));
  assert.throws(() => open(bytes, 'linux-provider'));
});
test('per-entry expanded bounds reject before trusting a compressed declared length', () => {
  for (const [role, limit] of [['primary', 64 * 1024 * 1024], ['linux-provider', 256 * 1024]]) {
    const bytes = archiveFixture([['large.bin', 'x']], { method: 8 });
    bytes.writeUInt32LE(limit + 1, central(bytes) + 24);
    assert.throws(() => open(bytes, role), /expanded byte bound/u);
  }
});
test('provider exact expanded entry/aggregate limits pass and one additional byte fails independently', () => {
  const value = Buffer.alloc(256 * 1024), entries = Array.from({ length: 4 }, (_, n) => [`entry-${n}.bin`, value]);
  const bytes = archiveFixture(entries, { method: 8 }), key = open(bytes, 'linux-provider');
  assert.equal(inspectPerformanceArchive(key).expandedBytes, 1024 * 1024); closePerformanceArchive(key);
  assert.throws(() => open(archiveFixture([...entries, ['extra.bin', 'x']], { method: 8 }), 'linux-provider'), /expanded byte bound/u);
});
test('CRC32 matches the independent standard vector and malformed late entries cannot expose a partial selection', () => {
  const known = archiveFixture([['vector.txt', '123456789']]);
  assert.equal(known.readUInt32LE(14), 0xcbf43926);
  const key = open(known); assert.equal(readPerformanceArchiveEntry(key, 'vector.txt').toString(), '123456789'); closePerformanceArchive(key);
  const bytes = sample(); bytes[central(bytes) + 46 + Buffer.byteLength('receipt.json') + 16] ^= 1;
  let partial;
  assert.throws(() => { partial = open(bytes); }); assert.equal(partial, undefined);
});
test('inflate output is bounded by declared size even when the compressed input expands much further', () => {
  const bytes = archiveFixture([['bomb.bin', Buffer.alloc(4096)]], { method: 8 });
  bytes.writeUInt32LE(1, central(bytes) + 24);
  assert.throws(() => open(bytes));
});
test('provider entry-count boundary remains separate from primary inventory capacity', () => {
  const entries = Array.from({ length: 256 }, (_, n) => [`entry-${n}.bin`, '']);
  const key = open(archiveFixture(entries), 'linux-provider'); assert.equal(inspectPerformanceArchive(key).entries, 256);
  closePerformanceArchive(key);
  assert.throws(() => open(archiveFixture([...entries, ['extra.bin', '']]), 'linux-provider'), /entry count/u);
});
test('primary exact 64 MiB entry and 128 MiB aggregate pass, while an extra byte fails without a larger entry', () => {
  const payload = Buffer.alloc(64 * 1024 * 1024);
  const entries = [['first.bin', payload], ['second.bin', payload]];
  const key = open(archiveFixture(entries, { method: 8 }));
  assert.equal(inspectPerformanceArchive(key).expandedBytes, 128 * 1024 * 1024);
  assert.equal(listPerformanceArchive(key)[0].bytes, 64 * 1024 * 1024); closePerformanceArchive(key);
  assert.throws(() => open(archiveFixture([...entries, ['extra.bin', 'x']], { method: 8 })), /expanded byte bound/u);
});
test('exposed backing ArrayBuffers cannot reach a private pooled archive snapshot', () => {
  Buffer.allocUnsafe(Buffer.poolSize / 2 - 1); Buffer.allocUnsafe(Buffer.poolSize / 2 - 1);
  const original = 'unique-private-archive-v1\n', needle = Buffer.alloc(Buffer.byteLength(original)); needle.write(original);
  const key = open(archiveFixture([['private.txt', original]]));
  const exposed = readPerformanceArchiveEntry(key, 'private.txt');
  const backing = Buffer.from(exposed.buffer);
  for (let offset = backing.indexOf(needle); offset >= 0; offset = backing.indexOf(needle, offset + needle.length))
    backing.fill(0x78, offset, offset + needle.length);
  assert.equal(readPerformanceArchiveEntry(key, 'private.txt').toString(), original);
  closePerformanceArchive(key);
});
