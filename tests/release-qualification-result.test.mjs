import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded } from '../scripts/distribution-release/canonical.mjs';
import { writeReleaseQualificationResult } from '../scripts/distribution-release/write-release-qualification-result.mjs';
import { createQualificationFixture } from './release-qualification-fixture.mjs';

async function fixture(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-qualification-result-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  return { ...await createQualificationFixture(), outputRoot: join(parent, 'output'), replica: 1 };
}

test('verifies and writes one exact qualification-only replica result', async (t) => {
  const value = await fixture(t);
  const result = await writeReleaseQualificationResult(value);
  assert.equal(result.productionAdmission, 'forbidden');
  assert.equal(readFileSync(join(value.outputRoot, 'qualification-result.json'), 'utf8'),
    `${canonicalBounded(result)}\n`);
  assert.deepEqual(readFileSync(join(value.outputRoot, result.artifacts.cli.path)), value.cli);
});

test('rejects inspection drift and an existing output root', async (t) => {
  const drift = await fixture(t);
  const changed = structuredClone(drift.inspectionValue);
  changed.binary.sha256 = 'f'.repeat(64);
  drift.inspection = Buffer.from(`${canonicalBounded(changed)}\n`);
  await assert.rejects(() => writeReleaseQualificationResult(drift),
    /R406-QUALIFICATION-INSPECTION: inspection, qualification input, and binary identities differ/);

  const existing = await fixture(t);
  await writeReleaseQualificationResult(existing);
  await assert.rejects(() => writeReleaseQualificationResult(existing), /EEXIST/);
});

test('rejects an unverified archive or caller-swapped inventory before writing', async (t) => {
  const archive = await fixture(t);
  archive.assembled = { ...archive.assembled, archive: Buffer.from('not an archive') };
  await assert.rejects(() => writeReleaseQualificationResult(archive));

  const inventory = await fixture(t);
  inventory.assembled = {
    ...inventory.assembled,
    inventory: Buffer.from('{"format":"swapped-inventory"}\n'),
  };
  await assert.rejects(() => writeReleaseQualificationResult(inventory),
    /verified archive CLI or inventory differs/);
});
