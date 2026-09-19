import { test } from 'node:test';
import assert from 'node:assert/strict';
import { canonicalVsix, vsixEntries } from '../scripts/portable-setup/vsix.mjs';
import { encodeZip } from '../scripts/distribution/archive-zip.mjs';

function fixture() {
  return encodeZip('extension', [{ path: 'package.json', mode: 0o644,
    data: Buffer.from('{"name":"zryna","version":"0.3.0"}') }]);
}

test('VSIX canonicalization retains every content byte and normalizes both timestamp fields', () => {
  const input = fixture();
  for (const entry of vsixEntries(input)) {
    input.writeUInt16LE(123, entry.local + 10);
    input.writeUInt16LE(456, entry.central + 12);
  }
  const actual = canonicalVsix(input);
  assert.deepEqual(vsixEntries(actual).map(entry => [entry.name, entry.data]),
    vsixEntries(input).map(entry => [entry.name, entry.data]));
  assert.deepEqual(canonicalVsix(actual), actual);
  for (const entry of vsixEntries(actual)) {
    assert.equal(actual.readUInt16LE(entry.local + 10), 0);
    assert.equal(actual.readUInt16LE(entry.central + 12), 0);
  }
});

test('VSIX parser rejects missing end, foreign method, encryption and excessive expansion', () => {
  assert.throws(() => vsixEntries(Buffer.alloc(4)));
  assert.throws(() => vsixEntries(fixture().subarray(0, -1)));
  for (const mutate of [
    (bytes, entry) => bytes.writeUInt16LE(99, entry.central + 10),
    (bytes, entry) => bytes.writeUInt16LE(1, entry.central + 8),
    (bytes, entry) => bytes.writeUInt32LE(2 * 1024 * 1024, entry.central + 24),
    (bytes, entry) => bytes.writeUInt32LE(0xffffffff, entry.central + 42),
  ]) {
    const bytes = fixture();
    mutate(bytes, vsixEntries(bytes)[1]);
    assert.throws(() => vsixEntries(bytes));
  }
});
