// Rejection-only synthetic controls; no verifier or material binary executes.
import assert from 'node:assert/strict';
import test from 'node:test';
import { lifecycleMaterialPins } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
import { authenticateMaterials, selectMaterialBytes, selectMaterialSources } from '../examples/playground/restricted/daemon/lifecycle-owner/authenticate-materials.mjs';

test('material bridge rejects an approval boolean before file access or verification', async () => {
  await assert.rejects(authenticateMaterials({ approved: true }));
});
test('a selected ten-file fragment cannot replace complete signed archive verification', () => {
  assert.throws(() => selectMaterialBytes(Array.from({ length: 10 }, () => ({ data: Buffer.alloc(1) }))));
});
test('complete inventory size and unique ordinary paths are checked before material selection', () => {
  const files = Array.from({ length: 349 }, () => ({ path: 'duplicate', data: { length: 0 } }));
  files[0].data.length = 154023815; // Invalid carriers reach the independent duplicate-path gate.
  assert.throws(() => selectMaterialBytes(files));
});

test('original material sources are closed complete ordered pins, without file or process access', () => {
  const rows = lifecycleMaterialPins().sort((a, b) => a.path < b.path ? -1 : 1)
    .map(({ path }, index) => ({ path, sourcePath: `/original/member-${index}`, ownerUid: 1001 }));
  assert.equal(selectMaterialSources(rows).length, 10);
  for (const change of [value => value.pop(), value => value.reverse(), value => { value[1] = value[0]; },
    value => { value[0].path = '/materials/foreign'; }, value => { value[0].ownerUid = -1; },
    value => { value[0].approved = true; }]) {
    const mutant = structuredClone(rows); change(mutant);
    assert.throws(() => selectMaterialSources(mutant));
  }
});
