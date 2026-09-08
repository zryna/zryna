import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { validateFixture } from '../../scripts/package-release/validate.mjs';
import { bytes, digest, parseCanonical } from '../../scripts/package-release/canonical.mjs';
import { bindGraph, bindRelease, makeFixture, recordHash, wire } from './builders.mjs';

const fixtureBytes = () => readFileSync(new URL('./valid.json', import.meta.url));
const accepts = fixture => validateFixture(wire(fixture));
const rejects = (fixture, code) => assert.throws(() => accepts(fixture), new RegExp(code));

test('static independent fixture has fixed wire, lock and release outcomes', () => {
  const input = fixtureBytes();
  const expected = JSON.parse(readFileSync(new URL('./expected.json', import.meta.url)));
  assert.deepEqual(validateFixture(input), expected);
  assert.deepEqual(wire(makeFixture()), input);
  assert.deepEqual(bytes({ z: 1, a: ['x', { c: true, b: false }] }),
    Buffer.from('{"a":["x",{"b":false,"c":true}],"z":1}\n'));
  assert.equal(digest('lock', { a: 1 }), recordHash('lock', { a: 1 }));
});

test('malformed and noncanonical wire rejects before graph interpretation', () => {
  const text = fixtureBytes().toString();
  for (const input of [
    '', '{', 'null', '[]', text.trimEnd(), '\ufeff' + text, text.replace(/\n$/, '\r\n'),
    text.replace('{"format":', '{"format":"duplicate","format":'),
    text.replace('"format"', '"\\u0066ormat"'), text.replace('"sourceDateEpoch":0', '"sourceDateEpoch":0.0'),
    JSON.stringify(JSON.parse(text), null, 2) + '\n',
  ]) assert.throws(() => validateFixture(Buffer.from(input)), /P168-(WIRE|SCHEMA)/);
  assert.throws(() => validateFixture(Buffer.from([0xff])), /P168-WIRE/);
  const parsed = JSON.parse(text);
  const reversed = Object.fromEntries(Object.entries(parsed).reverse());
  assert.throws(() => validateFixture(Buffer.from(JSON.stringify(reversed) + '\n')), /P168-WIRE/);
});

test('unknown and missing fields reject at every object shape', () => {
  const fixture = JSON.parse(fixtureBytes());
  const paths = [];
  function visit(value, path = []) {
    if (Array.isArray(value)) value.forEach((item, index) => visit(item, [...path, index]));
    else if (value && typeof value === 'object') {
      paths.push(path);
      for (const [key, item] of Object.entries(value)) visit(item, [...path, key]);
    }
  }
  visit(fixture);
  for (const path of paths) {
    const extra = structuredClone(fixture);
    path.reduce((value, key) => value[key], extra).unexpected = true;
    rejects(extra, 'P168-SCHEMA');
    const missing = structuredClone(fixture);
    const item = path.reduce((value, key) => value[key], missing);
    for (const key of Object.keys(item)) {
      const saved = item[key];
      delete item[key];
      rejects(missing, 'P168-SCHEMA');
      item[key] = saved;
    }
  }
});

