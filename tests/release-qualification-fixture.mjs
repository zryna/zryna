import assert from 'node:assert/strict';
import { canonical, canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  assembleQualification, prepareQualification,
} from '../scripts/distribution-release/release-qualification-core.mjs';

export const QUALIFICATION_COMMIT = 'b'.repeat(40);
export const QUALIFICATION_TARGET = 'x86_64-unknown-linux-gnu';

const artifact = (logicalPath, data) => ({ logicalPath, size: data.length, sha256: sha256(data) });
const tool = (name, version, origin, digit) => ({
  name, version, origin, size: 100 + digit,
  sha256: digit.toString(16).repeat(64),
  signatureEvidenceSha256: (digit + 1).toString(16).repeat(64),
});

export function qualificationPrimitives() {
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

export async function createQualificationFixture() {
  const receipts = {
    source: Buffer.from('{"source":true}\n'),
    architecture: Buffer.from('{"architecture":true}\n'),
    gates: Buffer.from('{"gates":true}\n'),
  };
  const material = { path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n') };
  const input = {
    format: 'zryna.release-qualification-input.v1', status: 'provisional-candidate',
    productionAdmission: 'forbidden', versionCandidate: '0.2.0',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: QUALIFICATION_COMMIT, tree: 'c'.repeat(40), sourceDateEpoch: 1_789_081_200 },
    workflow: { path: '.github/workflows/release-qualification.yml', sha256: 'a'.repeat(64) },
    target: { triple: QUALIFICATION_TARGET, platformBaseline: { os: 'linux',
      distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' } },
    recipeProposal: { gitPath: 'scripts/distribution/release-recipe-v1.json',
      size: 100, sha256: 'd'.repeat(64) },
    sourceReceipt: artifact('qualification/source.json', receipts.source),
    architectureReceipt: { ...artifact('qualification/architecture.json', receipts.architecture),
      sourceCommit: QUALIFICATION_COMMIT, sourceTree: 'c'.repeat(40) },
    gateReceipt: { ...artifact('qualification/gates.json', receipts.gates), runId: '123456789',
      runAttempt: 1, sourceCommit: QUALIFICATION_COMMIT,
      requiredJobs: ['adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)']
        .map((name) => ({ name, conclusion: 'success', sourceCommit: QUALIFICATION_COMMIT })) },
    toolchains: [
      tool('cargo', '1.97.1', 'repository:rust-toolchain.toml', 1),
      tool('node', '22.22.1', 'https://nodejs.org/dist/v22.22.1/', 2),
      tool('rustc', '1.97.1', 'repository:rust-toolchain.toml', 3),
    ],
    nativeTools: [tool('linker', 'GNU ld 2.42', 'ubuntu:binutils', 4)],
    materials: { files: [{ path: material.path, mode: material.mode, size: material.data.length,
      sha256: sha256(material.data), role: 'license', material: 'source', licenses: ['LICENSE'] }] },
    compile: { argv: ['cargo', 'rustc', '--locked', '--release', '--target', QUALIFICATION_TARGET,
      '-p', 'zryna', '--bin', 'zryna'],
    environment: [{ name: 'SOURCE_DATE_EPOCH', value: '1789081200' }],
    encodedRustFlags: ['-Cdebuginfo=0'], encodedLinkerFlags: ['--build-id=none'] },
    archive: { format: 'tar-gzip',
      root: `zryna-qualification-0.2.0-${QUALIFICATION_TARGET}-${QUALIFICATION_COMMIT.slice(0, 12)}`,
      tarFormat: 'ustar', uid: 0, gid: 0, owner: '', group: '', entryMtime: 'source-epoch',
      gzipLevel: 9, gzipMtime: 0, gzipOs: 255 },
  };
  const prepared = prepareQualification(input, [material], receipts);
  const cli = Buffer.concat([Buffer.alloc(64), Buffer.from(prepared.embeddingDigest)]);
  const primitives = qualificationPrimitives();
  const assembled = await assembleQualification(prepared.binding,
    { payload: prepared.payload, cli }, primitives);
  const inspectionValue = {
    format: 'zryna.release-qualification-inspection.v1', status: 'qualification-only',
    productionAdmission: 'forbidden', target: QUALIFICATION_TARGET,
    sourceCommit: QUALIFICATION_COMMIT, bindingSha256: sha256(prepared.binding),
    binary: { format: 'ELF', architecture: 'x86_64', size: cli.length, sha256: sha256(cli) },
    checks: { identityMarker: 'qualification-binding-sha256', sourceRootOccurrences: 0,
      workRootOccurrences: 0, dynamicDependencies: ['libc.so.6'] },
  };
  return { input, binding: prepared.binding, cli, assembled, primitives, inspectionValue,
    inspection: Buffer.from(`${canonicalBounded(inspectionValue)}\n`) };
}
