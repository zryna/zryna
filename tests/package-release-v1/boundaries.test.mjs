import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import Ajv from 'ajv';
import { validateFixture } from '../../scripts/package-release/validate.mjs';
import { bindGraph, bindRelease, checksum, makeFixture, material, wire } from './builders.mjs';

const schema = JSON.parse(readFileSync(
  new URL('../../schemas/zryna-package-release-v1.schema.json', import.meta.url)));
const ajv = new Ajv({ strict: true });
const accepts = f => validateFixture(wire(f));
const rejected = f => assert.throws(() => accepts(f), /P168-(SCHEMA|BUDGET)/);
const sample = node => {
  if (node.$ref) return sample(schema.$defs[node.$ref.split('/').at(-1)]);
  if ('const' in node) return node.const;
  if (node.enum) return node.enum[0];
  if (node.type === 'object') return Object.fromEntries(
    Object.entries(node.properties).map(([key, child]) => [key, sample(child)]));
  if (node.type === 'array') return Array.from({ length: node.minItems }, () => sample(node.items));
  if (node.type === 'integer') return node.minimum;
  if (node.pattern?.includes('{64}')) return 'a'.repeat(64);
  if (node.pattern?.includes('{40}')) return 'a'.repeat(40);
  if (node.pattern?.includes('{128}')) return 'a'.repeat(128);
  if (node.pattern?.includes('[1-9]')) return node.pattern.startsWith('^v') ? 'v1.0.0' : '1.0.0';
  if (node.pattern?.includes('[A-Z]')) return 'A';
  return 'a';
};

// Independent, reviewable budget inventory; deleting/raising a schema limit fails.
const limits = {
  'root.manifests': 16, 'root.materials': 64,
  digest: 64, commit: 40, name: 64, version: 14, path: 96,
  'source.locator': 96, 'source.revision': 40, 'compatibility.targets': 3,
  'checksum.size': 1024, 'manifest.files': 16, 'manifest.dependencies': 8,
  'package.dependencies': 8, 'lock.packages': 16, 'material.content': 1024,
  'sbomPackage.license': 64, 'sbom.packages': 16, 'sbom.edges': 128,
  'variable.name': 64, 'variable.value': 96, 'environment.toolchains': 8,
  'environment.variables': 8, 'recipe.arguments': 16, 'recipe.arguments[]': 96,
  'reproduction.expected': 16, 'reproduction.observed': 16,
  'provenance.materials': 64, 'provenance.sourceDateEpoch': 4294967295,
  'signature.signature': 128, 'notes.text': 1024, 'notes.signatures': 4,
  'rollback.reason': 1024, 'release.tag': 15, 'release.checksums': 16,
};
const found = {};
function inspect(node, path) {
  const limit = node.maxItems ?? node.maxLength ?? node.maximum;
  if (limit !== undefined) found[path] = { node, limit };
  for (const [key, child] of Object.entries(node.properties ?? {})) inspect(child, path + '.' + key);
  if (node.items) inspect(node.items, path + '[]');
}
inspect(schema, 'root');
for (const [name, node] of Object.entries(schema.$defs)) inspect(node, name);

test('schema budget inventory cannot silently drift', () => {
  assert.deepEqual(Object.fromEntries(Object.entries(found).map(([key, value]) => [key, value.limit])), limits);
});

for (const [path, { node, limit }] of Object.entries(found)) {
  test('schema exact and first-extra: ' + path, () => {
    const check = ajv.compile({ ...node, $defs: schema.$defs });
    let exact;
    let extra;
    if (node.type === 'array') {
      exact = Array.from({ length: limit }, () => sample(node.items));
      extra = [...exact, sample(node.items)];
    } else if (node.type === 'integer') {
      exact = limit;
      extra = limit + 1;
    } else {
      exact = path === 'version' ? '9999.9999.9999' :
        path === 'release.tag' ? 'v9999.9999.9999' :
        (path === 'variable.name' ? 'A' : 'a').repeat(limit);
      extra = exact + 'a';
    }
    assert.equal(check(exact), true, JSON.stringify(check.errors));
    assert.equal(check(extra), false, path + ' must reject limit + 1');
    assert.equal(check(null), false);
  });
}

test('independent graph fixture reaches 16 packages, eight dependencies and depth six', () => {
  const f = makeFixture(16);
  assert.equal(accepts(f).status, 'contract-fixture-valid');
  assert.equal(Math.max(...f.manifests.map(m => m.dependencies.length)), 8);
  rejected(makeFixture(17));
  const extraEdge = makeFixture(16);
  const root = extraEdge.manifests.find(m => m.name === 'package-15');
  root.dependencies.push(structuredClone(root.dependencies[0]));
  rejected(extraEdge);
  const depth = makeFixture();
  depth.manifests[0].dependencies.push({
    alias: 'x', name: 'x', version: '0.1.0',
    source: { kind: 'local', locator: 'x', revision: '', extra: {} },
  });
  assert.throws(() => accepts(depth), /P168-BUDGET/);
});

