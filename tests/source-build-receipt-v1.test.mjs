import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  validateSourceBuildReceipt,
  validateSourceBuildReceiptText,
} from '../scripts/distribution-release/validate-source-build-receipt.mjs';

const digest = (index) => index.toString(16).padStart(64, '0');

function fixture() {
  return {
    format: 'zryna.source-build-receipt.v1',
    source: {
      repository: 'https://github.com/zryna/zryna',
      commit: 'a'.repeat(40),
      tree: 'b'.repeat(40),
    },
    command: [
      'cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json',
    ],
    toolchain: {
      channel: '1.97.1',
      cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)',
      cargoSha256: digest(20),
      rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)',
      rustcSha256: digest(21),
    },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map((logicalPath, index) => ({ logicalPath, size: 100 + index, sha256: digest(index) })),
    report: { diagnostics: [] },
  };
}

test('accepts one canonical deterministic architecture receipt', () => {
  const value = fixture();
  const text = `${canonical(value)}\n`;
  const input = {
    source: value.source,
    toolchains: [
      { name: 'cargo', version: '1.97.1', sha256: value.toolchain.cargoSha256 },
      { name: 'rustc', version: '1.97.1', sha256: value.toolchain.rustcSha256 },
    ],
    architectureReceipt: { size: Buffer.byteLength(text), sha256: sha256(text) },
  };
  assert.equal(validateSourceBuildReceipt(value), value);
  assert.deepEqual(validateSourceBuildReceiptText(text, input), value);
});

test('rejects descriptor, source, and build-toolchain drift', () => {
  const value = fixture();
  const text = `${canonical(value)}\n`;
  const input = {
    source: value.source,
    toolchains: [
      { name: 'cargo', version: '1.97.1', sha256: value.toolchain.cargoSha256 },
      { name: 'rustc', version: '1.97.1', sha256: value.toolchain.rustcSha256 },
    ],
    architectureReceipt: { size: Buffer.byteLength(text), sha256: sha256(text) },
  };
  input.architectureReceipt.sha256 = 'c'.repeat(64);
  assert.throws(() => validateSourceBuildReceiptText(text, input), /R406-ARCH-DIGEST:/);
  input.architectureReceipt.sha256 = sha256(text);
  input.source = { ...input.source, tree: 'c'.repeat(40) };
  assert.throws(() => validateSourceBuildReceipt(value, input), /R406-ARCH-SOURCE:/);
  input.source = value.source;
  input.toolchains[0].sha256 = digest(22);
  assert.throws(() => validateSourceBuildReceipt(value, input), /R406-ARCH-TOOLCHAIN:/);
});

test('rejects input omission, duplication, and ordering drift', () => {
  const omitted = fixture();
  omitted.inputs.pop();
  assert.throws(() => validateSourceBuildReceipt(omitted), /R406-ARCH-SCHEMA:/);

  for (const mutate of [
    (value) => { value.inputs[1].logicalPath = value.inputs[0].logicalPath; },
    (value) => { value.inputs.reverse(); },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateSourceBuildReceipt(value), /R406-ARCH-INPUTS:/);
  }
});

test('rejects unpinned tools, unsuccessful reports, and nondeterministic fields', () => {
  const toolchain = fixture();
  toolchain.toolchain.rustcVersion = 'rustc 1.98.0 (222222222 2026-08-01)';
  assert.throws(() => validateSourceBuildReceipt(toolchain), /R406-ARCH-SCHEMA:/);

  for (const mutate of [
    (value) => { value.report.diagnostics.push({ code: 'failure' }); },
    (value) => { value.runId = '123'; },
    (value) => { value.hostPath = 'C:\\runner'; },
    (value) => { value.timestamp = '2026-09-11T00:00:00Z'; },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateSourceBuildReceipt(value), /R406-ARCH-SCHEMA:/);
  }
});

test('rejects non-canonical wire order', () => {
  assert.throws(() => validateSourceBuildReceiptText(JSON.stringify(fixture())),
    /R406-CANONICAL:/);
});
