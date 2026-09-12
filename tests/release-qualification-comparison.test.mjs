import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, parseCanonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { compareReleaseQualifications } from '../scripts/distribution-release/compare-release-qualifications.mjs';
import { writeReleaseQualificationResult } from '../scripts/distribution-release/write-release-qualification-result.mjs';
import {
  createQualificationFixture, QUALIFICATION_TARGET,
} from './release-qualification-fixture.mjs';

async function build(root, replica) {
  const fixture = await createQualificationFixture();
  const value = await writeReleaseQualificationResult({ ...fixture, outputRoot: root, replica });
  return { fixture, value };
}

function roots(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-release-qualification-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  return { firstRoot: join(parent, 'first'), secondRoot: join(parent, 'second'),
    outputRoot: join(parent, 'output') };
}

function rewriteManifest(root, mutate) {
  const path = join(root, 'qualification-result.json');
  const value = parseCanonical(readFileSync(path, 'utf8'));
  mutate(value);
  writeFileSync(path, `${canonicalBounded(value)}\n`);
}

function replaceClaimedArtifact(root, descriptor, bytes) {
  writeFileSync(join(root, descriptor.path), bytes);
  rewriteManifest(root, (value) => {
    const entry = Object.values(value.artifacts).find(({ path }) => path === descriptor.path);
    entry.size = bytes.length;
    entry.sha256 = sha256(bytes);
  });
}

test('independently verifies both replicas before emitting a canonical comparison', async (t) => {
  const paths = roots(t);
  const first = await build(paths.firstRoot, 1);
  const second = await build(paths.secondRoot, 2);
  const result = await compareReleaseQualifications({
    ...paths, target: QUALIFICATION_TARGET, primitives: first.fixture.primitives,
  });
  assert.equal(result.status, 'qualification-only');
  assert.equal(result.productionAdmission, 'forbidden');
  assert.equal(result.comparison, 'byte-identical');
  assert.notEqual(result.builds[0].manifestSha256, result.builds[1].manifestSha256);
  assert.equal(readFileSync(join(paths.outputRoot, 'qualification-comparison.json'), 'utf8'),
    `${canonicalBounded(result)}\n`);
  for (const descriptor of Object.values(first.value.artifacts)) {
    assert.deepEqual(readFileSync(join(paths.outputRoot, descriptor.path)),
      readFileSync(join(paths.firstRoot, descriptor.path)));
  }
  assert.deepEqual(first.fixture.assembled.archive, second.fixture.assembled.archive);
});

test('rejects byte, identity, inventory, and replica drift', async (t) => {
  const bytes = roots(t);
  const firstBytes = await build(bytes.firstRoot, 1);
  const secondBytes = await build(bytes.secondRoot, 2);
  writeFileSync(join(bytes.secondRoot, secondBytes.value.artifacts.archive.path), 'forged archive');
  await assert.rejects(() => compareReleaseQualifications({ ...bytes, target: QUALIFICATION_TARGET,
    primitives: firstBytes.fixture.primitives }), /bytes differ from their qualification descriptor/);

  const identity = roots(t);
  const firstIdentity = await build(identity.firstRoot, 1);
  await build(identity.secondRoot, 2);
  rewriteManifest(identity.secondRoot, (value) => { value.source.tree = 'd'.repeat(40); });
  await assert.rejects(() => compareReleaseQualifications({ ...identity,
    target: QUALIFICATION_TARGET, primitives: firstIdentity.fixture.primitives }),
  /qualification result identity differs from its verified binding/);

  const inventory = roots(t);
  const firstInventory = await build(inventory.firstRoot, 1);
  await build(inventory.secondRoot, 2);
  writeFileSync(join(inventory.firstRoot, 'unexpected.txt'), 'extra');
  await assert.rejects(() => compareReleaseQualifications({ ...inventory,
    target: QUALIFICATION_TARGET, primitives: firstInventory.fixture.primitives }),
  /release file inventory differs/);

  const replica = roots(t);
  const firstReplica = await build(replica.firstRoot, 1);
  await build(replica.secondRoot, 2);
  rewriteManifest(replica.secondRoot, (value) => { value.replica = 1; });
  await assert.rejects(() => compareReleaseQualifications({ ...replica,
    target: QUALIFICATION_TARGET, primitives: firstReplica.fixture.primitives }),
  /qualification target or replica differs/);
});