const negativeCases = [
  ['material tampering', 'P168-INTEGRITY', f => { f.materials[0].content += 'x'; }],
  ['source size', 'P168-INTEGRITY', f => { f.manifests[0].files[0].size++; }],
  ['source digest', 'P168-INTEGRITY', f => { f.lock.packages[0].sourceSha256 = 'f'.repeat(64); }],
  ['lock binding', 'P168-INTEGRITY', f => { f.release.lockSha256 = 'f'.repeat(64); }],
  ['SBOM omission', 'P168-SBOM', f => { f.release.sbom.packages.pop(); }],
  ['SBOM edge', 'P168-SBOM', f => { f.release.sbom.edges = []; }],
  ['material provenance', 'P168-PROVENANCE', f => { f.release.provenance.materials.pop(); }],
  ['recipe binding', 'P168-INTEGRITY', f => { f.release.reproduction.recipe.arguments.push('x'); }],
  ['environment binding', 'P168-INTEGRITY', f => { f.release.reproduction.environment.os = 'windows-x86_64'; }],
  ['tool inventory', 'P168-COMPATIBILITY', f => {
    f.release.reproduction.environment.compilerSha256 = 'f'.repeat(64); bindRelease(f);
  }],
  ['undeclared tool', 'P168-REPRODUCTION', f => {
    f.release.reproduction.recipe.executable = 'other'; bindRelease(f);
  }],
  ['reproduction mismatch', 'P168-REPRODUCTION', f => { f.release.reproduction.observed[0].size++; }],
  ['reproduction expected mismatch', 'P168-REPRODUCTION', f => { f.release.reproduction.expected[0].size++; }],
  ['note subject', 'P168-INTEGRITY', f => { f.release.notes.subjectSha256 = 'f'.repeat(64); }],
  ['note text payload', 'P168-INTEGRITY', f => { f.release.notes.text += ' altered'; }],
  ['signature payload', 'P168-INTEGRITY', f => { f.release.notes.signatures[0].payloadSha256 = 'f'.repeat(64); }],
  ['rollback source', 'P168-ROLLBACK', f => { f.release.rollback.from = 'f'.repeat(64); }],
  ['rollback same release', 'P168-ROLLBACK', f => { f.release.rollback.to = f.release.rollback.from; }],
  ['wrong tag', 'P168-RELEASE', f => { f.release.tag = 'v0.2.0'; }],
  ['false signature claim', 'P168-SCHEMA', f => { f.release.notes.signatures[0].status = 'verified'; }],
  ['network fallback', 'P168-SCHEMA', f => { f.release.reproduction.environment.network = 'enabled'; }],
];
for (const [name, code, mutate] of negativeCases) {
  test(name, () => {
    const fixture = makeFixture();
    mutate(fixture);
    rejects(fixture, code);
  });
}

test('arrays must retain explicit order and uniqueness', () => {
  const arrays = [
    f => f.manifests, f => f.lock.packages, f => f.materials,
    f => f.lock.compatibility.targets, f => f.manifests[0].compatibility.targets,
    f => f.release.reproduction.environment.variables,
  ];
  for (const select of arrays) {
    for (const duplicate of [false, true]) {
      const fixture = makeFixture();
      const items = select(fixture);
      if (duplicate) items[1] = structuredClone(items[0]);
      else items.reverse();
      rejects(fixture, 'P168-ORDER|P168-LOCK');
    }
  }
});

test('exact dependency selection rejects stale, forged, cyclic and orphan graphs', () => {
  const fixture = makeFixture();
  fixture.lock.packages.find(pkg => pkg.dependencies.length).dependencies[0].package = fixture.lock.root;
  rejects(fixture, 'P168-LOCK');
  for (const mutate of [
    f => { f.manifests.find(m => m.dependencies.length).dependencies[0].version = '0.2.0'; },
    f => { f.manifests.find(m => m.dependencies.length).dependencies[0].source.revision = 'f'.repeat(40); },
  ]) {
    const f = makeFixture();
    mutate(f);
    f.manifests.sort((a, b) => recordHash('manifest', a).localeCompare(recordHash('manifest', b)));
    f.lock.packages = f.manifests.map(m => ({
      id: recordHash('manifest', m), sourceSha256: recordHash('source-files', m.files), dependencies: [],
    }));
    f.lock.root = recordHash('manifest', f.manifests.find(m => m.name === 'package-01'));
    rejects(f, 'P168-RESOLUTION');
  }
  const cyclic = makeFixture();
  const leaf = cyclic.manifests.find(m => m.name === 'package-00');
  const root = cyclic.manifests.find(m => m.name === 'package-01');
  leaf.dependencies.push({ alias: root.name, name: root.name, version: root.version, source: root.source });
  bindGraph(cyclic, root.name);
  rejects(cyclic, 'P168-GRAPH');
  const orphan = makeFixture();
  orphan.manifests.forEach(m => { m.dependencies = []; });
  bindGraph(orphan, 'package-01');
  rejects(orphan, 'P168-GRAPH');
});

