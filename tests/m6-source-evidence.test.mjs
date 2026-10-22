import assert from 'node:assert/strict';
import test from 'node:test';
import { validateM6Record } from '../scripts/m6/evidence.mjs';
import { validateSourceM6Record, validatePublishedSourceComposition } from '../scripts/m6/source-evidence.mjs';
import { sourceRegistry } from '../scripts/m6/source-registry.mjs';
import { sourceFixture } from './m6/source-fixture.mjs';
import { encode, hash, fixture } from './m6/fixture.mjs';

function reject(change, pattern) {
  const { record, policy } = sourceFixture();
  change(record, policy);
  const data = encode(record); policy.recordSha256 = hash(data);
  assert.throws(() => validateSourceM6Record(data, policy), pattern);
}

test('complete synthetic v2 metadata validates only its fixed route, never v1 or publication authority', () => {
  const { record, policy } = sourceFixture();
  assert.deepEqual(JSON.parse(JSON.stringify(validateSourceM6Record(encode(record), policy))), record);
  assert.throws(() => validateM6Record(encode(record), policy));
  const original = fixture();
  assert.throws(() => validateSourceM6Record(encode(original.record), original.policy));
  for (const candidate of [{}, record, record.publication, { status: 'authenticated' }]) {
    assert.throws(() => validatePublishedSourceComposition(record, candidate), /PUBLICATION-CAPABILITY/);
  }
});

test('all v2 cases, including original obligations and explicit signature successor, remain mandatory', () => {
  const { record } = sourceFixture();
  assert.equal(record.gates.length, 23);
  for (const gate of record.gates) {
    for (const item of gate.cases) {
      reject(r => { r.gates.find(g => g.id === gate.id).cases = gate.cases.filter(c => c.id !== item.id); }, /M6-CASES/);
      for (const field of ['failed', 'skipped']) reject(r => {
        r.gates.find(g => g.id === gate.id).cases.find(c => c.id === item.id)[field] = 1;
      }, /M6-CASES/);
    }
  }
  reject(r => { r.gates.find(g => g.id === 'publication-linux').cases.find(c =>
    c.id === 'published-compiler-signature').id = 'outer-signature'; }, /M6-CASES/);
});

test('material roots, images, fixed helpers and each platform tool are required in their owning gates', () => {
  for (const family of sourceRegistry.gates) for (const platform of family.platforms) {
    for (const role of family.artifactRoles.map(r => r.replace('-platform', `-${platform}`))) {
      reject(r => { const gate = r.gates.find(g => g.id === `${family.id}-${platform}`);
        gate.artifactRoles = gate.artifactRoles.filter(value => value !== role); }, /M6-/);
    }
  }
  for (const role of sourceRegistry.requiredArtifacts) reject((r, p) => {
    r.artifacts = r.artifacts.filter(value => value.role !== role);
    p.artifactPins = p.artifactPins.filter(value => value.role !== role);
  }, /M6-/);
});

test('separate archive, executable, composition and docs identities cannot collapse into old toolkit claims', () => {
  for (const field of ['compilerArchiveSha256', 'compositionSha256', 'docsManifestSha256']) {
    reject(r => { r.publication[field] = hash('substitution'); }, /M6-PUBLICATION/);
  }
  reject(r => { delete r.publication.compilerBinarySha256; });
  reject(r => { r.publication.compilerBinarySha256 = 'A'.repeat(64); }, /M6-PUBLICATION/);
  reject(r => { r.publication.toolkitSha256 = r.publication.compositionSha256; });
  reject((r, p) => { r.artifacts.find(a => a.role === 'source-composition').version = '9.9.9';
    p.artifactPins = structuredClone(r.artifacts); }, /M6-VERSION/);
  reject(r => { r.integration.clean = false; }, /M6-INTEGRATION/);
  reject(r => { r.gates.find(g => g.id === 'hosted-windows').hosted.conclusion = 'skipped'; }, /M6-HOSTED/);
});

test('selected digest cannot excuse altered pins, expiring retention or noncanonical record bytes', () => {
  reject(r => { r.artifacts[0].sha256 = hash('other'); }, /M6-ARTIFACT-PIN/);
  reject(r => { r.gates[0].receipt.url = 'https://github.com/zryna/zryna/actions/runs/1/artifacts/2'; }, /M6-RETENTION/);
  const { record, policy } = sourceFixture();
  for (const data of [Buffer.concat([encode(record), Buffer.from(' ')]), Buffer.alloc(262145),
    Buffer.from(encode(record).toString().replace('"version":2', '"version":2,"version":2'))]) {
    policy.recordSha256 = hash(data);
    assert.throws(() => validateSourceM6Record(data, policy));
  }
});
