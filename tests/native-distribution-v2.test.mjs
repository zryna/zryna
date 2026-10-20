import test from 'node:test';
import assert from 'node:assert/strict';
import { bytes, sha256 } from '../scripts/distribution/canonical.mjs';
import { validateDistribution } from '../scripts/distribution/prepare.mjs';
import {
  NATIVE_FORMAT, NATIVE_RECIPE_SHA256, nativeRecipe, nativeRole, nativeRustMaterials,
  validateNativeDistribution,
} from '../scripts/distribution/native-inventory-v2.mjs';
import {
  nativeInstallationDocuments, prepareNativeDistribution, prepareNativePayload,
} from '../scripts/distribution/native-distribution-v2.mjs';

const TARGETS = ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'];
function fixture(target = TARGETS[0]) {
  const tuple = path => ({ path, size: 1, sha256: 'a'.repeat(64), role: nativeRole(path, target),
    mode: 0o644, material: 'source', licenses: ['LICENSE'] });
  const files = ['LICENSE', 'NOTICE', 'README.md', 'SUPPORT.md', 'VERSION',
    'metadata/architecture-receipt.json', 'metadata/materials.json'].map(tuple);
  for (const item of nativeRustMaterials(target).flatMap(record => record.files)) {
    files.push({ path: item.path, size: item.size, sha256: item.sha256, role: 'license',
      mode: 0o644, material: sha256(Buffer.from(item.path.split('/')[2])), licenses: [item.path] });
  }
  files.sort((left, right) => left.path < right.path ? -1 : 1);
  return { format: NATIVE_FORMAT, version: '0.2.3',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: 'a'.repeat(40), tree: 'b'.repeat(40), sourceDateEpoch: 42 },
    target: { triple: target, archiveFormat: target === TARGETS[0] ? 'tar-gzip' : 'zip',
      platformBaseline: target === TARGETS[0]
        ? { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' }
        : { os: 'windows', product: 'windows-server', version: '2022', architecture: 'x86_64',
          runtime: 'operating-system-ucrt' } },
    recipe: { format: nativeRecipe.format, sha256: NATIVE_RECIPE_SHA256 }, files };
}

test('separate native grammar admits both complete supplier inventories with no runtime payload', () => {
  for (const target of TARGETS) {
    const record = fixture(target);
    validateNativeDistribution(record);
    assert(record.files.every(file => !['runtime', 'provider'].includes(file.role)));
    assert(record.files.some(file => file.path.startsWith('licenses/rust/')));
    assert.throws(() => validateDistribution(record, { productionCandidate: true }), /distribution format/);
    assert.equal(nativeRole(target === TARGETS[0] ? 'bin/zryna' : 'bin/zryna.exe', target), 'cli');
  }
});

test('native candidate rejects release tags, foreign sources, recipe changes and unknown fields', () => {
  for (const mutate of [
    record => { record.source.ref = 'refs/tags/v0.2.3'; },
    record => { record.source.repository = 'https://github.com/foreign/project'; },
    record => { record.source.commit = 'A'.repeat(40); },
    record => { record.source.tree = 'b'.repeat(39); },
    record => { record.source.sourceDateEpoch = 0x100000000; },
    record => { record.source.extra = true; },
    record => { record.recipe.format = 'zryna.distribution-recipe.v1'; },
    record => { record.recipe.sha256 = 'f'.repeat(64); },
    record => { record.format = 'zryna.distribution.v1'; },
    record => { record.version = '0.2.4'; },
    record => { record.extra = true; },
  ]) {
    const record = fixture(); mutate(record);
    assert.throws(() => validateNativeDistribution(record));
  }
});

test('native candidate retains exact supported platform and archive baselines', () => {
  for (const mutate of [
    record => { record.target.triple = 'aarch64-apple-darwin'; },
    record => { record.target.archiveFormat = 'zip'; },
    record => { record.target.platformBaseline.version = '22.04'; },
    record => { record.target.platformBaseline.extra = true; },
  ]) {
    const record = fixture(); mutate(record);
    assert.throws(() => validateNativeDistribution(record));
  }
});

test('native inventory rejects ambient executables, bootstrap, Node and undeclared metadata', () => {
  for (const path of ['runtime/node/bin/node', 'lib/zryna/bootstrap/worker.mjs', 'bin/helper',
    'licenses/node-LICENSE', 'licenses/typescript-LICENSE.txt', 'metadata/extra.json']) {
    const record = fixture();
    record.files.push({ ...record.files[0], path });
    record.files.sort((left, right) => left.path < right.path ? -1 : 1);
    assert.throws(() => validateNativeDistribution(record), path);
  }
});

