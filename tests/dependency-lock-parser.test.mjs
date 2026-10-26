// Synthetic pure objects only: no YAML module, install, Node child, OSV or SDK execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { npmQueries } from '../scripts/stability-gates/dependency-lock-parser.mjs';
const integrity = `sha512-${Buffer.alloc(64, 1).toString('base64')}`;
const fresh = () => ({ lockfileVersion: '9.0',
  packages: { '@scope/pkg@6.0.1': { resolution: { integrity } } },
  snapshots: { '@scope/pkg@6.0.1(patch_hash=synthetic)': {} },
  importers: { '.': { dependencies: { '@scope/pkg': { version: '6.0.1(patch_hash=synthetic)' } } } } });
test('pure npm parser includes exact scoped versions without treating patch hashes as versions', () => {
  assert.deepEqual(npmQueries(fresh()), [{ package: { ecosystem: 'npm', name: '@scope/pkg' }, version: '6.0.1' }]);
});
test('pure npm parser rejects custom tarball Git directory or missing integrity origins', () => {
  for (const resolution of [{ integrity, tarball: 'https://other.invalid/pkg.tgz' },
    { repo: 'git', commit: 'bad' }, { directory: '../local' }, {}, { integrity: 'sha512-short' }]) {
    const lock = fresh(); lock.packages['@scope/pkg@6.0.1'].resolution = resolution;
    assert.throws(() => npmQueries(lock));
  }
});
test('pure npm parser rejects nonregistry unknown or unversioned package and snapshot locators', () => {
  for (const name of ['file:../local', 'https://other.invalid/pkg', '@scope/pkg@git:main', '@scope/pkg@latest']) {
    const lock = fresh(); lock.packages = { [name]: { resolution: { integrity } } };
    assert.throws(() => npmQueries(lock));
  }
  const lock = fresh(); lock.snapshots = { 'unknown@1.0.0': {} }; assert.throws(() => npmQueries(lock));
});
test('pure npm parser refuses importer and transitive nonregistry references omitted from packages', () => {
  for (const value of ['link:../local', 'file:/tmp/pkg', 'git+https://other.invalid/a', '2.0.0']) {
    const lock = fresh(); lock.importers['.'].dependencies['@scope/pkg'].version = value;
    assert.throws(() => npmQueries(lock));
    const transitive = fresh(); Object.values(transitive.snapshots)[0].dependencies = { other: value };
    assert.throws(() => npmQueries(transitive));
  }
});
test('pure npm parser rejects missing snapshots bad lock version and private count overflow', () => {
  for (const mutate of [l => { l.snapshots = {}; }, l => { l.lockfileVersion = '8.0'; },
    l => { l.importers = null; }, l => { l.packages = Array(4097).fill({}); }]) {
    const lock = fresh(); mutate(lock); assert.throws(() => npmQueries(lock));
  }
});
test('pure npm parser audits unused retained registry package records too', () => {
  const lock = fresh(); lock.packages['extra@1.2.3'] = { resolution: { integrity } };
  lock.snapshots['extra@1.2.3'] = {};
  assert.equal(npmQueries(lock).length, 2);
});
