import assert from 'node:assert/strict';
import { readFile, writeFile, unlink } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { exposeTestExports } from './wasm-inspection.mjs';

const [target, entry, id, artifactPath] = process.argv.slice(2);
const cases = JSON.parse(await readFile(new URL('./cases.json', import.meta.url), 'utf8'));
const row = cases.find(candidate => candidate.id === id);
assert.ok(row?.inspect, 'a fixed private observation must be selected');
const [code, ordinal] = row.fault ?? [2, 1048576];
const command = 0x20000000 | (code << 24) | ordinal;
const expectedStatus = Number.isInteger(row.result) ? 0 : { bounds: 1, allocation: 2, capacity: 3 }[row.result];

if (target === 'javascript') {
  // Append inspection in the private test module; the emitted artifact stays unchanged.
  const source = await readFile(artifactPath, 'utf8');
  const exported = [...source.matchAll(/export \{ (\$zryna\$de\d+) as (\w+) \};/g)]
    .find(match => match[2] === entry);
  assert.ok(exported, 'the scalar entry must have an emitted export alias');
  const harness = `${source}
const observations = [];
const released = new Set();
const originalDrop = $zryna$drop;
$zryna$drop = function(value) {
  if (value && typeof value === 'object' && (value.$k === 1 || value.$k === 2)) {
    if (released.has(value)) throw new Error('owner released twice');
    released.add(value);
    observations.push(value.$k === 1 ? [...Buffer.from(value.$v, 'utf8')] : [...value.$v]);
  }
  return originalDrop(value);
};
const originalClone = $zryna$clone;
$zryna$clone = function(value) {
  const copied = originalClone(value);
  if (value && typeof value === 'object') {
    if (copied === value) throw new Error('clone reused its source owner');
    if (value.$k === 2 && copied.$v === value.$v) throw new Error('clone shared mutable storage');
  }
  return copied;
};
$zryna$observation(${command});
const scalar = ${exported[1]}(...${JSON.stringify(row.arguments)});
export const inspected = { scalar, status: $zryna$observation(0), observations };
`;
  const path = `${artifactPath}.mjs`;
  await writeFile(path, harness, { flag: 'wx' });
  try {
    const { inspected } = await import(pathToFileURL(path).href);
    assert.equal(inspected.status, expectedStatus);
    assert.equal(inspected.scalar, expectedStatus === 0 ? row.result : 0);
    assert.deepEqual(inspected.observations, row.drops);
  } finally {
    await unlink(path);
  }
} else {
  assert.equal(target, 'webassembly');
  const original = new Uint8Array(await readFile(artifactPath));
  const originalModule = new WebAssembly.Module(original);
  assert.deepEqual(WebAssembly.Module.imports(originalModule), []);
  assert.ok(WebAssembly.Module.exports(originalModule).every(value => value.kind === 'function'));
  // Expose memory only in an in-memory test copy, preserving all code/data sections.
  const bytes = exposeTestExports(original, [{ name: 'inspectionMemory', kind: 2, index: 0 }]);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const observe = instance.exports.$zryna$observation;
  observe(command);
  const scalar = instance.exports[entry](...row.arguments);
  assert.equal(observe(0), expectedStatus);
  assert.equal(scalar, expectedStatus === 0 ? row.result : 0);
  const memory = new DataView(instance.exports.inspectionMemory.buffer);
  const snapshots = [];
  const pointers = new Set();
  // These tiny fixtures fit in the first 4 KiB of the private invocation arena.
  for (let record = 65536; record < 65536 + 4096; record += 4) {
    const pointer = memory.getUint32(record, true);
    const length = memory.getUint32(record + 4, true);
    const capacity = memory.getUint32(record + 8, true);
    if (pointer !== record + 12 || length < 1 || length > 32 || capacity < length || capacity > 32) continue;
    assert.ok(!pointers.has(pointer), 'distinct records must own distinct storage');
    pointers.add(pointer);
    snapshots.push(Array.from({ length }, (_, index) => row.storage === 'string'
      ? memory.getUint8(pointer + index)
      : memory.getInt32(pointer + 4 * index, true)));
  }
  assert.deepEqual(snapshots, row.snapshots);
}
console.log('allocation observation passed');
