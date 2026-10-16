import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { bytes, sha256 } from '../../../../scripts/distribution/canonical.mjs';
import {
  createCompilerSpdx, MAX_COMPILER_SPDX_BYTES, validateCompilerSpdx,
} from './compiler-publication-sbom.mjs';
import { compilerSpdxFixture, refreshMaterials } from './compiler-publication-sbom-fixture.mjs';

test('deterministic SPDX 2.3 covers actual files and binary/source identity without archive cycle', () => {
  const context = compilerSpdxFixture();
  const data = createCompilerSpdx(context);
  const spdx = validateCompilerSpdx(data, context);
  assert.equal(spdx.spdxVersion, 'SPDX-2.3');
  assert.equal(spdx.files.length, context.files.length);
  assert.equal(spdx.packages.length, 2);
  assert.equal(spdx.packages[0].filesAnalyzed, true);
  assert.match(spdx.documentNamespace, new RegExp(`${context.envelope.source.commit}/`
    + `${context.envelope.binary.sha256}$`));
  const verification = createHash('sha1').update(context.files.map(file =>
    createHash('sha1').update(file.data).digest('hex')).sort().join('')).digest('hex');
  assert.equal(spdx.packages[0].packageVerificationCode.packageVerificationCodeValue, verification);
  for (const file of context.files) {
    const actual = spdx.files.find(entry => entry.fileName === `./${file.path}`);
    assert.equal(actual.checksums[1].checksumValue, sha256(file.data));
  }
  assert.equal(spdx.packages[0].licenseDeclared, 'NOASSERTION');
  assert.deepEqual(createCompilerSpdx({ ...context, files: [...context.files].reverse() }), data);
  context.envelope.archive = { sha256: '9'.repeat(64) };
  assert.deepEqual(createCompilerSpdx(context), data);
  context.files.push({ path: 'metadata/compiler.spdx.json', data, mode: 0o644 });
  assert.deepEqual(createCompilerSpdx(context), data);
  assert.equal(validateCompilerSpdx(data, context).files.length, context.files.length - 1);
});

const graphMutations = {
  'missing ordinary file': spdx => { spdx.files.pop(); },
  'extra ordinary file': spdx => { spdx.files.push(structuredClone(spdx.files[0])); },
  'wrong checksum': spdx => { spdx.files[0].checksums[1].checksumValue = 'f'.repeat(64); },
  'wrong SHA1': spdx => { spdx.files[0].checksums[0].checksumValue = 'f'.repeat(40); },
  'missing material': spdx => { spdx.packages.pop(); },
  'wrong crate checksum': spdx => { spdx.packages[1].checksums[0].checksumValue = 'f'.repeat(64); },
  'wrong material license reference': spdx => { spdx.packages[1].sourceInfo = '{}'; },
  'wrong source identity': spdx => { spdx.packages[0].sourceInfo = '{}'; },
  'wrong binary identity': spdx => { spdx.packages[0].checksums[0].checksumValue = 'f'.repeat(64); },
  'wrong verification code': spdx => {
    spdx.packages[0].packageVerificationCode.packageVerificationCodeValue = 'f'.repeat(40);
  },
  'missing relationship': spdx => { spdx.relationships.pop(); },
  'extra relationship': spdx => { spdx.relationships.push(structuredClone(spdx.relationships[0])); },
  'wrong relationship target': spdx => { spdx.relationships[0].relatedSpdxElement = 'SPDXRef-Other'; },
  'extra top-level field': spdx => { spdx.authenticated = true; },
  'extra nested field': spdx => { spdx.files[0].authenticated = true; },
  'wrong document namespace': spdx => { spdx.documentNamespace += '/other'; },
};
for (const [label, mutate] of Object.entries(graphMutations)) {
  test(`closed SPDX rejects ${label}`, () => {
    const context = compilerSpdxFixture();
    const spdx = JSON.parse(createCompilerSpdx(context));
    mutate(spdx);
    assert.throws(() => validateCompilerSpdx(bytes(spdx), context), /SPDX/);
  });
}

