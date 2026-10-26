import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { rustMaterials, validateRustMaterials } from '../scripts/distribution/rust-materials.mjs';
import { securityRustMaterials, matchesSecurityRustInputs, securityNativeRecipe,
  SECURITY_NATIVE_RECIPE_SHA256 } from '../scripts/distribution/rust-security-materials.mjs';
import { nativeRustMaterials, nativeRecipe, NATIVE_RECIPE_SHA256,
  nativeRecipeFor, validateNativeDistribution, nativeRole } from '../scripts/distribution/native-inventory-v2.mjs';
import { rustNoticeOwner } from '../scripts/distribution/rust-notice-owner.mjs';

const TARGETS = ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'];
const read = file => JSON.parse(readFileSync(new URL(`../scripts/distribution/${file}`, import.meta.url)));
const binding = read('rust-security-binding-v1.json');
function fixture(target = TARGETS[0]) {
  const source = { repository: binding.sourceRepository, commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
  const receipt = { source, inputs: structuredClone(binding.inputs) };
  const distribution = { version: '0.2.3', source: { ...source, ref: binding.sourceRef },
    target: { triple: target }, recipe: { format: 'zryna.distribution-recipe.v1', sha256: binding.recipeSha256 } };
  return { target, receipt, distribution };
}
const context = value => ({ productionCandidate: true,
  architectureReceipt: value.receipt, distribution: value.distribution });
const entries = records => records.flatMap(record => record.files)
  .map(({ path, size, sha256 }) => ({ path, size, sha256 })).sort((a, b) => a.path < b.path ? -1 : 1);
function validate(value, files = entries(securityRustMaterials(value.target))) {
  return validateRustMaterials(files, value.target, value.receipt, '0.2.3', context(value));
}

test('security candidate selects exact current target closures while legacy defaults remain historical', () => {
  for (const [index, target] of TARGETS.entries()) {
    const value = fixture(target), records = rustMaterials(target, context(value));
    assert.equal(records.length, [163, 170][index]);
    assert.equal(records.find(record => record.name === 'wasmtime').version, '48.0.7');
    assert.equal(rustMaterials(target).find(record => record.name === 'wasmtime').version, '48.0.1');
    assert.deepEqual(records, securityRustMaterials(target));
    validate(value);
    assert.throws(() => validate(value, entries(rustMaterials(target))), /Rust notice inventory differs/);
  }
});

test('security source binding rejects drift in all four inputs and pristine retry remains valid', () => {
  for (let index = 0; index < 4; index++) {
    for (const field of ['size', 'sha256', 'logicalPath']) {
      const value = fixture(); validate(value);
      value.receipt.inputs[index][field] = field === 'size' ? 1 : 'unreviewed';
      assert.equal(matchesSecurityRustInputs(value.receipt), false);
      assert.throws(() => validate(value), /Rust material lockfile identity/);
      validate(fixture());
    }
  }
  for (const mutate of [value => value.receipt.inputs.pop(),
    value => value.receipt.inputs.push(value.receipt.inputs[0]), value => value.receipt.inputs.reverse()]) {
    const value = fixture(); mutate(value); assert.throws(() => validate(value)); validate(fixture());
  }
});

test('security candidate cannot be selected by tags truthy flags missing identities or foreign bindings', () => {
  for (const mutate of [
    value => { value.distribution.source.ref = 'refs/tags/v0.2.3'; },
    value => { value.distribution.source.repository = 'https://example.invalid/project'; },
    value => { value.distribution.source.commit = 'c'.repeat(40); },
    value => { delete value.distribution.source.commit; delete value.receipt.source.commit; },
    value => { value.distribution.source.tree = 'B'.repeat(40); value.receipt.source.tree = 'B'.repeat(40); },
    value => { value.distribution.recipe.sha256 = '0'.repeat(64); },
    value => { value.distribution.recipe.format = 'unreviewed'; },
    value => { value.distribution.version = '0.2.4'; },
    value => { value.distribution.target.triple = TARGETS[1]; },
  ]) {
    const value = fixture(); validate(value); mutate(value); assert.throws(() => validate(value)); validate(fixture());
  }
  const value = fixture();
  assert.throws(() => rustMaterials(value.target, { ...context(value), productionCandidate: 'true' }));
  assert.throws(() => validateRustMaterials(entries(securityRustMaterials(value.target)), value.target,
    value.receipt, '0.2.3', { ...context(value), productionCandidate: false }));
});

test('security notice inventory rejects omitted extra duplicate substituted and previous-version records', () => {
  for (const mutate of [files => files.pop(), files => files.push({ ...files[0], path: 'licenses/rust/extra/LICENSE' }),
    files => files.push(files[0]), files => { files[0].sha256 = '0'.repeat(64); },
    files => { files[0].size++; }, files => { files[0].path += '.foreign'; }]) {
    const value = fixture(), files = entries(securityRustMaterials(value.target)); validate(value, files);
    mutate(files); assert.throws(() => validate(value, files), /Rust notice inventory differs/); validate(fixture());
  }
});

test('native successor selection is explicit and the original native recipe remains unchanged', () => {
  const selected = { format: securityNativeRecipe.format, sha256: SECURITY_NATIVE_RECIPE_SHA256 };
  for (const target of TARGETS) {
    assert.equal(nativeRustMaterials(target).find(record => record.name === 'wasmtime').version, '48.0.1');
    assert.deepEqual(nativeRustMaterials(target, selected), securityRustMaterials(target));
    assert.equal(nativeRecipeFor(selected), securityNativeRecipe);
    assert.equal(nativeRecipeFor({ format: nativeRecipe.format, sha256: NATIVE_RECIPE_SHA256 }), nativeRecipe);
    assert.throws(() => nativeRustMaterials(target, { ...selected, sha256: 'f'.repeat(64) }));
  }
});

function nativeFixture(target) {
  const tuple = (path, size = 1, sha256 = 'a'.repeat(64)) => ({ path, size, sha256,
    role: nativeRole(path, target), mode: 0o644, material: 'source', licenses: [path.startsWith('licenses/') ? path : 'LICENSE'] });
  const files = ['LICENSE', 'NOTICE', 'README.md', 'SUPPORT.md', 'VERSION',
    'metadata/architecture-receipt.json', 'metadata/materials.json'].map(path => tuple(path));
  files.push(...securityRustMaterials(target).flatMap(record => record.files).map(file => tuple(file.path, file.size, file.sha256)));
  files.sort((a, b) => a.path < b.path ? -1 : 1);
  return { format: 'zryna.native-distribution.v2', version: '0.2.3', files,
    source: { repository: binding.sourceRepository, ref: 'refs/heads/main', commit: 'a'.repeat(40), tree: 'b'.repeat(40), sourceDateEpoch: 42 },
    target: { triple: target, archiveFormat: target === TARGETS[0] ? 'tar-gzip' : 'zip',
      platformBaseline: target === TARGETS[0] ? { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' }
        : { os: 'windows', product: 'windows-server', version: '2022', architecture: 'x86_64', runtime: 'operating-system-ucrt' } },
    recipe: { format: securityNativeRecipe.format, sha256: SECURITY_NATIVE_RECIPE_SHA256 } };
}

test('native successor grammar rejects historical recipe confusion and incomplete new supplier inventories', () => {
  for (const target of TARGETS) {
    const value = nativeFixture(target); validateNativeDistribution(value);
    const old = structuredClone(value); old.recipe.sha256 = NATIVE_RECIPE_SHA256;
    assert.throws(() => validateNativeDistribution(old), /supplier notice closure/);
    value.files.pop(); assert.throws(() => validateNativeDistribution(value));
    validateNativeDistribution(nativeFixture(target));
  }
});

test('upstream notices bind a unique selected Wasmtime version even when metadata contains old versions', () => {
  const records = securityRustMaterials(TARGETS[0]);
  const record = records.find(item => item.files.some(file => file.origin.includes('raw.githubusercontent.com')));
  const file = record.files.find(item => item.origin.includes('raw.githubusercontent.com'));
  const pkg = version => ({ name: 'wasmtime', version, source: 'registry+https://github.com/rust-lang/crates.io-index', manifest_path: `/registry/wasmtime-${version}/Cargo.toml` });
  const metadata = { packages: [pkg('48.0.1'), pkg('48.0.7')] };
  assert.equal(rustNoticeOwner(record, file, records, metadata).version, '48.0.7');
  for (const mutate of [value => value.packages.pop(), value => value.packages.push(pkg('48.0.7')),
    value => { value.packages[1].source = null; }]) {
    const bad = structuredClone(metadata); mutate(bad); assert.throws(() => rustNoticeOwner(record, file, records, bad));
    assert.equal(rustNoticeOwner(record, file, records, metadata).version, '48.0.7');
  }
  const wasmtime = records.find(item => item.name === 'wasmtime');
  assert.equal(rustNoticeOwner(wasmtime, wasmtime.files[0], records, metadata).version, '48.0.7');
  assert.throws(() => rustNoticeOwner(record, file, [...records, wasmtime], metadata));
  const oldRecords = rustMaterials(TARGETS[0]);
  const oldRecord = oldRecords.find(item => item.files.some(notice => notice.origin.includes('raw.githubusercontent.com')));
  const oldFile = oldRecord.files.find(item => item.origin.includes('raw.githubusercontent.com'));
  assert.equal(rustNoticeOwner(oldRecord, oldFile, oldRecords, metadata).version, '48.0.1');
  const missing = records.filter(item => item.name !== 'wasmtime');
  assert.throws(() => rustNoticeOwner(record, file, missing, metadata));
  const wrong = structuredClone(file); wrong.sha256 = '0'.repeat(64);
  assert.throws(() => rustNoticeOwner(record, wrong, records, metadata));
});
