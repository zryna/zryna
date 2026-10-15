import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { sha256 } from '../scripts/distribution/canonical.mjs';
import { rustMaterials, validateRustMaterials } from '../scripts/distribution/rust-materials.mjs';

function commandCandidate(target = 'x86_64-unknown-linux-gnu') {
  const binding = JSON.parse(readFileSync(new URL(
    '../scripts/distribution/command-h1-rust-binding-v1.json', import.meta.url)));
  const source = { repository: 'https://github.com/zryna/zryna',
    commit: '1'.repeat(40), tree: '2'.repeat(40) };
  const receipt = { source, inputs: binding.inputs };
  const distribution = { version: '0.2.3', source: { ...source, ref: 'refs/heads/main' },
    target: { triple: target }, recipe: { format: 'zryna.distribution-recipe.v1',
      sha256: '0f522abd343ba4e3c07a38d93c778fa7a0e30aac2295abe1f075091f6b6d7db1' } };
  return { entries: rustMaterials(target, { productionCandidate: true, distribution,
    architectureReceipt: receipt }).flatMap(record => record.files)
    .map(({ path, size, sha256: digest }) => ({ path, size, sha256: digest })),
  target, receipt, distribution };
}

function validateCommandCandidate(value, options = {}) {
  return validateRustMaterials(value.entries, value.target, value.receipt, '0.2.3',
    { productionCandidate: true, distribution: value.distribution, ...options });
}

test('exact H1 source binding admits both candidate targets with the candidate notice overlay', () => {
  for (const target of ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']) {
    assert.doesNotThrow(() => validateCommandCandidate(commandCandidate(target)));
  }
});

test('H1 candidate binding authenticates current workspace bytes and rejects an unreviewed manifest', () => {
  const inputs = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
    .map(logicalPath => {
      const data = readFileSync(new URL(`../${logicalPath}`, import.meta.url));
      return { logicalPath, size: data.length, sha256: sha256(data) };
    });
  const changed = Buffer.concat([readFileSync(new URL('../Cargo.toml', import.meta.url)),
    Buffer.from('\n# unreviewed manifest change\n')]);
  for (const target of ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']) {
    const value = commandCandidate(target);
    value.receipt.inputs = inputs;
    assert.doesNotThrow(() => validateCommandCandidate(value));
    value.receipt.inputs = inputs.map(input => input.logicalPath === 'Cargo.toml'
      ? { ...input, size: changed.length, sha256: sha256(changed) } : input);
    assert.throws(() => validateCommandCandidate(value), /Rust material lockfile identity/);
  }
});

test('H1 source binding cannot replace tagged or predecessor release lock pins', () => {
  assert.throws(() => validateCommandCandidate(commandCandidate(), { productionCandidate: false }),
    /Rust material lockfile identity/);
  assert.throws(() => validateCommandCandidate(commandCandidate(), { productionCandidate: 'true' }),
    /Rust material lockfile identity/);
  for (const version of ['0.2.1', '0.2.2', '0.2.3']) {
    const value = commandCandidate();
    value.distribution.source.ref = `refs/tags/v${version}`;
    assert.throws(() => validateCommandCandidate(value), /Rust material lockfile identity/);
    assert.throws(() => validateRustMaterials(value.entries, value.target, value.receipt, version,
      { productionCandidate: true, distribution: value.distribution }), /Rust material lockfile identity/);
  }
});

test('candidate material binding rejects source, recipe, version and target substitutions', () => {
  for (const mutate of [
    value => { value.distribution.source.repository = 'https://example.invalid/repository'; },
    value => { value.distribution.source.ref = 'refs/heads/other'; },
    value => { value.distribution.source.commit = '3'.repeat(40); },
    value => { value.distribution.source.tree = '3'.repeat(40); },
    value => { value.distribution.recipe.sha256 = '0'.repeat(64); },
    value => { value.distribution.recipe.format = 'unreviewed'; },
    value => { value.distribution.version = '0.2.4'; },
    value => { value.distribution.target.triple = 'x86_64-pc-windows-msvc'; },
  ]) {
    const value = commandCandidate(); mutate(value);
    assert.throws(() => validateCommandCandidate(value), /Rust material lockfile identity/);
  }
});

