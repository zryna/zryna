// Nonexecutable synthetic ELF/metadata carriers: no compiler/signature/publication proof.
import { bytes, sha256 } from '../../../../scripts/distribution/canonical.mjs';
import { binaryPath, inputPaths, product, sourceRef, target, workflowPath } from './compiler-publication-constants.mjs';

export function compilerPublicationFixture() {
  const source = { repository: 'https://github.com/zryna/zryna', ref: sourceRef,
    commit: '1'.repeat(40), tree: '2'.repeat(40), sourceDateEpoch: 1700000000,
    tagObject: '3'.repeat(40), tagType: 'annotated' };
  const binary = Buffer.alloc(128);
  binary.set([0x7f, 69, 76, 70, 2, 1, 1]); binary.writeUInt16LE(2, 16); binary.writeUInt16LE(62, 18);
  binary.writeBigUInt64LE(64n, 32); binary.writeUInt16LE(56, 54); binary.writeUInt16LE(1, 56);
  binary.writeUInt32LE(1, 64); binary.writeUInt32LE(5, 68); binary.writeBigUInt64LE(128n, 96);
  const license = Buffer.from('Synthetic fixture only; not real crate license/provenance.\n');
  const licensePath = 'licenses/rust/test-crate-1.2.3/LICENSE-MIT';
  const inputs = Object.fromEntries(Object.entries(inputPaths).map(([name, path]) =>
    [name, { path, bytes: 17, sha256: sha256(Buffer.from(`synthetic:${path}`)) }]));
  const materials = { format: 'zryna.playground-compiler-rust-materials.v1', version: 1,
    source, target, cargoLockSha256: inputs.cargoLock.sha256,
    packages: [{ name: 'test-crate', version: '1.2.3', crateSha256: '4'.repeat(64),
      licenses: [{ path: licensePath, bytes: license.length, sha256: sha256(license) }] }] };
  const files = [{ path: 'LICENSE', mode: 0o644, data: Buffer.from('Synthetic workspace license fixture\n') },
    { path: 'NOTICE', mode: 0o644, data: Buffer.from('Synthetic notice fixture\n') },
    { path: 'README.md', mode: 0o644, data: Buffer.from('Synthetic metadata fixture, never executed.\n') },
    { path: binaryPath, mode: 0o755, data: binary },
    { path: licensePath, mode: 0o644, data: license }];
  const provenance = { workflow: { path: workflowPath, ref: sourceRef, sha: source.commit, event: 'push' },
    toolchain: { rust: '1.97.1', node: '22.22.1', pnpm: '11.18.0' }, inputs,
    build: { package: product, profile: 'release', target, locked: true, sourceCommitEnv: source.commit },
    requiredCi: { workflowPath: '.github/workflows/ci.yml', runId: 1, runAttempt: 1,
      headSha: source.commit, headTree: source.tree, conclusion: 'success' },
    reproduction: { status: 'byte-identical', firstBuildSha256: sha256(binary), secondBuildSha256: sha256(binary) } };
  return { source, files, materials: bytes(materials), provenance };
}
