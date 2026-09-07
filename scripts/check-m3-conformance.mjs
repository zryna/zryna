import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { lstatSync, readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
export const workspaceRoot = resolve(dirname(scriptPath), '..');
const expectedDigest = '09b5d5b0c9c9d7b49eb903a14ec95c45ed997cfd37148243ba05828e7c426c64';
export function digest(value) {
  return createHash('sha256').update(value).digest('hex');
}
export function validateM3Registry(bytes, root = workspaceRoot) {
  assert(Buffer.byteLength(bytes) <= 64 * 1024, 'bounded registry');
  assert.equal(digest(bytes), expectedDigest, 'M3 registry differs from its frozen oracle');
  const registry = JSON.parse(bytes);
  const ids = new Set();
  const paths = new Set();
  for (const fixture of registry.fixtures) {
    assert.match(fixture.path, /^tests\/m3-fixtures\/conformance\/[a-z0-9-]+\.zry$/);
    assert(!ids.has(fixture.id), 'duplicate fixture id');
    ids.add(fixture.id);
    paths.add(fixture.path.split('/').at(-1));
    let current = root;
    for (const segment of fixture.path.split('/')) {
      current = resolve(current, segment);
      assert(!lstatSync(current).isSymbolicLink(), 'fixture link forbidden');
    }
    assert(lstatSync(current).isFile());
    assert.equal(digest(readFileSync(current)), fixture.sha256, fixture.id);
  }
  assert.deepEqual(readdirSync(resolve(root, 'tests/m3-fixtures/conformance')).sort(), [...paths].sort(),
    'unregistered or missing corpus fixture');
  const cases = new Set();
  for (const entry of [...registry.valid, ...registry.invalid, ...registry.runtimeInvalid]) {
    assert(!cases.has(entry.id), 'duplicate case id');
    cases.add(entry.id);
    assert(ids.has(entry.fixture), 'unregistered fixture');
  }
  for (const evidence of registry.evidence) {
    const source = readFileSync(resolve(root, evidence.path), 'utf8');
    assert(source.includes(`fn ${evidence.test.split('::').at(-1)}(`), evidence.id);
  }
  return registry;
}
export function loadAndValidateM3Conformance() {
  return validateM3Registry(readFileSync(resolve(workspaceRoot, 'tests/m3-conformance-v1.json')));
}
if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  const registry = loadAndValidateM3Conformance();
  console.log(`M3 registry: ${registry.valid.length} valid, ${registry.invalid.length} invalid, ${registry.evidence.length} boundary oracles.`);
}
