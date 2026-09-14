import test from 'node:test';
import assert from 'node:assert/strict';
import { preparePayload } from '../scripts/distribution/payload.mjs';
import { installationDocuments } from '../scripts/distribution/documents.mjs';
import { validateDistribution } from '../scripts/distribution/prepare.mjs';

const identity = { version: '0.2.3', target: { triple: 'x86_64-unknown-linux-gnu' } };

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
  assert.throws(() => installationDocuments('0.2.3', 'aarch64-apple-darwin'), /target/);
});

test('distribution validation admits the legacy release only through an explicit exact binding', () => {
  const record = {
    format: 'zryna.distribution.v1', version: '0.2.1',
    source: { repository: 'invalid', ref: 'refs/tags/v0.2.1', commit: 'a'.repeat(40),
      tree: 'b'.repeat(40), sourceDateEpoch: 1 },
    target: {}, recipe: {}, files: [],
  };
  assert.throws(() => validateDistribution(record), /unapproved distribution version/);
  assert.throws(() => validateDistribution(record, { acceptedVersion: '0.2.1' }),
    /source identity/);
  assert.throws(() => validateDistribution(
    { ...record, version: '0.2.0' }, { acceptedVersion: '0.2.0' },
  ), /unapproved distribution version/);
  assert.throws(() => validateDistribution(
    record, { acceptedVersion: '0.2.1', productionCandidate: true },
  ), /unapproved distribution version/);
});