test('compatibility and portable sources fail closed after self-consistent rebinding', () => {
  for (const mutate of [
    m => { m.compatibility.compiler = '0.2.0'; },
    m => { m.compatibility.profile = 'data-ownership-v1'; },
    m => { m.compatibility.targets = ['javascript']; },
  ]) {
    const f = makeFixture();
    mutate(f.manifests.find(m => m.name === 'package-00'));
    bindGraph(f, 'package-01');
    rejects(f, 'P168-COMPATIBILITY');
  }
  for (const bad of ['../escape', 'src//x', 'con.txt', 'a/../b', 'a./b']) {
    const f = makeFixture();
    f.manifests[0].files[0].path = bad;
    rejects(f, bad === '../escape' ? 'P168-SCHEMA' : 'P168-PATH');
  }
  for (const locator of ['http://example.invalid/sample.git', 'https://user@example.invalid/a.git',
    'https://example.invalid/a.git?branch=main', 'https://EXAMPLE.invalid/a.git']) {
    const f = makeFixture();
    const leaf = f.manifests.find(m => m.source.kind === 'git');
    leaf.source.locator = locator;
    f.manifests.find(m => m.dependencies.length).dependencies[0].source.locator = locator;
    bindGraph(f, 'package-01');
    rejects(f, 'P168-SOURCE');
  }
});

test('native compatibility is a declaration, never target execution', () => {
  const f = makeFixture();
  for (const m of f.manifests) m.compatibility.targets = ['native-linux-x86_64'];
  f.lock.compatibility.targets = ['native-linux-x86_64'];
  bindGraph(f, 'package-01');
  assert.equal(accepts(f).reproduction, 'not-executed');
  f.release.reproduction.environment.os = 'windows-x86_64';
  bindRelease(f);
  rejects(f, 'P168-COMPATIBILITY');
});

test('schema and canonical failures recover without mutating supplied bytes', () => {
  const input = fixtureBytes();
  const before = Buffer.from(input);
  for (let i = 0; i < 3; i++) {
    assert.throws(() => parseCanonical(Buffer.from('{"x":1,"x":2}\n')), /P168-WIRE/);
    assert.equal(validateFixture(input).status, 'contract-fixture-valid');
    assert.deepEqual(input, before);
  }
});

test('wrong primitive types reject independently throughout the fixture', () => {
  const fixture = makeFixture();
  const paths = [];
  function visit(value, path = []) {
    if (Array.isArray(value)) value.forEach((child, i) => visit(child, [...path, i]));
    else if (value && typeof value === 'object') {
      for (const [key, child] of Object.entries(value)) visit(child, [...path, key]);
    } else paths.push(path);
  }
  visit(fixture);
  for (const path of paths) {
    const copy = structuredClone(fixture);
    const parent = path.slice(0, -1).reduce((value, key) => value[key], copy);
    const key = path.at(-1);
    parent[key] = typeof parent[key] === 'string' ? 1 : '1';
    rejects(copy, 'P168-SCHEMA');
  }
  for (const value of [-1, 0.5, 4294967296]) {
    const copy = makeFixture();
    copy.release.provenance.sourceDateEpoch = value;
    rejects(copy, 'P168-SCHEMA');
  }
});

test('source trees, selection ambiguity and frozen lock omissions reject', () => {
  const collision = makeFixture();
  const file = collision.manifests[0].files[0];
  collision.manifests[0].files = [{ ...file, path: 'a' }, { ...file, path: 'a/b' }];
  rejects(collision, 'P168-PATH');
  const ambiguous = makeFixture();
  const duplicate = structuredClone(ambiguous.manifests.find(m => m.name === 'package-00'));
  duplicate.files[0].path = 'other.zry';
  ambiguous.manifests.push(duplicate);
  bindGraph(ambiguous, 'package-01');
  rejects(ambiguous, 'P168-RESOLUTION');
  const missing = makeFixture();
  missing.lock.packages.pop();
  rejects(missing, 'P168-LOCK');
  const absentMaterial = makeFixture();
  absentMaterial.materials = absentMaterial.materials.filter(item =>
    item.sha256 !== absentMaterial.manifests[0].files[0].sha256);
  bindRelease(absentMaterial);
  rejects(absentMaterial, 'P168-INTEGRITY');
});