const contextMutations = {
  'missing workspace LICENSE': context => {
    context.files = context.files.filter(file => file.path !== 'LICENSE');
  },
  'missing workspace NOTICE': context => {
    context.files = context.files.filter(file => file.path !== 'NOTICE');
  },
  'missing declared crate license': context => {
    context.files = context.files.filter(file => !file.path.startsWith('licenses/'));
  },
  'changed declared crate license bytes': context => {
    context.files.find(file => file.path.startsWith('licenses/')).data = Buffer.from('changed');
  },
  'unaccounted Rust license': context => {
    context.files.push({ path: 'licenses/rust/extra-1.0/LICENSE', data: Buffer.from('extra'), mode: 0o644 });
  },
  'wrong archive license owner': context => {
    context.materials.packages[0].name = 'different'; refreshMaterials(context);
  },
  'duplicate materials': context => {
    context.materials.packages.push(context.materials.packages[0]); refreshMaterials(context);
  },
  'empty materials': context => { context.materials.packages = []; refreshMaterials(context); },
  'materials with extra fields': context => { context.materials.extra = true; refreshMaterials(context); },
  'material source mismatch': context => {
    context.materials.source.tree = 'a'.repeat(40); refreshMaterials(context);
  },
  'material digest with wrong type': context => {
    context.materials.packages[0].crateSha256 = 1; refreshMaterials(context);
  },
  'noncanonical material bytes': context => {
    context.files.find(file => file.path === 'metadata/rust-materials.json').data =
      Buffer.from(JSON.stringify(context.materials));
  },
  'binary content mismatch': context => {
    context.files.find(file => file.path.startsWith('bin/')).data = Buffer.from('different');
  },
  'unexpected executable': context => {
    context.files.push({ path: 'bin/extra', data: Buffer.from('extra'), mode: 0o755 });
  },
  'wrong file mode': context => { context.files[0].mode = 0o755; },
  'path traversal': context => { context.files[0].path = '../LICENSE'; },
  'case alias': context => {
    context.files.push({ path: 'license', data: Buffer.from('alias'), mode: 0o644 });
  },
  'file ancestor collision': context => {
    context.files.push({ path: 'LICENSE/child', data: Buffer.from('alias'), mode: 0o644 });
  },
  'wrong source tag': context => { context.envelope.source.ref = 'refs/heads/main'; },
  'extra source field': context => { context.envelope.source.extra = true; },
  'invalid source epoch': context => { context.envelope.source.sourceDateEpoch = 0x100000000; },
  'wrong target': context => { context.envelope.target = 'aarch64-unknown-linux-gnu'; },
};
for (const [label, mutate] of Object.entries(contextMutations)) {
  test(`SPDX context rejects ${label}`, () => {
    const context = compilerSpdxFixture();
    mutate(context);
    assert.throws(() => createCompilerSpdx(context));
  });
}

test('valid context changes invalidate old SPDX even when bytes and descriptors agree locally', () => {
  const context = compilerSpdxFixture();
  const old = createCompilerSpdx(context);
  context.materials.packages[0].crateSha256 = 'a'.repeat(64);
  refreshMaterials(context);
  assert.throws(() => validateCompilerSpdx(old, context), /SPDX/);
  assert.doesNotThrow(() => validateCompilerSpdx(createCompilerSpdx(context), context));
});

test('noncanonical, duplicate-key, invalid UTF-8 and excessive SPDX bytes reject', () => {
  const context = compilerSpdxFixture();
  const data = createCompilerSpdx(context);
  for (const wrong of [Buffer.concat([data, Buffer.from('\n')]),
    Buffer.from('{"files":[],"files":[]}\n'), Buffer.from([0xff]),
    Buffer.alloc(MAX_COMPILER_SPDX_BYTES + 1), 'not a Buffer']) {
    assert.throws(() => validateCompilerSpdx(wrong, context), /SPDX/);
  }
});

test('existing file, package and text cardinality bounds remain enforced', () => {
  const context = compilerSpdxFixture();
  assert.throws(() => createCompilerSpdx({ ...context, files: Array(513).fill(context.files[0]) }));
  context.materials.packages = Array(257).fill(context.materials.packages[0]);
  refreshMaterials(context);
  assert.throws(() => createCompilerSpdx(context));
  const large = compilerSpdxFixture();
  large.files[0].data = Buffer.alloc(2 * 1024 * 1024 + 1);
  assert.throws(() => createCompilerSpdx(large));
});
