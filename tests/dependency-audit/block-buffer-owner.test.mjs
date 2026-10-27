// Pure parser/projection controls; no process, Cargo, compiler or candidate execution.
import assert from 'node:assert/strict';
import { dirname, join, resolve } from 'node:path';
import test from 'node:test';
import { ownerEnvironment, selectOwnerGraph, selectSha2Artifact, TESTS, validateProbe }
  from '../../scripts/stability-gates/dependency-block-buffer-owner.mjs';
const source = 'registry+https://github.com/rust-lang/crates.io-index';
const bytes = value => Buffer.from(JSON.stringify(value) + '\n');
function metadata() {
  const packages = [['sha2', '0.10.9'], ['digest', '0.10.7'], ['block-buffer', '0.10.4']]
    .map(([name, version]) => ({ name, version, source, id: `${source}#${name}@${version}`,
      manifest_path: resolve('registry', `${name}-${version}`, 'Cargo.toml') }));
  return { packages, resolve: { nodes: [
    { id: packages[0].id, deps: [{ name: 'digest', pkg: packages[1].id }] },
    { id: packages[1].id, deps: [{ name: 'block_buffer', pkg: packages[2].id }] },
  ] } };
}
test('metadata binds the actual registry generation and both resolved edges', () => {
  const input = metadata(), graph = selectOwnerGraph(bytes(input));
  assert.equal(graph.blockBuffer.manifestPath, input.packages[2].manifest_path);
  for (const change of [
    d => { d.packages[2].version = '0.12.1'; },
    d => { d.packages[2].source = null; },
    d => { d.packages[2].id += '-forged'; },
    d => { d.packages[2].manifest_path = 'relative/Cargo.toml'; },
    d => { d.packages.push(structuredClone(d.packages[2])); },
    d => { d.resolve.nodes[1].deps = []; },
    d => { d.resolve.nodes[0].deps[0].pkg = d.packages[2].id; },
    d => { d.resolve.nodes.push(structuredClone(d.resolve.nodes[0])); },
  ]) { const bad = structuredClone(input); change(bad); assert.throws(() => selectOwnerGraph(bytes(bad))); }
  assert.throws(() => selectOwnerGraph(Buffer.from(JSON.stringify(input).replace('"packages":', '"packages":[],"packages":') + '\n')));
  assert.throws(() => selectOwnerGraph(Buffer.from([255])));
});
test('Cargo completion and rlib must select the actual sha2 library in the fresh target', () => {
  const graph = selectOwnerGraph(bytes(metadata())), target = resolve('fresh-target');
  const library = join(target, 'debug', 'libsha2.rlib');
  const artifact = { reason: 'compiler-artifact', package_id: graph.sha2.id,
    target: { name: 'sha2', kind: ['lib'], crate_types: ['lib'], src_path: join(dirname(graph.sha2.manifestPath), 'src', 'lib.rs') },
    profile: { test: false, opt_level: '3' }, filenames: [library] };
  const rows = [artifact, { reason: 'build-finished', success: true }];
  const log = data => Buffer.concat(data.map(bytes));
  assert.equal(selectSha2Artifact(log(rows), graph, target), library);
  for (const change of [
    d => { d[0].package_id += '-wrong'; },
    d => { d[0].target.kind = ['bin']; },
    d => { d[0].target.src_path = resolve('foreign.rs'); },
    d => { d[0].filenames = [resolve('old-target', 'debug', 'libsha2.rlib')]; },
    d => { d[0].profile.test = true; },
    d => { d[0].profile.opt_level = '0'; },
    d => { d[1].success = false; },
    d => { d.push(structuredClone(d[0])); },
    d => { d.splice(1, 0, structuredClone(d[0])); },
  ]) { const bad = structuredClone(rows); change(bad); assert.throws(() => selectSha2Artifact(log(bad), graph, target)); }
  assert.throws(() => selectSha2Artifact(log(rows).subarray(0, -1), graph, target));
});
test('only the complete three passing unfiltered probe names satisfy the owner transcript', () => {
  const stdout = `\nrunning 3 tests\n${TESTS.map(name => `test ${name} ... ok`).join('\n')}\n\n` +
    'test result: ok. 3 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n';
  assert.doesNotThrow(() => validateProbe(Buffer.from(stdout), Buffer.alloc(0)));
  for (const bad of [stdout.replace(TESTS[0], TESTS[1]), stdout.replace(' ... ok', ' ... ignored'),
    stdout.replace('3 passed', '2 passed'), stdout.replace('0 ignored', '1 ignored'),
    stdout.replace('0 filtered', '1 filtered'), stdout.replace('0 measured', '1 measured'),
    stdout + 'test unrelated ... ok\n', stdout.replace(`test ${TESTS[0]} ... ok\n`, '')])
    assert.throws(() => validateProbe(Buffer.from(bad), Buffer.alloc(0)));
  assert.throws(() => validateProbe(Buffer.from(stdout), Buffer.from('unaccounted error')));
});
test('owner environment drops credentials, flags and wrappers and fixes offline tool/target inputs', () => {
  const env = { HOME: resolve('home'), CARGO_HOME: resolve('cargo'), RUSTUP_HOME: resolve('rustup'),
    PATH: 'trusted-path', SystemRoot: resolve('Windows'), RUSTFLAGS: '--cfg skip', RUSTC_WRAPPER: 'wrapper',
    RUSTC: 'wrong', CARGO_TARGET_DIR: 'old', CARGO_BUILD_JOBS: '999', SECRET_TOKEN: 'private' };
  const rustc = resolve('toolchain', 'rustc'), target = resolve('target');
  const actual = ownerEnvironment(env, rustc, target);
  for (const key of ['RUSTFLAGS', 'RUSTC_WRAPPER', 'SECRET_TOKEN']) assert(!Object.hasOwn(actual, key));
  assert.equal(actual.RUSTC, rustc); assert.equal(actual.CARGO_TARGET_DIR, target);
  assert.equal(actual.CARGO_BUILD_JOBS, '2'); assert.equal(actual.CARGO_NET_OFFLINE, 'true');
  assert.equal(actual.RUSTUP_TOOLCHAIN, '1.97.1');
  assert.throws(() => ownerEnvironment({ ...env, CARGO_HOME: 'relative' }, rustc, target));
  assert.throws(() => ownerEnvironment({ ...env, PATH: 'bad\0path' }, rustc, target));
});
