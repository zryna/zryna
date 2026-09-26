import { test } from 'node:test';
import assert from 'node:assert/strict';
import { canonicalVsix, vsixEntries } from '../scripts/portable-setup/vsix.mjs';
import { verifyEditorManifest } from '../scripts/portable-setup/build.mjs';
import { encodeZip } from '../scripts/distribution/archive-zip.mjs';

function fixture() {
  return encodeZip('extension', [{ path: 'package.json', mode: 0o644,
    data: Buffer.from('{"name":"zryna","version":"0.4.0"}') }]);
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

test('candidate assembly rejects a VSIX with mismatched server, editor or profile metadata', () => {
  const metadata = {
    name: 'zryna', version: '0.4.0', zrynaCompatibility: {
      compilerVersion: '0.2.3', serverVersion: '0.4.0', installationCapability: 'portable-setup-v1',
      requiredCapabilities: { 'scalar-v2': 'scalar-format-v1', 'control-flow-v1': 'control-flow-format-v1' },
      profiles: ['scalar-v2', 'control-flow-v1'], sourceBuildRequired: false,
      releasedCompilerCompatible: true,
    },
  };
  const entries = [{ name: 'extension/package.json', data: Buffer.from(JSON.stringify(metadata)) }];
  assert.doesNotThrow(() => verifyEditorManifest(entries));
  for (const [field, value] of [['version', '0.3.0'], ['version', '0.5.0']]) {
    const original = metadata[field];
    metadata[field] = value;
    entries[0].data = Buffer.from(JSON.stringify(metadata));
    assert.throws(() => verifyEditorManifest(entries), /compatibility/);
    metadata[field] = original;
  }
  for (const [field, value] of [['compilerVersion', '0.3.0'], ['serverVersion', '0.3.0'],
    ['installationCapability', 'portable-setup-v2']]) {
    const original = metadata.zrynaCompatibility[field];
    metadata.zrynaCompatibility[field] = value;
    entries[0].data = Buffer.from(JSON.stringify(metadata));
    assert.throws(() => verifyEditorManifest(entries), /compatibility/);
    metadata.zrynaCompatibility[field] = original;
  }
  metadata.zrynaCompatibility.requiredCapabilities['control-flow-v1'] = 'scalar-format-v1';
  entries[0].data = Buffer.from(JSON.stringify(metadata));
  assert.throws(() => verifyEditorManifest(entries), /compatibility/);
});
