import assert from 'node:assert/strict';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { compareReleaseQualifications } from '../scripts/distribution-release/compare-release-qualifications.mjs';

const TARGET = 'x86_64-unknown-linux-gnu';
const COMMIT = 'a'.repeat(40);
const BASE = `zryna-qualification-0.2.0-${TARGET}-${COMMIT.slice(0, 12)}`;
const FILES = {
  binding: `${BASE}.input.json`,
  cli: `${BASE}.elf`,
  inventory: `${BASE}.inventory.json`,
  archive: `${BASE}.tar.gz`,
  inspection: `${BASE}.inspection.json`,
};

function build(root, replica, mutate = () => {}) {
  const bytes = {
    binding: Buffer.from('{"qualification":true}\n'),
    cli: Buffer.alloc(64, 1),
    inventory: Buffer.from('{"files":[]}\n'),
    archive: Buffer.from('qualification archive'),
    inspection: Buffer.from('{"format":"ELF"}\n'),
  };
  const value = {
    format: 'zryna.release-qualification-result.v1',
    status: 'qualification-only',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    target: TARGET,
    replica,
    source: {
      ref: 'refs/heads/main', commit: COMMIT, tree: 'b'.repeat(40), sourceDateEpoch: 1_789_081_200,
    },
    recipeProposal: { size: 42, sha256: 'c'.repeat(64) },
    artifacts: Object.fromEntries(Object.entries(FILES).map(([key, path]) => [key, {
      path, size: bytes[key].length, sha256: sha256(bytes[key]),
    }])),
  };
  mutate({ bytes, value });
  for (const [key, path] of Object.entries(FILES)) writeFileSync(join(root, path), bytes[key]);
  writeFileSync(join(root, 'qualification-result.json'), `${canonicalBounded(value)}\n`);
  return value;
}

function roots(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-release-qualification-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const firstRoot = join(parent, 'first');
  const secondRoot = join(parent, 'second');
  const outputRoot = join(parent, 'output');
  mkdirSync(firstRoot);
  mkdirSync(secondRoot);
  return { firstRoot, secondRoot, outputRoot };
}

test('compares every qualification byte and emits a canonical non-production result', (t) => {
  const paths = roots(t);
  const first = build(paths.firstRoot, 1);
  build(paths.secondRoot, 2);
  const result = compareReleaseQualifications({ ...paths, target: TARGET });
  assert.equal(result.status, 'qualification-only');
  assert.equal(result.productionAdmission, 'forbidden');
  assert.equal(result.comparison, 'byte-identical');
  assert.notEqual(result.builds[0].manifestSha256, result.builds[1].manifestSha256);
  assert.equal(readFileSync(join(paths.outputRoot, 'qualification-comparison.json'), 'utf8'),
    `${canonicalBounded(result)}\n`);
  for (const [key, path] of Object.entries(FILES)) {
    assert.deepEqual(readFileSync(join(paths.outputRoot, path)),
      readFileSync(join(paths.firstRoot, first.artifacts[key].path)));
  }
});

test('rejects byte, identity, inventory, and replica drift', (t) => {
  const bytes = roots(t);
  build(bytes.firstRoot, 1);
  build(bytes.secondRoot, 2, ({ bytes: data }) => { data.archive = Buffer.from('forged archive bytes'); });
  assert.throws(() => compareReleaseQualifications({ ...bytes, target: TARGET }),
    /R406-RELEASE-FILE:.*replica (sizes|bytes) differ/);

  const identity = roots(t);
  build(identity.firstRoot, 1);
  build(identity.secondRoot, 2, ({ value }) => { value.source.tree = 'd'.repeat(40); });
  assert.throws(() => compareReleaseQualifications({ ...identity, target: TARGET }),
    /R406-QUALIFICATION-COMPARISON: qualification result identities differ/);

  const inventory = roots(t);
  build(inventory.firstRoot, 1);
  build(inventory.secondRoot, 2);
  writeFileSync(join(inventory.firstRoot, 'unexpected.txt'), 'extra');
  assert.throws(() => compareReleaseQualifications({ ...inventory, target: TARGET }),
    /R406-RELEASE-FILE: release file inventory differs/);

  const replica = roots(t);
  build(replica.firstRoot, 1);
  build(replica.secondRoot, 1);
  assert.throws(() => compareReleaseQualifications({ ...replica, target: TARGET }),
    /R406-QUALIFICATION-COMPARISON: qualification target or replica differs/);
});

test('rejects noncanonical manifests and reused roots', (t) => {
  const paths = roots(t);
  const value = build(paths.firstRoot, 1);
  build(paths.secondRoot, 2);
  writeFileSync(join(paths.firstRoot, 'qualification-result.json'), JSON.stringify(value));
  assert.throws(() => compareReleaseQualifications({ ...paths, target: TARGET }),
    /R406-CANONICAL:/);
  assert.throws(() => compareReleaseQualifications({
    firstRoot: paths.secondRoot, secondRoot: paths.secondRoot,
    outputRoot: paths.outputRoot, target: TARGET,
  }), /R406-QUALIFICATION-COMPARISON: exact distinct roots/);
});
