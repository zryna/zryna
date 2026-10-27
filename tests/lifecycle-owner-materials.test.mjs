import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { graphFixture } from '../scripts/m6/compiler-release/material-fixtures.mjs';
import { selectLifecycleCrates, captureLifecycleCrates } from '../scripts/stability-gates/lifecycle-owner-materials.mjs';

function fixture(name = 'wasmtime', version = '48.0.7') {
  const { metadata, sourceRoot } = graphFixture();
  metadata.packages[0].dependencies = [];
  Object.assign(metadata.packages[1], { name, version,
    manifest_path: `/synthetic-cache/${name}-${version}/Cargo.toml` });
  return { metadata, sourceRoot, cargoLock: readFileSync(new URL('../Cargo.lock', import.meta.url)),
    cargoTree: Buffer.from(`zryna-playground-compiler v0.1.0 (${sourceRoot}/apps/zryna-playground-compiler)\n${name} v${version}\n`) };
}
test('lifecycle selection uses the pinned current security record for patched compiler crates', () => {
  for (const [name, version] of [['wasmtime', '48.0.7'], ['cranelift-codegen', '0.135.7'], ['wasmparser', '0.254.2']]) {
    const result = selectLifecycleCrates(fixture(name, version));
    assert.equal(result.external.length, 1);
    assert.equal(result.external[0].pin.name, name); assert.equal(result.external[0].pin.version, version);
    result.external[0].pin.crateSha256 = '0'.repeat(64);
    assert.notEqual(selectLifecycleCrates(fixture(name, version)).external[0].pin.crateSha256, '0'.repeat(64));
  }
});
test('lifecycle rejects stale or substituted locks and unknown crate versions', () => {
  const value = fixture();
  assert.throws(() => selectLifecycleCrates({ ...value, cargoLock: Buffer.concat([value.cargoLock, Buffer.from('\n')]) }), /exact lock/);
  assert.throws(() => selectLifecycleCrates(fixture('wasmtime', '48.0.8')), /unknown crate|lock identity/);
  assert.throws(() => selectLifecycleCrates({ ...value, approved: new Map() }));
});
test('lifecycle cannot hide test-only dependencies in a normal/build graph', () => {
  const value = fixture(); value.metadata.packages[0].dependencies.push({ name: 'hidden-test-crate', kind: 'dev' });
  assert.throws(() => selectLifecycleCrates(value), /forbids root dev dependencies/);
});
test('lifecycle requires original captures and rejects caller supplied authority', () => {
  const value = { ...fixture(), crateArchives: [], upstreamLicense: null };
  assert.throws(() => captureLifecycleCrates(value), /capture count/);
  assert.throws(() => captureLifecycleCrates({ ...value, executionApproved: true }));
});
