import test from 'node:test';
import assert from 'node:assert/strict';
import { bytes } from '../scripts/distribution/canonical.mjs';
import { validateSourceReceipt } from '../scripts/distribution/source-receipt.mjs';

const source = { repository: 'https://github.com/zryna/zryna',
  commit: 'a'.repeat(40), tree: 'b'.repeat(40) };
function receipt() {
  return {
    format: 'zryna.source-build-receipt.v1', source,
    command: ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'],
    toolchain: { channel: '1.97.1', cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)',
      cargoSha256: 'c'.repeat(64), rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)',
      rustcSha256: 'd'.repeat(64) },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map(logicalPath => ({ logicalPath, size: 1, sha256: 'e'.repeat(64) })),
    report: { diagnostics: [] },
  };
}

test('installed receipt accepts the final five-field toolchain record', () => {
  const document = receipt();
  assert.deepEqual(validateSourceReceipt(bytes(document), { source }), document);
});

test('installed receipt rejects missing, malformed, or extra toolchain fields', () => {
  for (const tool of ['cargo', 'rustc']) {
    for (const value of [undefined, 'A'.repeat(64), 'a'.repeat(63), 123]) {
      const document = receipt();
      if (value === undefined) delete document.toolchain[`${tool}Sha256`];
      else document.toolchain[`${tool}Sha256`] = value;
      assert.throws(() => validateSourceReceipt(bytes(document), { source }));
    }
  }
  const document = receipt();
  document.toolchain.extra = true;
  assert.throws(() => validateSourceReceipt(bytes(document), { source }));
});

test('installed receipt rejects alternate compiler builds and source identities', () => {
  for (const tool of ['cargo', 'rustc']) {
    const document = receipt();
    document.toolchain[`${tool}Version`] = `${tool} 1.97.1 (different build)`;
    assert.throws(() => validateSourceReceipt(bytes(document), { source }), /source tool version/);
  }
  assert.throws(() => validateSourceReceipt(bytes(receipt()), {
    source: { ...source, tree: 'f'.repeat(40) },
  }), /source receipt identity/);
});