test('source files and artifact inventories accept 16 and reject 17', () => {
  for (const source of [true, false]) {
    const f = makeFixture();
    const content = source ? 'synthetic source fixture\n' : 'synthetic artifact fixture\n';
    const entries = Array.from({ length: 16 }, (_, i) =>
      checksum(`files/p${String(i).padStart(2, '0')}.txt`, content));
    if (source) {
      f.manifests[0].files = entries;
      bindGraph(f, 'package-01');
    } else {
      f.release.checksums = entries;
      f.release.reproduction.expected = structuredClone(entries);
      f.release.reproduction.observed = structuredClone(entries);
      bindRelease(f);
    }
    accepts(f);
    entries.push(checksum('files/p16.txt', content));
    rejected(f);
  }
});

test('material count, material bytes and checksum size are independently bounded', () => {
  const f = makeFixture();
  while (f.materials.length < 64) f.materials.push(material('m' + f.materials.length));
  f.materials.sort((a, b) => a.sha256 < b.sha256 ? -1 : 1);
  bindRelease(f);
  accepts(f);
  f.materials.push(material('extra'));
  rejected(f);

  const size = makeFixture();
  const text = 'x'.repeat(1024);
  size.materials.push(material(text));
  size.materials.sort((a, b) => a.sha256 < b.sha256 ? -1 : 1);
  size.release.checksums = [checksum('artifact.txt', text)];
  size.release.reproduction.expected = structuredClone(size.release.checksums);
  size.release.reproduction.observed = structuredClone(size.release.checksums);
  bindRelease(size);
  accepts(size);
  size.materials.find(m => m.content === text).content += 'x';
  rejected(size);
});

test('wire budget accepts 65536 exact bytes and rejects the first extra byte', () => {
  const f = makeFixture(1);
  // Fill independently declared fixture materials; no filesystem or build is used.
  while (f.materials.length < 63) f.materials.push(material('m' + f.materials.length + 'x'.repeat(830)));
  const filler = material('');
  f.materials.push(filler);
  function bind() {
    f.materials.sort((a, b) => a.sha256 < b.sha256 ? -1 : 1);
    bindRelease(f);
  }
  bind();
  let remaining = 65536 - wire(f).length;
  for (const item of f.materials) {
    if (!item.content.startsWith('m')) continue;
    const add = Math.min(1024 - item.content.length, Math.max(0, remaining));
    Object.assign(item, material(item.content + 'x'.repeat(add)));
    remaining -= add;
  }
  bind();
  assert.equal(wire(f).length, 65536);
  accepts(f);
  assert.throws(() => validateFixture(Buffer.concat([wire(f), Buffer.from(' ')])), /P168-BUDGET/);
});

test('key, environment, recipe and numeric limits retain full fixture bindings', () => {
  const f = makeFixture();
  f.release.notes.signatures = Array.from({ length: 4 }, (_, i) => ({
    ...f.release.notes.signatures[0], keyId: String(i).repeat(64),
  }));
  f.release.reproduction.recipe.arguments = Array(16).fill('x'.repeat(96));
  const env = f.release.reproduction.environment;
  while (env.toolchains.length < 8) env.toolchains.push({
    ...env.toolchains[0], name: 'tool-' + env.toolchains.length,
  });
  while (env.variables.length < 8) env.variables.push({
    name: 'VARIABLE_' + env.variables.length, value: 'x'.repeat(96),
  });
  env.variables.sort((a, b) => a.name < b.name ? -1 : 1);
  f.release.provenance.sourceDateEpoch = 4294967295;
  env.variables.find(v => v.name === 'SOURCE_DATE_EPOCH').value = '4294967295';
  f.release.notes.text = 'x'.repeat(1024);
  f.release.rollback.reason = 'x'.repeat(1024);
  bindRelease(f);
  accepts(f);
  for (const select of [
    g => g.release.notes.signatures,
    g => g.release.reproduction.recipe.arguments,
    g => g.release.reproduction.environment.toolchains,
    g => g.release.reproduction.environment.variables,
  ]) {
    const copy = structuredClone(f);
    const items = select(copy);
    items.push(structuredClone(items[0]));
    rejected(copy);
  }
  f.release.provenance.sourceDateEpoch++;
  rejected(f);
});
