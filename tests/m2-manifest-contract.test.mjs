import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

async function text(relative) {
  return readFile(new URL(`../${relative}`, import.meta.url), 'utf8');
}

test('documents one explicit profile without weakening default M1', async () => {
  const [cli, manifest, status, roadmap] = await Promise.all([
    text('docs/CLI.md'),
    text('docs/M2_MANIFEST_V2.md'),
    text('docs/STATUS.md'),
    text('docs/ROADMAP.md'),
  ]);
  const normalizedManifest = manifest.replace(/\s+/g, ' ');

  for (const required of [
    '`--profile control-flow-v1`',
    'omission means M1',
    '`zryna-manifest-v1.json`',
    '`zryna-manifest-v2.json`',
    '`ZRYNA-N4002`',
    'no partial JavaScript/WebAssembly bundle',
  ]) {
    assert.ok(cli.includes(required), `CLI contract is missing: ${required}`);
  }

  for (const required of [
    'Omitting `--profile` selects the existing protocol-v2 `I32V1` path',
    '`version`, exactly `2`',
    '`profile`, exactly `zryna-control-flow-v1`',
    '`ZRYNA-M2-GRAPH\\0`',
    'Sources, edges, targets, artifacts, results, and diagnostics',
    'The successful rename is the only commit point.',
    'no fallback and no partial portable-target bundle',
    'Issue #56 must still provide',
  ]) {
    assert.ok(normalizedManifest.includes(required), `manifest-v2 contract is missing: ${required}`);
  }

  assert.match(status, /exact `--profile control-flow-v1`[\s\S]*deterministic \[`zryna-manifest-v2\.json`\]/);
  assert.match(roadmap, /^\| #55 \| .* \| #47, #51, #52, #54 \| complete \|$/m);
  assert.match(roadmap, /Aggregate M2 conformance[\s\S]*remains unclaimed until #56/);
});

test('website documentation registry exports the profile and manifest authority', async () => {
  const registry = JSON.parse(await text('docs/website-bundle-v1.json'));
  const manifestEntry = registry.documents.find(({ id }) => id === 'reference/m2-manifest-v2');
  assert.deepEqual(manifestEntry, {
    id: 'reference/m2-manifest-v2',
    source: 'docs/M2_MANIFEST_V2.md',
    path: 'documents/reference/m2-manifest-v2.md',
    title: 'M2 manifest and atomic bundles',
  });
  assert.ok(registry.documents.some(({ id }) => id === 'reference/cli'));
  assert.ok(registry.documents.some(({ id }) => id === 'status/current'));
  assert.ok(registry.documents.some(({ id }) => id === 'status/roadmap'));
});

test('the quick gate includes the focused public manifest/profile contract', async () => {
  const packageDocument = JSON.parse(await text('package.json'));
  assert.match(packageDocument.scripts['m2:quick'], /tests\/m2-manifest-contract\.test\.mjs/);
  assert.equal(
    packageDocument.scripts['docs:check'],
    'node --test tests/docs-bundle.test.mjs tests/m2-contract.test.mjs tests/m2-manifest-contract.test.mjs',
  );
});