test('rejects matching garbage archives and matching swapped inventories', async (t) => {
  const garbage = roots(t);
  const firstGarbage = await build(garbage.firstRoot, 1);
  const secondGarbage = await build(garbage.secondRoot, 2);
  const garbageBytes = Buffer.from('same claimed archive in both replicas');
  replaceClaimedArtifact(garbage.firstRoot, firstGarbage.value.artifacts.archive, garbageBytes);
  replaceClaimedArtifact(garbage.secondRoot, secondGarbage.value.artifacts.archive, garbageBytes);
  await assert.rejects(() => compareReleaseQualifications({ ...garbage,
    target: QUALIFICATION_TARGET, primitives: firstGarbage.fixture.primitives }));

  const swapped = roots(t);
  const firstSwapped = await build(swapped.firstRoot, 1);
  const secondSwapped = await build(swapped.secondRoot, 2);
  const swappedBytes = Buffer.from('{"format":"same-swapped-inventory"}\n');
  replaceClaimedArtifact(swapped.firstRoot, firstSwapped.value.artifacts.inventory, swappedBytes);
  replaceClaimedArtifact(swapped.secondRoot, secondSwapped.value.artifacts.inventory, swappedBytes);
  await assert.rejects(() => compareReleaseQualifications({ ...swapped,
    target: QUALIFICATION_TARGET, primitives: firstSwapped.fixture.primitives }),
  /verified archive CLI or inventory differs/);
});

test('rejects matching source and recipe claims that differ from both verified bindings', async (t) => {
  const cases = [
    ['source commit', (value) => {
      value.source.commit = `${value.source.commit.slice(0, 12)}${'e'.repeat(28)}`;
    }],
    ['source tree', (value) => { value.source.tree = 'e'.repeat(40); }],
    ['source epoch', (value) => { value.source.sourceDateEpoch += 1; }],
    ['recipe size', (value) => { value.recipeProposal.size += 1; }],
    ['recipe digest', (value) => { value.recipeProposal.sha256 = 'e'.repeat(64); }],
  ];
  for (const [name, mutate] of cases) {
    await t.test(name, async (subtest) => {
      const paths = roots(subtest);
      const first = await build(paths.firstRoot, 1);
      await build(paths.secondRoot, 2);
      rewriteManifest(paths.firstRoot, mutate);
      rewriteManifest(paths.secondRoot, mutate);
      await assert.rejects(() => compareReleaseQualifications({ ...paths,
        target: QUALIFICATION_TARGET, primitives: first.fixture.primitives }),
      /qualification result identity differs from its verified binding/);
    });
  }
});

test('rejects noncanonical manifests and reused roots', async (t) => {
  const paths = roots(t);
  const first = await build(paths.firstRoot, 1);
  await build(paths.secondRoot, 2);
  writeFileSync(join(paths.firstRoot, 'qualification-result.json'), JSON.stringify(first.value));
  await assert.rejects(() => compareReleaseQualifications({ ...paths,
    target: QUALIFICATION_TARGET, primitives: first.fixture.primitives }), /R406-CANONICAL:/);
  await assert.rejects(() => compareReleaseQualifications({
    firstRoot: paths.secondRoot, secondRoot: paths.secondRoot,
    outputRoot: paths.outputRoot, target: QUALIFICATION_TARGET,
    primitives: first.fixture.primitives,
  }), /exact distinct roots/);
});
