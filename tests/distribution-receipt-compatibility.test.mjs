import test from 'node:test';
import assert from 'node:assert/strict';
import { bytes, sha256 } from '../scripts/distribution/canonical.mjs';
import { validateSourceReceipt } from '../scripts/distribution/source-receipt.mjs';
import { validateSourceBuildReceiptText } from
  '../scripts/distribution-release/validate-source-build-receipt.mjs';

function fixture() {
  const receipt = {
    format: 'zryna.source-build-receipt.v1',
    source: { repository: 'https://github.com/zryna/zryna', commit: 'a'.repeat(40), tree: 'b'.repeat(40) },
    command: ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'],
    toolchain: { channel: '1.97.1', cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)',
      cargoSha256: 'c'.repeat(64), rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)',
      rustcSha256: 'd'.repeat(64) },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map(logicalPath => ({ logicalPath, size: 1, sha256: 'e'.repeat(64) })),
    report: { diagnostics: [] },
  };
  const data = bytes(receipt);
  const input = { source: receipt.source,
    architectureReceipt: { size: data.length, sha256: sha256(data) },
    toolchains: ['cargo', 'rustc'].map(name => ({ name, version: '1.97.1',
      sha256: receipt.toolchain[`${name}Sha256`] })),
  };
  return { receipt, data, input };
}

test('identical canonical receipt bytes pass installed and release input checks', () => {
  const { receipt, data, input } = fixture();
  assert.deepEqual(validateSourceReceipt(data, input), receipt);
  assert.deepEqual(validateSourceBuildReceiptText(data.toString('utf8'), input), receipt);
});

test('both toolchain digests must match the external build input even for valid receipt bytes', () => {
  for (const index of [0, 1]) {
    const { receipt, data, input } = fixture();
    input.toolchains[index].sha256 = 'f'.repeat(64);
    assert.deepEqual(validateSourceReceipt(data, input), receipt);
    assert.throws(() => validateSourceBuildReceiptText(data.toString('utf8'), input),
      /R406-ARCH-TOOLCHAIN/);
  }
});

test('release input rejects changed bytes before structural admission', () => {
  const { data, input } = fixture();
  input.architectureReceipt.sha256 = 'f'.repeat(64);
  assert.throws(() => validateSourceBuildReceiptText(data.toString('utf8'), input),
    /R406-ARCH-DIGEST/);
});
