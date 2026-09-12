import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  assembleQualification, prepareQualification, verifyQualification,
} from '../scripts/distribution-release/release-qualification-core.mjs';
import { validateBuildInput } from '../scripts/distribution-release/validate-build-input.mjs';
import { validateReleaseQualificationInput } from '../scripts/distribution-release/validate-release-qualification-input.mjs';

const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const TARGET = 'x86_64-unknown-linux-gnu';
const artifact = (logicalPath, data) => ({ logicalPath, size: data.length, sha256: sha256(data) });
const tool = (name, version, origin, digit) => ({
  name, version, origin, size: 100 + digit,
  sha256: digit.toString(16).repeat(64),
  signatureEvidenceSha256: (digit + 1).toString(16).repeat(64),
});

function fixtures() {
  const receipts = {
    source: Buffer.from('{"source":true}\n'),
    architecture: Buffer.from('{"architecture":true}\n'),
    gates: Buffer.from('{"gates":true}\n'),
  };
  const material = { path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n') };
  const input = {
    format: 'zryna.release-qualification-input.v1',
    status: 'provisional-candidate',
    productionAdmission: 'forbidden',
    versionCandidate: '0.2.0',
    source: {
      repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: TREE, sourceDateEpoch: 1_789_081_200,
    },
    workflow: { path: '.github/workflows/release-qualification.yml', sha256: 'a'.repeat(64) },
    target: {
      triple: TARGET,
      platformBaseline: { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' },
    },
    recipeProposal: {
      gitPath: 'scripts/distribution/release-recipe-v1.json', size: 100, sha256: 'b'.repeat(64),
    },
    sourceReceipt: artifact('qualification/source.json', receipts.source),
    architectureReceipt: {
      ...artifact('qualification/architecture.json', receipts.architecture),
      sourceCommit: COMMIT, sourceTree: TREE,
    },
    gateReceipt: {
      ...artifact('qualification/gates.json', receipts.gates),
      runId: '123456789', runAttempt: 1, sourceCommit: COMMIT,
      requiredJobs: [
        'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
      ].map((name) => ({ name, conclusion: 'success', sourceCommit: COMMIT })),
    },
    toolchains: [
      tool('cargo', '1.97.1', 'repository:rust-toolchain.toml', 1),
      tool('node', '22.22.1', 'https://nodejs.org/dist/v22.22.1/', 2),
      tool('rustc', '1.97.1', 'repository:rust-toolchain.toml', 3),
    ],
    nativeTools: [tool('linker', 'GNU ld 2.42', 'ubuntu:binutils', 4)],
    materials: { files: [{
      path: material.path, mode: material.mode, size: material.data.length,
      sha256: sha256(material.data), role: 'license', material: 'source', licenses: ['LICENSE'],
    }] },
    compile: {
      argv: ['cargo', 'rustc', '--locked', '--release', '--target', TARGET,
        '-p', 'zryna', '--bin', 'zryna'],
      environment: [{ name: 'SOURCE_DATE_EPOCH', value: '1789081200' }],
      encodedRustFlags: ['-Cdebuginfo=0'],
      encodedLinkerFlags: ['--build-id=none'],
    },
    archive: {
      format: 'tar-gzip', root: `zryna-qualification-0.2.0-${TARGET}-${COMMIT.slice(0, 12)}`,
      tarFormat: 'ustar', uid: 0, gid: 0, owner: '', group: '', entryMtime: 'source-epoch',
      gzipLevel: 9, gzipMtime: 0, gzipOs: 255,
    },
  };
  return { input, material, receipts };
}

function primitives() {
  const encode = (root, files) => Buffer.from(`${canonical({ root, files: files.map((file) => ({
    path: file.path, mode: file.mode, data: file.data.toString('base64'),
  })) })}\n`);
  const decode = (archive, root) => {
    const value = JSON.parse(archive.toString('utf8'));
    assert.equal(value.root, root);
    return value.files.map((file) => ({
      path: file.path, mode: file.mode, data: Buffer.from(file.data, 'base64'),
    }));
  };
  return {
    requireArchiveRuntime() {},
    encodeTar: async (root, files) => encode(root, files),
    decodeTar: async (archive, root) => decode(archive, root),
    encodeZip: encode,
    decodeZip: decode,
    verifyCompiledIdentity(binary, digest) {
      assert(binary.includes(Buffer.from(digest)));
    },
  };
}

test('prepares, assembles, and independently verifies qualification-only bytes', async () => {
  const { input, material, receipts } = fixtures();
  assert.equal(validateReleaseQualificationInput(input), input);
  const prepared = prepareQualification(input, [material], receipts);
  const cli = Buffer.concat([Buffer.alloc(64), Buffer.from(prepared.embeddingDigest)]);
  const assembled = await assembleQualification(prepared.binding,
    { payload: prepared.payload, cli }, primitives());
  assert.match(assembled.filename, /^zryna-qualification-/);
  assert(!assembled.files.some(({ path }) => path === 'metadata/distribution.json'));
  assert(!assembled.files.some(({ path }) => path.endsWith('.build-receipt.json')));
  const verified = await verifyQualification(assembled.archive, {
    filename: assembled.filename,
    size: assembled.archive.length,
    sha256: sha256(assembled.archive),
    binding: prepared.binding,
  }, primitives());
  assert.equal(verified.archiveSha256, sha256(assembled.archive));
  assert.equal(verified.inventory.productionAdmission, 'forbidden');
  assert.throws(() => validateBuildInput(input), /R406-BUILD-SCHEMA:/);
});

test('rejects production identity, unordered materials, and archive descriptor drift', async () => {
  const production = fixtures();
  production.input.source.ref = 'refs/tags/v0.2.0';
  assert.throws(() => validateReleaseQualificationInput(production.input),
    /R406-QUALIFICATION-INPUT-SCHEMA:/);

  const mismatch = fixtures();
  const extra = { path: 'A', mode: 0o644, data: Buffer.from('a') };
  assert.throws(() => prepareQualification(mismatch.input, [mismatch.material, extra], mismatch.receipts),
    /R406-QUALIFICATION-CORE:/);

  const valid = fixtures();
  const prepared = prepareQualification(valid.input, [valid.material], valid.receipts);
  const cli = Buffer.concat([Buffer.alloc(64), Buffer.from(prepared.embeddingDigest)]);
  const assembled = await assembleQualification(prepared.binding,
    { payload: prepared.payload, cli }, primitives());
  await assert.rejects(() => verifyQualification(assembled.archive, {
    filename: assembled.filename, size: assembled.archive.length,
    sha256: 'f'.repeat(64), binding: prepared.binding,
  }, primitives()), /qualification archive descriptor differs/);
});

test('rejects a qualification file that aliases a parent directory', () => {
  const { input, receipts } = fixtures();
  const captured = [
    { path: 'runtime', mode: 0o644, data: Buffer.from('file collision') },
    { path: 'runtime/node', mode: 0o755, data: Buffer.from('nested executable') },
  ];
  captured.sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
  input.materials.files = captured.map(({ path, mode, data }) => ({
    path, mode, size: data.length, sha256: sha256(data), role: 'runtime',
    material: 'node-22.22.1', licenses: ['runtime'],
  }));
  assert.throws(() => prepareQualification(input, captured, receipts),
    /R406-QUALIFICATION-CORE: qualification file path, order, mode, or bytes differ/);
});
