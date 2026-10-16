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

test('workspace-wide optional metadata additions cannot claim exact app identity closure', () => {
  const f = graphFixture();
  f.metadata.packages.push({ id: 'optional', name: 'extra-feature-crate', version: '1.0.0',
    source: f.metadata.packages[1].source, manifest_path: '/synthetic-cache/extra-feature-crate-1.0.0/Cargo.toml' });
  f.metadata.resolve.nodes.push({ id: 'optional', deps: [], features: [] });
  f.metadata.resolve.nodes[1].deps.push({ pkg: 'optional', dep_kinds: [{ kind: null, target: null }] });
  assert.throws(() => validateCompilerTree(Buffer.from(valid), f.metadata, f.sourceRoot), /exact app/);
});

test('tree rejects invalid UTF8 oversize and nonbuffer carriers', () => {
  const f = graphFixture();
  for (const data of [Buffer.from([0xff]), Buffer.alloc(2 * 1024 * 1024 + 1), valid]) {
    assert.throws(() => validateCompilerTree(data, f.metadata, f.sourceRoot));
  }
});
