// Rejection-only synthetic controls; no verifier or material binary executes.
import assert from 'node:assert/strict';
import test from 'node:test';
import { lifecycleMaterialPins } from '../scripts/stability-gates/lifecycle-owner-inputs.mjs';
import { authenticateMaterials, materialArchiveExpectation, materialInputFiles, selectMaterialBytes, selectMaterialSources } from '../examples/playground/restricted/daemon/lifecycle-owner/authenticate-materials.mjs';
import { release } from '../examples/playground/restricted/daemon/cli-parity/plan.mjs';

test('signed envelope tag metadata stays outside the archive source identity without accepting another release', () => {
  const source = { repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/v0.2.3',
    commit: release.sourceCommit, tree: release.sourceTree, sourceDateEpoch: 0, tagObject: 'c'.repeat(40) };
  const document = { version: release.version, source, recipe: {}, subjects: [{
    target: 'x86_64-unknown-linux-gnu', platformBaseline: {},
    archive: { path: 'selected.tar.gz', size: release.archive.bytes, sha256: release.archive.sha256 },
  }] };
  const expected = materialArchiveExpectation(document);
  assert.equal(Object.hasOwn(expected.source, 'tagObject'), false);
  assert.equal(document.source.tagObject, 'c'.repeat(40), 'complete signed envelope is retained');
  for (const change of [value => { value.source.commit = 'f'.repeat(40); },
    value => { value.source.tree = 'f'.repeat(40); }, value => { value.version = '0.2.2'; },
    value => { value.subjects[0].archive.sha256 = 'f'.repeat(64); }]) {
    const mutant = structuredClone(document); change(mutant);
    assert.throws(() => materialArchiveExpectation(mutant));
  }
});

test('material bridge rejects an approval boolean before file access or verification', async () => {
  await assert.rejects(authenticateMaterials({ approved: true }));
});
test('a selected ten-file fragment cannot replace complete signed archive verification', () => {
  const fragment = Array.from({ length: 10 }, () => ({ data: Buffer.alloc(1) }));
  assert.throws(() => selectMaterialBytes(fragment));
  assert.throws(() => materialInputFiles(fragment));
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
