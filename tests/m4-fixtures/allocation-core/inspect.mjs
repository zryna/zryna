import assert from 'node:assert/strict';
import { readFile, writeFile, unlink } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import {
  cleanupHandles,
  exposeTestExports,
  findFunctionExport,
  instrumentFunctionArgument,
} from './wasm-inspection.mjs';

const { target, entry, id, dropIndex } = JSON.parse(
  await readFile('allocation-inspection.json', 'utf8'),
);
const artifactPath = 'inspection-input';
const cases = JSON.parse(await readFile(new URL('./cases.json', import.meta.url), 'utf8'));
const row = cases.find(candidate => candidate.id === id);
assert.ok(row?.inspect, 'a fixed private observation must be selected');
const recovery = row.recovery && cases.find(candidate => candidate.id === row.recovery);
assert.equal(Boolean(row.recovery), Boolean(recovery), 'recovery must select one fixed case');
const [code, ordinal] = row.fault ?? [2, 1048576];
const command = 0x20000000 | (code << 24) | ordinal;
const normalCommand = 0x20000000 | (2 << 24) | 1048576;

function expectedStatus(selected) {
  return Number.isInteger(selected.result)
    ? 0
    : { bounds: 1, allocation: 2, capacity: 3 }[selected.result];
}

function expectedRuns(run) {
  const runs = [row];
  if (recovery) runs.push(recovery, recovery);
  return runs.map(run);
}

if (target === 'javascript') {
  // Append inspection in the private test module; the emitted artifact stays unchanged.
  const source = await readFile(artifactPath, 'utf8');
  const exported = [...source.matchAll(/export \{ (\$zryna\$de\d+) as (\w+) \};/g)]
    .find(match => match[2] === entry);
  assert.ok(exported, 'the scalar entry must have an emitted export alias');
  const selections = [[row, command]];
  if (recovery) selections.push([recovery, normalCommand], [recovery, normalCommand]);
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
function execute(selected, fault) {
  const start = observations.length;
  $zryna$observation(fault);
  const scalar = ${exported[1]}(...selected.arguments);
  return { scalar, status: $zryna$observation(0), observations: observations.slice(start) };
}
export const inspected = ${JSON.stringify(selections)}.map(([selected, fault]) => execute(selected, fault));
`;
  const path = `${artifactPath}.mjs`;
  await writeFile(path, harness, { flag: 'wx' });
  try {
    const { inspected } = await import(pathToFileURL(path).href);
    assert.deepEqual(inspected, expectedRuns(selected => ({
      scalar: expectedStatus(selected) === 0 ? selected.result : 0,
      status: expectedStatus(selected),
      observations: selected.drops,
    })));
  } finally {
    await unlink(path);
  }
} else {
  assert.equal(target, 'webassembly');
  const original = new Uint8Array(await readFile(artifactPath));
  const originalModule = new WebAssembly.Module(original);
  assert.deepEqual(WebAssembly.Module.imports(originalModule), []);
  assert.ok(WebAssembly.Module.exports(originalModule).every(value => value.kind === 'function'));
  const observation = findFunctionExport(original, '$zryna$observation');
  const instrumented = instrumentFunctionArgument(original, dropIndex, observation + 1);
  const bytes = exposeTestExports(instrumented, [
    { name: 'inspectionMemory', kind: 2, index: 0 },
  ]);
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const observe = instance.exports.$zryna$observation;
  const memory = new DataView(instance.exports.inspectionMemory.buffer);

  function payload(handle, selected) {
    assert.ok(handle >= 65536 && handle + 12 <= memory.byteLength, 'drop handle must name a record');
    const pointer = memory.getUint32(handle, true);
    const length = memory.getUint32(handle + 4, true);
    const capacity = memory.getUint32(handle + 8, true);
    assert.equal(pointer, handle + 12, 'drop handle must own its adjacent storage');
    assert.ok(length >= 1 && length <= 32 && capacity >= length && capacity <= 32);
    return Array.from({ length }, (_, index) => selected.storage === 'string'
      ? memory.getUint8(pointer + index)
      : memory.getInt32(pointer + 4 * index, true));
  }

  function execute(selected, fault) {
    observe(fault);
    const scalar = instance.exports[entry](...selected.arguments);
    const status = observe(0);
    const count = observe(0x40000000);
    assert.ok(count >= 0 && count <= 4096, 'bounded private trace');
    const trace = Array.from({ length: count }, (_, index) => observe(0x40000001 + index) >>> 0);
    const handles = cleanupHandles(trace, selected.cleanup ?? [], selected.module ?? 0);
    assert.deepEqual(handles.map(handle => payload(handle, selected)), selected.drops);

    const snapshots = [];
    for (let record = 65536; record < 65536 + 4096; record += 4) {
      const pointer = memory.getUint32(record, true);
      const length = memory.getUint32(record + 4, true);
      const capacity = memory.getUint32(record + 8, true);
      if (pointer !== record + 12 || length < 1 || length > 32 || capacity < length || capacity > 32) {
        continue;
      }
      snapshots.push(Array.from({ length }, (_, index) => selected.storage === 'string'
        ? memory.getUint8(pointer + index)
        : memory.getInt32(pointer + 4 * index, true)));
    }
    assert.deepEqual(snapshots, selected.snapshots);
    return { scalar, status, drops: handles.map(handle => payload(handle, selected)), snapshots };
  }

  const selections = [[row, command]];
  if (recovery) selections.push([recovery, normalCommand], [recovery, normalCommand]);
  const inspected = selections.map(([selected, fault]) => execute(selected, fault));
  assert.deepEqual(inspected, expectedRuns(selected => ({
    scalar: expectedStatus(selected) === 0 ? selected.result : 0,
    status: expectedStatus(selected),
    drops: selected.drops,
    snapshots: selected.snapshots,
  })));
}
console.log('allocation observation passed');
