// Pure bounded v9 registry inventory, not a package install, source or advisory authority.
import assert from 'node:assert/strict';
const semver = /^(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)\.(?:0|[1-9][0-9]*)(?:-[A-Za-z0-9.-]+)?(?:\+[A-Za-z0-9.-]+)?$/u;
const namePattern = /^(?:@[A-Za-z0-9_.-]+\/)?[A-Za-z0-9_.-]+$/u;
const object = value => value && typeof value === 'object' && !Array.isArray(value);
function locator(key) {
  assert(typeof key === 'string' && key.length <= 4096);
  const base = key.split('(')[0], at = base.lastIndexOf('@');
  assert(at > 0 && at < base.length - 1);
  const name = base.slice(0, at), version = base.slice(at + 1);
  assert(namePattern.test(name) && semver.test(version));
  return { base, name, version };
}
export function npmQueries(lock) {
  assert.equal(String(lock.lockfileVersion), '9.0');
  assert(object(lock.packages) && object(lock.snapshots) && object(lock.importers));
  const keys = Object.keys(lock.packages); assert(keys.length > 0 && keys.length <= 4096);
  const packages = keys.map(key => {
    const item = locator(key); assert.equal(item.base, key, 'package records have ordinary registry locators');
    const resolution = lock.packages[key].resolution;
    assert(object(resolution)); assert.deepEqual(Object.keys(resolution), ['integrity']);
    assert(/^sha512-[A-Za-z0-9+/]{86}==$/u.test(resolution.integrity));
    assert.equal(Buffer.from(resolution.integrity.slice(7), 'base64').length, 64);
    return { package: { ecosystem: 'npm', name: item.name }, version: item.version };
  });
  const present = new Set(keys), used = new Set();
  const references = section => {
    assert(object(section)); assert(Object.keys(section).length <= 4096);
    for (const [name, entry] of Object.entries(section)) {
      const version = typeof entry === 'string' ? entry : entry?.version;
      assert(namePattern.test(name) && typeof version === 'string');
      const baseVersion = version.split('(')[0];
      const target = locator(semver.test(baseVersion) ? `${name}@${version}` : version);
      assert(present.has(target.base), 'no unknown/nonregistry lock reference');
    }
  };
  const snapshots = Object.entries(lock.snapshots); assert(snapshots.length <= 8192);
  for (const [key, snapshot] of snapshots) {
    const item = locator(key); assert(present.has(item.base)); used.add(item.base);
    assert(object(snapshot));
    for (const kind of ['dependencies', 'optionalDependencies']) references(snapshot[kind] ?? {});
  }
  assert.equal(used.size, present.size, 'every package has a snapshot');
  for (const importer of Object.values(lock.importers)) {
    assert(object(importer));
    for (const kind of ['dependencies', 'devDependencies', 'optionalDependencies']) references(importer[kind] ?? {});
  }
  return packages;
}
