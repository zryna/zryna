import assert from 'node:assert/strict';
import test from 'node:test';
import { validateCompilerTree } from './material-tree.mjs';
import { graphFixture } from './material-fixtures.mjs';

const root = 'zryna-playground-compiler v0.1.0 (/synthetic-source/apps/zryna-playground-compiler)';
const valid = `${root}\nautocfg v1.5.1\n`;
const validate = text => {
  const f = graphFixture();
  return validateCompilerTree(Buffer.from(text), f.metadata, f.sourceRoot);
};

test('tree identity proof matches sole app normal/build graph without claiming compilation units', () => {
  const proof = validate(valid);
  assert.equal(proof.packageCount, 2);
  assert.equal(proof.registryPackageCount, 1);
  assert.match(proof.identitySetSha256, /^[0-9a-f]{64}$/);
  const duplicate = validate(`${valid}[build-dependencies]\nautocfg v1.5.1 (*)\n\n`
    + '(*) - dependencies omitted (already shown)\n');
  assert.equal(duplicate.identitySetSha256, proof.identitySetSha256);
  assert.notEqual(duplicate.stdoutSha256, proof.stdoutSha256);
});

test('proc macro closed annotation matches metadata kind', () => {
  const f = graphFixture();
  f.metadata.packages[1].targets = [{ kind: ['proc-macro'] }];
  assert.equal(validateCompilerTree(Buffer.from(`${root}\nautocfg v1.5.1 (proc-macro)\n`),
    f.metadata, f.sourceRoot).packageCount, 2);
  assert.throws(() => validate(`${root}\nautocfg v1.5.1 (proc-macro)\n`));
});

for (const [name, text] of [
  ['missing dependency', `${root}\n`],
  ['extra dev dependency', `${valid}unapproved-dev v1.0.0\n`],
  ['different version', `${root}\nautocfg v1.5.0\n`],
  ['wrong root', 'autocfg v1.5.1\n' + root + '\n'],
  ['workspace escape', valid.replace('/synthetic-source/apps', '/outside/apps')],
  ['registry origin annotation', valid.replace('autocfg v1.5.1', 'autocfg v1.5.1 (https://foreign.invalid)')],
  ['dev heading', `${valid}[dev-dependencies]\n`],
  ['unexpected features annotation', `${valid}autocfg v1.5.1 features=unapproved\n`],
  ['malformed dedupe', `${valid}autocfg v1.5.1 (**)\n`],
  ['unknown footer', `${valid}(*) - arbitrary claim\n`],
  ['content after footer', `${valid}(*) - dependencies omitted (already shown)\nautocfg v1.5.1\n`],
  ['missing newline', valid.trimEnd()],
  ['Windows newline', valid.replaceAll('\n', '\r\n')],
]) test(`tree rejects ${name}`, () => assert.throws(() => validate(text)));

function optionalFixture() {
  const f = graphFixture();
  f.metadata.packages.push({ id: 'optional', name: 'extra-feature-crate', version: '1.0.0',
    source: f.metadata.packages[1].source, manifest_path: '/synthetic-cache/extra-feature-crate-1.0.0/Cargo.toml' });
  f.metadata.resolve.nodes.push({ id: 'optional', deps: [], features: [] });
  f.metadata.resolve.nodes[1].deps.push({ name: 'extra_feature_crate', pkg: 'optional',
    dep_kinds: [{ kind: null, target: null }] });
  f.metadata.packages[1].dependencies = [{ name: 'extra-feature-crate', rename: null,
    kind: null, target: null, optional: true }];
  return f;
}

test('fixed Cargo tree prunes only inactive explicitly optional workspace metadata edges', () => {
  const f = optionalFixture();
  const omitted = validateCompilerTree(Buffer.from(valid), f.metadata, f.sourceRoot);
  assert.equal(omitted.packageCount, 2);
  const active = validateCompilerTree(Buffer.from(valid + 'extra-feature-crate v1.0.0\n'), f.metadata, f.sourceRoot);
  assert.equal(active.packageCount, 3);
  assert.notEqual(active.identitySetSha256, omitted.identitySetSha256);
  // Build aliases/kinds are checked just as strictly as normal target dependencies.
  f.metadata.packages[1].dependencies[0].kind = 'build';
  f.metadata.packages[1].dependencies[0].rename = 'specific-alias';
  f.metadata.resolve.nodes[1].deps.at(-1).name = 'specific_alias';
  f.metadata.resolve.nodes[1].deps.at(-1).dep_kinds[0].kind = 'build';
  assert.equal(validateCompilerTree(Buffer.from(valid), f.metadata, f.sourceRoot).packageCount, 2);
});

for (const [name, mutate] of [
  ['mandatory dependency', f => { f.metadata.packages[1].dependencies[0].optional = false; }],
  ['unknown optional declaration', f => { delete f.metadata.packages[1].dependencies; }],
  ['alias mismatch', f => { f.metadata.packages[1].dependencies[0].rename = 'another-alias'; }],
  ['target mismatch', f => { f.metadata.packages[1].dependencies[0].target = 'cfg(unix)'; }],
  ['build/normal mismatch', f => { f.metadata.packages[1].dependencies[0].kind = 'build'; }],
  ['duplicate declaration', f => { f.metadata.packages[1].dependencies.push(f.metadata.packages[1].dependencies[0]); }],
]) test(`tree projection rejects missing ${name}`, () => {
  const f = optionalFixture(); mutate(f);
  assert.throws(() => validateCompilerTree(Buffer.from(valid), f.metadata, f.sourceRoot), /missing mandatory/);
});

test('tree projection refuses ambiguous name/version identities even if an optional copy is absent', () => {
  const f = optionalFixture();
  f.metadata.packages.push({ ...f.metadata.packages.at(-1), id: 'ambiguous' });
  f.metadata.resolve.nodes.push({ id: 'ambiguous', deps: [], features: [] });
  f.metadata.resolve.nodes[0].deps.push({ pkg: 'ambiguous', dep_kinds: [{ kind: null, target: null }] });
  assert.throws(() => validateCompilerTree(Buffer.from(valid), f.metadata, f.sourceRoot), /ambiguity/);
});

test('tree rejects invalid UTF8 oversize and nonbuffer carriers', () => {
  const f = graphFixture();
  for (const data of [Buffer.from([0xff]), Buffer.alloc(2 * 1024 * 1024 + 1), valid]) {
    assert.throws(() => validateCompilerTree(data, f.metadata, f.sourceRoot));
  }
});