test('native inventory rejects missing required content and partial or altered supplier notices', () => {
  const record = fixture();
  for (const path of ['LICENSE', 'NOTICE', 'README.md', 'SUPPORT.md', 'VERSION',
    'metadata/architecture-receipt.json', 'metadata/materials.json',
    record.files.find(file => file.path.startsWith('licenses/rust/')).path]) {
    const value = structuredClone(record); value.files = value.files.filter(file => file.path !== path);
    assert.throws(() => validateNativeDistribution(value), path);
  }
  for (const field of ['size', 'sha256']) {
    const value = structuredClone(record);
    const item = value.files.find(file => file.path.startsWith('licenses/rust/'));
    item[field] = field === 'size' ? item.size + 1 : 'f'.repeat(64);
    assert.throws(() => validateNativeDistribution(value), /supplier notice closure/);
  }
});

test('native file admission retains bounds, modes, license coverage, order and alias rejection', () => {
  for (const mutate of [
    record => { record.files[0].size = 0; },
    record => { record.files[0].size = 2 * 1024 * 1024 + 1; },
    record => { record.files[0].mode = 0o755; },
    record => { record.files[0].licenses = ['MISSING']; },
    record => { record.files.push(record.files[0]); },
    record => { record.files.reverse(); },
    record => { record.files[0].path = '../LICENSE'; },
    record => { record.files.push({ ...record.files[0], path: 'license' }); },
  ]) {
    const record = fixture(); mutate(record);
    assert.throws(() => validateNativeDistribution(record));
  }
});

test('native prepared metadata never substitutes for complete captured bytes', () => {
  const record = fixture();
  assert.throws(() => prepareNativeDistribution(record, []), /inventory count mismatch/);
  const payload = record.files.map(file => ({ path: file.path, mode: file.mode, data: Buffer.from('x') }));
  assert.throws(() => prepareNativeDistribution(record, payload), /inventory digest mismatch/);
});

test('native captured material cannot replace source documents, compiler or metadata', () => {
  const record = fixture();
  for (const path of ['README.md', 'SUPPORT.md', 'VERSION', 'NOTICE', 'bin/zryna',
    'metadata/materials.json', 'metadata/architecture-receipt.json', 'metadata/distribution.json',
    'runtime/node/bin/node', 'lib/zryna/bootstrap/worker.mjs']) {
    assert.throws(() => prepareNativePayload(record,
      [{ path, mode: 0o644, data: bytes({ replacement: true }) }], Buffer.from('{}\n')),
    /unexpected native captured material/);
  }
});

test('native documents describe the actual separate source-candidate and runtime boundary', () => {
  const documents = nativeInstallationDocuments('0.2.3', TARGETS[0]);
  assert.equal(documents.length, 4);
  assert(documents.find(file => file.path === 'README.md').data.includes('Compilation requires no Node.js or pnpm'));
  assert(documents.find(file => file.path === 'SUPPORT.md').data.includes('not an authenticated outer production release'));
  assert.throws(() => nativeInstallationDocuments('0.2.4', TARGETS[0]));
  assert.throws(() => nativeInstallationDocuments('0.2.3', 'aarch64-apple-darwin'));
});

test('native source policy admits actual main/codex branches and rejects hostile labels', () => {
  for (const ref of ['refs/heads/main', 'refs/heads/codex/native-install', 'refs/heads/codex/m414/v2.1']) {
    const record = fixture(); record.source.ref = ref; validateNativeDistribution(record);
  }
  for (const ref of ['refs/heads/codex/', 'refs/heads/codex/a//b', 'refs/heads/codex/../main',
    'refs/heads/codex/a..b', 'refs/heads/codex/a.lock', 'refs/heads/codex/a.',
    'refs/heads/codex/.hidden', 'refs/heads/codex/UPPER', 'refs/heads/codex/a@b',
    'refs/heads/codex/a\\b', 'refs/pull/414/merge', 'refs/heads/feature/native',
    'refs/heads/main/extra', `refs/heads/codex/${'a'.repeat(65)}`,
    `refs/heads/codex/${Array(10).fill('a').join('/')}`,
    `refs/heads/codex/${Array(3).fill('a'.repeat(64)).join('/')}`]) {
    const record = fixture(); record.source.ref = ref;
    assert.throws(() => validateNativeDistribution(record), ref);
  }
});