test('candidate binding rejects drift in every exact source input and inventory shape', () => {
  for (let index = 0; index < 4; index++) {
    for (const field of ['size', 'sha256', 'logicalPath']) {
      const value = commandCandidate();
      value.receipt.inputs[index][field] = field === 'size' ? 1 : field === 'sha256'
        ? '0'.repeat(64) : 'other';
      assert.throws(() => validateCommandCandidate(value), /Rust material lockfile identity/);
    }
  }
  for (const mutate of [
    inputs => inputs.pop(), inputs => inputs.push({ ...inputs[0] }), inputs => inputs.reverse(),
  ]) {
    const value = commandCandidate(); mutate(value.receipt.inputs);
    assert.throws(() => validateCommandCandidate(value), /Rust material lockfile identity/);
  }
});

test('candidate binding retains exact notice hashes and full material coverage', () => {
  for (const mutate of [
    entries => entries.pop(), entries => { entries[0].sha256 = '0'.repeat(64); },
    entries => entries.push({ ...entries[0] }),
  ]) {
    const value = commandCandidate(); mutate(value.entries);
    assert.throws(() => validateCommandCandidate(value), /Rust notice inventory/);
  }
});

test('candidate-only Rust additions have exact independent upstream crate and notice identities', () => {
  const overlay = readFileSync(new URL(
    '../scripts/distribution/materials-rust-command-candidate-v1.json', import.meta.url));
  assert.equal(sha256(overlay), '499be6581463e71e54ede6d2c5807c346173f34e7505e6faeaae95944c254483');
  for (const [target, count] of [['x86_64-unknown-linux-gnu', 163], ['x86_64-pc-windows-msvc', 170]]) {
    const value = commandCandidate(target);
    const records = rustMaterials(target, { productionCandidate: true,
      distribution: value.distribution, architectureReceipt: value.receipt });
    assert.equal(records.length, count);
    const additions = records.filter(record => ['autocfg', 'memoffset'].includes(record.name));
    assert.deepEqual(additions.map(record => [record.name, record.version, record.crateSha256]), [
      ['autocfg', '1.5.1', 'f2032f911046de80f0a198e0901378627c33f59ea0ac00e363d481118bd70a53'],
      ['memoffset', '0.9.1', '488016bfae457b036d996092f6cb448677611ce4449e970ceaf42695203f218a'],
    ]);
    assert.deepEqual(additions.flatMap(record => record.files).map(file =>
      [file.path, file.size, file.sha256]), [
      ['licenses/rust/autocfg-1.5.1/LICENSE-APACHE', 10847, 'a60eea817514531668d7e00765731449fe14d059d3249e0bc93b36de45f759f2'],
      ['licenses/rust/autocfg-1.5.1/LICENSE-MIT', 1054, '27995d58ad5c1145c1a8cd86244ce844886958a35eb2b78c6b772748669999ac'],
      ['licenses/rust/memoffset-0.9.1/LICENSE', 1055, '3234ac55816264ee7b6c7ee27efd61cf0a1fe775806870e3d9b4c41ea73c5cb1'],
    ]);
    assert(!rustMaterials(target).some(record => ['autocfg', 'memoffset'].includes(record.name)));
    const releaseReceipt = { inputs: [{ logicalPath: 'Cargo.lock',
      sha256: '78733d507b74bd9a343c938285ede13c3a607cae36cdad6d116f6c2b572e14f1' }] };
    assert.throws(() => validateRustMaterials(value.entries, target, releaseReceipt),
      /Rust notice inventory differs/);
  }
});

test('candidate Rust notices reject omitted extra substituted and duplicate inventories', () => {
  for (const mutate of [
    entries => entries.splice(entries.findIndex(file => file.path.includes('/autocfg-')), 1),
    entries => entries.push({ ...entries[0], path: 'licenses/rust/extra/LICENSE' }),
    entries => { entries.find(file => file.path.includes('/memoffset-')).sha256 = '0'.repeat(64); },
    entries => { entries.find(file => file.path.includes('/memoffset-')).size++; },
    entries => entries.push({ ...entries.find(file => file.path.includes('/memoffset-')) }),
  ]) {
    const value = commandCandidate(); mutate(value.entries);
    assert.throws(() => validateCommandCandidate(value), /Rust notice inventory differs/);
  }
  const value = commandCandidate();
  assert(!rustMaterials(value.target, { productionCandidate: false,
    distribution: value.distribution, architectureReceipt: value.receipt })
    .some(record => record.name === 'autocfg'));
  assert.throws(() => rustMaterials(value.target, { productionCandidate: 'true',
    distribution: value.distribution, architectureReceipt: value.receipt }),
  /candidate Rust material source binding/);
});
