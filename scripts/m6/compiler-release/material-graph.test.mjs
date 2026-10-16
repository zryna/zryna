import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { selectCompilerGraph } from './material-graph.mjs';
import { compilerLockPackages } from './material-lock.mjs';
import { selectCompilerCrates, MATERIAL_PINS, captureCompilerMaterials } from './materials.mjs';
import { sha256 } from '../../distribution/canonical.mjs';
import { graphFixture } from './material-fixtures.mjs';
import { compilerTargetMatches } from './material-target.mjs';

test('compiler graph selects sole app normal/build closure and both independent descriptor pins remain unchanged', () => {
  const input = graphFixture();
  const selected = selectCompilerGraph(input);
  assert.equal(selected.external.length, 1);
  assert.equal(selected.external[0].pin.name, 'autocfg');
  assert.equal(selected.proof.selectedNodeCount, 2);
  const cargoTree=Buffer.from('zryna-playground-compiler v0.1.0 (/synthetic-source/apps/zryna-playground-compiler)\nautocfg v1.5.1\n');
  assert.equal(selectCompilerCrates(input.metadata, input.sourceRoot,cargoTree)[0].manifest_path,
    '/synthetic-cache/autocfg-1.5.1/Cargo.toml');
  assert.throws(()=>selectCompilerCrates(input.metadata,input.sourceRoot),/tree byte budget/);
  for (const [name, pin] of [['materials-rust-v1.json', MATERIAL_PINS.base],
    ['materials-rust-command-candidate-v1.json', MATERIAL_PINS.additions]]) {
    assert.equal(sha256(readFileSync(new URL(`../../distribution/${name}`, import.meta.url))), pin);
  }
});

test('development and explicit other-target edges do not enter the fixed production closure', () => {
  const input = graphFixture();
  const other = { id: 'other', name: 'unapproved-dev-fixture', version: '9.9.9', source: 'git+invalid' };
  input.metadata.packages.push(other);
  input.metadata.resolve.nodes[0].deps.push({ pkg: 'other', dep_kinds: [
    { kind: 'dev', target: null }, { kind: null, target: 'aarch64-unknown-linux-gnu' },
  ] });
  assert.equal(selectCompilerGraph(input).external.length, 1);
});

test('build dependencies are mandatory and cycles terminate with one selected identity', () => {
  const input = graphFixture();
  input.metadata.resolve.nodes[0].deps[0].dep_kinds[0].kind = 'build';
  input.metadata.resolve.nodes[1].deps.push({ pkg: 'app', dep_kinds: [{ kind: null, target: null }] });
  assert.equal(selectCompilerGraph(input).proof.selectedNodeCount, 2);
});

test('fixed Linux x86 target handles ordinary cfg expressions and refuses unknown cfg authority', () => {
  for (const condition of [null, 'x86_64-unknown-linux-gnu', 'cfg(unix)',
    'cfg(all(target_os = "linux", target_arch = "x86_64"))',
    'cfg(any(windows, not(target_env="musl")))', 'cfg(target_has_atomic="64")']) {
    assert.equal(compilerTargetMatches(condition), true);
  }
  for (const condition of ['aarch64-unknown-linux-gnu', 'cfg(windows)', 'cfg(target_os="macos")',
    'cfg(all(unix, target_pointer_width="32"))']) assert.equal(compilerTargetMatches(condition), false);
  for (const condition of ['cfg(feature="unapproved")', 'cfg(custom_flag)', 'cfg(not())',
    'cfg(unix) trailing', 'cfg(all(unix)']) assert.throws(() => compilerTargetMatches(condition));
  const input = graphFixture();
  input.metadata.packages.push({ id: 'windows-only', name: 'unapproved', version: '1.0.0' });
  input.metadata.resolve.nodes[0].deps.push({ pkg: 'windows-only', dep_kinds: [{ kind: null, target: 'cfg(windows)' }] });
  assert.equal(selectCompilerGraph(input).external.length, 1);
});

