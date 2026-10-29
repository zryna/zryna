import assert from 'node:assert/strict';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { parse, readSafe, sha256 } from './input.mjs';
import { loadRegistry } from './registry.mjs';

export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
export const REGISTRY = 'tests/stability-fuzz-campaign-v2.json';
export const REGISTRY_SHA256 = '4013cea60a0cc9036b8d5913dedb61a8a2310dd88ace5ced5d3edc38713d8e86';

export function loadFuzzRegistry(root = ROOT) {
  const bytes = readSafe(root, REGISTRY);
  assert.equal(sha256(bytes), REGISTRY_SHA256, 'frozen campaign registry');
  const registry = parse(bytes);
  assert.equal(registry.format, 'zryna.m7.fuzz-campaign-registry.v2');
  assert.equal(registry.closureStatus, 'blocked');
  assert.equal(registry.globalM7, false);
  const legacy = loadRegistry(root);
  assert.deepEqual(registry.fullOwner, legacy.gates.find(gate => gate.id === 'security-authorities'));
  assert.deepEqual(registry.toolchains, legacy.toolchains);
  for (const pin of registry.sourcePins) {
    const source = readSafe(root, pin.path, 2 * 1024 * 1024);
    assert.equal(source.length, pin.bytes);
    assert.equal(sha256(source), pin.sha256, 'registered generator source');
  }
  return { registry, bytes };
}