test('two aliases may resolve to one exact dependency without type-identity claims', () => {
  const f = makeFixture();
  const root = f.manifests.find(m => m.dependencies.length);
  root.dependencies.push({ ...root.dependencies[0], alias: 'second' });
  bindGraph(f, 'package-01');
  accepts(f);
  root.dependencies.reverse();
  bindGraph(f, 'package-01');
  rejects(f, 'P168-ORDER');
});

test('environment, local revisions and moving Git refs are not repaired', () => {
  for (const mutate of [
    f => { f.release.reproduction.environment.compilerVersion = '0.2.0'; },
    f => { f.release.reproduction.environment.toolchains[0].version = '0.2.0'; },
  ]) {
    const f = makeFixture();
    mutate(f);
    bindRelease(f);
    rejects(f, 'P168-COMPATIBILITY');
  }
  const epoch = makeFixture();
  epoch.release.provenance.sourceDateEpoch = 1;
  bindRelease(epoch);
  rejects(epoch, 'P168-REPRODUCTION');
  for (const source of [
    { kind: 'local', locator: '../outside', revision: '' },
    { kind: 'local', locator: 'packages/x', revision: 'a'.repeat(40) },
    { kind: 'git', locator: 'https://example.invalid/x.git', revision: '' },
  ]) {
    const f = makeFixture(1);
    f.manifests[0].source = source;
    bindGraph(f, 'package-00');
    rejects(f, 'P168-SOURCE');
  }
  const branch = makeFixture(1);
  branch.manifests[0].source.revision = 'main';
  rejects(branch, 'P168-SCHEMA');
});

test('ordered release arrays reject duplicates and changed projections', () => {
  for (const mutate of [
    f => { f.release.checksums.push(structuredClone(f.release.checksums[0])); },
    f => { f.release.sbom.packages.reverse(); },
    f => { f.release.sbom.edges[0].to = f.lock.root; },
    f => { f.release.provenance.materials.reverse(); },
    f => { f.release.notes.signatures.push(structuredClone(f.release.notes.signatures[0])); },
    f => { f.release.reproduction.environment.toolchains.push(
      structuredClone(f.release.reproduction.environment.toolchains[0])); },
  ]) {
    const f = makeFixture();
    mutate(f);
    rejects(f, 'P168-(ORDER|SBOM|PROVENANCE)');
  }
});

test('contract links and routed CI retain the dedicated fixture gate', () => {
  const root = new URL('../../', import.meta.url);
  const spec = readFileSync(new URL('spec/package/PACKAGE_RELEASE_V1.md', root), 'utf8');
  for (const match of spec.matchAll(/\]\((\.\.\/[^)#]+)(?:#[^)]*)?\)/g)) {
    assert.ok(readFileSync(new URL(match[1], new URL('spec/package/', root))).length > 0);
  }
  for (const issue of [168, 167, 356, 360, 361, 362]) {
    assert.ok(spec.includes('https://github.com/zryna/zryna/issues/' + issue));
  }
  const pkg = JSON.parse(readFileSync(new URL('package.json', root)));
  assert.equal(pkg.scripts['package:contract'],
    'node --test tests/package-release-v1/validation.test.mjs tests/package-release-v1/boundaries.test.mjs tests/package-source-trust.test.mjs');
  const workflow = readFileSync(new URL('.github/workflows/ci.yml', root), 'utf8');
  assert.match(workflow,
    /if: needs\.route-contracts\.outputs\.package_release == 'true'/);
  assert.match(workflow, /os: \[ubuntu-latest, windows-latest\]/);
  assert.match(workflow, /pnpm install --frozen-lockfile/);
  assert.match(workflow, /pnpm package:contract/);
});
test('ASCII fields reject trailing line breaks and non-ASCII text', () => {
  for (const text of ['name\n', 'name\r', 'name\u2028', 'caf\u00e9']) {
    const f = makeFixture();
    f.manifests[0].name = text;
    rejects(f, 'P168-SCHEMA');
    const note = makeFixture();
    note.release.notes.text = text;
    rejects(note, 'P168-SCHEMA');
  }
});