test('locked rustix Linux raw dependencies retain their exact non-Miri no-override recipe', () => {
  // Actual rustix normal-dependency condition retained by Cargo --filter-platform on this lock.
  const condition = 'cfg(all(not(rustix_use_libc), not(miri), target_os = "linux", '
    + 'any(target_endian = "little", any(target_arch = "s390x", target_arch = "powerpc")), '
    + 'any(target_arch = "arm", all(target_arch = "aarch64", target_pointer_width = "64"), '
    + 'target_arch = "riscv64", all(rustix_use_experimental_asm, target_arch = "powerpc"), '
    + 'all(rustix_use_experimental_asm, target_arch = "powerpc64"), '
    + 'all(rustix_use_experimental_asm, target_arch = "s390x"), '
    + 'all(rustix_use_experimental_asm, target_arch = "mips"), '
    + 'all(rustix_use_experimental_asm, target_arch = "mips32r6"), '
    + 'all(rustix_use_experimental_asm, target_arch = "mips64"), '
    + 'all(rustix_use_experimental_asm, target_arch = "mips64r6"), '
    + 'target_arch = "x86", all(target_arch = "x86_64", target_pointer_width = "64"))))';
  assert.equal(compilerTargetMatches(condition), true);
  const input = graphFixture();
  input.metadata.resolve.nodes[0].deps[0].dep_kinds[0].target = condition;
  assert.equal(selectCompilerGraph(input).external.length, 1);
  for (const flag of ['miri', 'rustix_use_libc', 'rustix_use_experimental_asm']) {
    assert.equal(compilerTargetMatches(`cfg(${flag})`), false);
    assert.equal(compilerTargetMatches(`cfg(not(${flag}))`), true);
  }
  for (const condition of ['cfg(rustix_unreviewed)', 'cfg(miri="true")',
    'cfg(rustix_use_libc="true")', 'cfg(all(unix, unknown_flag))']) {
    assert.throws(() => compilerTargetMatches(condition));
  }
});

for (const [name, mutate] of [
  ['duplicate package id', x => x.metadata.packages.push(x.metadata.packages[1])],
  ['duplicate node id', x => x.metadata.resolve.nodes.push(x.metadata.resolve.nodes[1])],
  ['missing selected node', x => x.metadata.resolve.nodes.pop()],
  ['unknown selected package', x => { x.metadata.resolve.nodes[0].deps[0].pkg = 'missing'; }],
  ['unknown edge kind', x => { x.metadata.resolve.nodes[0].deps[0].dep_kinds[0].kind = 'runtime'; }],
  ['duplicate selected dependency', x => x.metadata.resolve.nodes[0].deps.push(x.metadata.resolve.nodes[0].deps[0])],
  ['nonworkspace app', x => { x.metadata.workspace_members = []; }],
  ['wrong app manifest', x => { x.metadata.packages[0].manifest_path = '/outside/Cargo.toml'; }],
  ['source root substitution', x => { x.metadata.workspace_root = '/elsewhere'; }],
  ['nonregistry source', x => { x.metadata.packages[1].source = 'git+https://foreign.invalid'; }],
  ['unknown approved crate', x => { x.approved.clear(); }],
  ['lock checksum substitution', x => { x.cargoLock = Buffer.from(x.cargoLock.toString().replace(/f2032f9/, 'a2032f9')); }],
  ['lock source substitution', x => { x.cargoLock = Buffer.from(x.cargoLock.toString().replace('registry+', 'foreign+')); }],
  ['ambiguous lock package', x => { x.cargoLock = Buffer.concat([x.cargoLock, x.cargoLock.subarray(x.cargoLock.indexOf('[[package]]'))]); }],
  ['external cache path substitution', x => { x.metadata.packages[1].manifest_path = '/outside/wrong/Cargo.toml'; }],
]) {
  test(`compiler graph rejects ${name}`, () => {
    const input = graphFixture(); mutate(input); assert.throws(() => selectCompilerGraph(input));
  });
}

test('Cargo.lock parser rejects duplicate fields unsupported TOML malformed arrays and invalid UTF8', () => {
  const good = graphFixture().cargoLock;
  assert.equal(compilerLockPackages(good).size, 2);
  for (const bad of [Buffer.concat([good, Buffer.from('checksum = "f"\n')]),
    Buffer.from(good.toString().replace('version = 4', 'version = 3')),
    Buffer.concat([good, Buffer.from('dependencies = [\n "x",\n')]),
    Buffer.concat([good, Buffer.from('[metadata]\nx = 1\n')]), Buffer.from([0xff])]) {
    assert.throws(() => compilerLockPackages(bad));
  }
});

test('public capture rejects fabricated source, missing captures and metadata-only adjacent authorization', () => {
  const graph = graphFixture();
  const source = { repository: 'https://github.com/zryna/zryna', commit: '1'.repeat(40),
    tree: '2'.repeat(40), sourceDateEpoch: 1700000000 };
  const cargoTree=Buffer.from('zryna-playground-compiler v0.1.0 (/synthetic-source/apps/zryna-playground-compiler)\nautocfg v1.5.1\n');
  const input = { ...graph, source, cargoTree, crateArchives: [], upstreamLicense: null };
  delete input.approved;
  assert.throws(() => captureCompilerMaterials(input), /capture count/);
  assert.throws(() => captureCompilerMaterials({ ...input, source: { ...source, ref: 'fake' } }));
  assert.throws(() => captureCompilerMaterials({ ...input, extraApproval: true }));
});
