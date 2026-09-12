import test from 'node:test';
import assert from 'node:assert/strict';
import { preparePayload } from '../scripts/distribution/payload.mjs';
import { installationDocuments } from '../scripts/distribution/documents.mjs';

const identity = { version: '0.2.0', target: { triple: 'x86_64-unknown-linux-gnu' } };

test('captured inputs cannot replace generated installation metadata or instructions', () => {
  for (const path of ['README.md', 'SUPPORT.md', 'VERSION', 'metadata/materials.json',
    'metadata/architecture-receipt.json', 'metadata/distribution.json', 'bin/zryna']) {
    const captured = [{ path, mode: 0o644, data: Buffer.from('replacement') }];
    assert.throws(() => preparePayload(identity, captured, Buffer.from('{}\n')),
      /unexpected captured payload/);
  }
});

test('installation documents reject unapproved versions and targets', () => {
  assert.throws(() => installationDocuments('0.1.0', identity.target.triple), /unapproved/);
  assert.throws(() => installationDocuments('0.2.0', 'aarch64-apple-darwin'), /target/);
});
