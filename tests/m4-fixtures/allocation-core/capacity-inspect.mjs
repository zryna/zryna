import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { exposeTestExports } from './wasm-inspection.mjs';

const artifactPath = 'allocation-boundaries.wasm';
const cases = JSON.parse(await readFile(new URL('./capacity-cases.json', import.meta.url), 'utf8'));
const original = new Uint8Array(await readFile(artifactPath));
const module = new WebAssembly.Module(original);
assert.deepEqual(WebAssembly.Module.imports(module), []);
assert.ok(WebAssembly.Module.exports(module).every(value => value.kind === 'function'));

// encode.rs declares the allocator first and the arena/status globals at 0/1.
// This private copy exposes those existing definitions without replacing code.
const bytes = exposeTestExports(original, [
  { name: 'inspectionAllocate', kind: 0, index: 0 },
  { name: 'inspectionMemory', kind: 2, index: 0 },
  { name: 'inspectionArena', kind: 3, index: 0 },
  { name: 'inspectionStatus', kind: 3, index: 1 },
]);
const inspected = new WebAssembly.Module(bytes);
const actual = [];
for (const row of cases.webassemblyAllocator) {
  const instance = new WebAssembly.Instance(inspected, {});
  const e = instance.exports;
  assert.equal(e.inspectionMemory.buffer.byteLength, cases.limits.webassemblyArenaBytes);
  assert.equal(e.inspectionArena.value, cases.limits.webassemblyHeapStart);
  assert.equal(e.inspectionStatus.value, 0);
  // No payload is constructed or accessed, including for the 2 GiB request.
  actual.push({
    id: row.id,
    pointer: e.inspectionAllocate(row.bytes),
    status: e.inspectionStatus.value,
    arena: e.inspectionArena.value,
  });
  assert.equal(e.inspectionMemory.buffer.byteLength, cases.limits.webassemblyArenaBytes);
}
const expected = cases.webassemblyAllocator.map(({ bytes: size, ...row }) => row);
assert.deepEqual(actual, expected, 'target exhaustion must retain ALLOCATION within the ABI limit');
console.log('allocation capacity observation passed');
