import { bytes, sha256 } from '../../../../scripts/distribution/canonical.mjs';

// Synthetic metadata fixtures exercise no compiler, signature, or publication gate.
export function compilerSpdxFixture() {
  const source = {
    repository: 'https://github.com/zryna/zryna',
    ref: 'refs/tags/v0.1.0-playground-compiler.2', commit: '1'.repeat(40),
    tree: '2'.repeat(40), sourceDateEpoch: 1700000000,
    tagObject: '3'.repeat(40), tagType: 'annotated',
  };
  const binary = Buffer.from('synthetic compiler metadata subject, never executable');
  const license = Buffer.from('synthetic crate license fixture\n');
  const path = 'licenses/rust/test-crate-1.2.3/LICENSE-MIT';
  const materials = {
    format: 'zryna.playground-compiler-rust-materials.v1', version: 1,
    source: structuredClone(source), target: 'x86_64-unknown-linux-gnu',
    cargoLockSha256: '4'.repeat(64),
    packages: [{ name: 'test-crate', version: '1.2.3', crateSha256: '5'.repeat(64),
      licenses: [{ path, bytes: license.length, sha256: sha256(license) }] }],
  };
  const files = [
    { path: 'LICENSE', data: Buffer.from('workspace license fixture\n'), mode: 0o644 },
    { path: 'NOTICE', data: Buffer.from('workspace notice fixture\n'), mode: 0o644 },
    { path: 'README.md', data: Buffer.from('synthetic metadata only\n'), mode: 0o644 },
    { path: 'bin/zryna-playground-compiler', data: binary, mode: 0o755 },
    { path, data: license, mode: 0o644 },
    { path: 'metadata/build-receipt.json', data: bytes({ fixture: true }), mode: 0o644 },
    { path: 'metadata/rust-materials.json', data: bytes(materials), mode: 0o644 },
  ];
  const envelope = {
    product: 'zryna-playground-compiler', productVersion: '0.1.0',
    target: materials.target, source,
    binary: { path: 'bin/zryna-playground-compiler', bytes: binary.length,
      sha256: sha256(binary) },
  };
  return { files, envelope, materials };
}

export function refreshMaterials(context) {
  context.files.find(file => file.path === 'metadata/rust-materials.json').data = bytes(context.materials);
}
