import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import test from 'node:test';
import { exposeTestExports } from './m4-fixtures/allocation-core/wasm-inspection.mjs';

const root = new URL('./m4-fixtures/allocation-core/', import.meta.url);
const cases = JSON.parse(await readFile(new URL('cases.json', root), 'utf8'));
const negatives = JSON.parse(await readFile(new URL('negatives.json', root), 'utf8'));
const capacity = JSON.parse(await readFile(new URL('capacity-cases.json', root), 'utf8'));

test('allocation corpus retains independent Q4–Q8 results and exact bounds', () => {
  const expected = new Map([
    ['q4', 17], ['q5', 17], ['q6-copy', 9], ['q6-copy-bound', 'bounds'],
    ['q6-original', 13], ['q7-last', 9], ['q7-negative', 'bounds'],
    ['q7-first-extra', 'bounds'], ['q7-empty', 'bounds'], ['q8', 13],
  ]);
  assert.equal(new Set(cases.map(row => row.id)).size, cases.length);
  for (const [id, value] of expected) assert.equal(cases.find(row => row.id === id)?.result, value, id);
  for (const [id, index] of [['q7-last', 1], ['q7-negative', -1], ['q7-first-extra', 2]]) {
    assert.deepEqual(cases.find(row => row.id === id).arguments, [index]);
  }
  assert.deepEqual(cases.find(row => row.id === 'q4').bytes, [[104, 195, 169], [104, 195, 169]]);
  assert.deepEqual(cases.find(row => row.id === 'q5').bytes, [[104, 195, 169], [33], [104, 195, 169, 33]]);
});

test('fault cases are bounded and retain completed owners without partial append', () => {
  for (const row of cases.filter(row => row.fault)) {
    assert.ok([2, 3].includes(row.fault[0]));
    assert.ok(Number.isInteger(row.fault[1]) && row.fault[1] >= 1 && row.fault[1] <= 1048576);
    assert.ok(Array.isArray(row.cleanup));
  }
  for (const id of ['clone-first', 'concat-first', 'vec-first', 'push-first']) {
    assert.deepEqual(cases.find(row => row.id === id).cleanup, []);
  }
  assert.deepEqual(cases.find(row => row.id === 'push-growth').drops, [[7, 9]]);
  assert.deepEqual(cases.find(row => row.id === 'prefix-two').drops,
    [[104, 195, 169, 104, 195, 169], [104, 195, 169], [104, 195, 169]]);
});

test('N7 and N10 keep source diagnostics and all source files are used', async () => {
  assert.deepEqual(negatives.map(row => row.oracle), ['N7', 'N7', 'N10', 'N10', 'N10']);
  const used = new Set([...cases, ...negatives, ...capacity.source].map(row => `${row.fixture}.zry`));
  const present = (await readdir(root)).filter(path => path.endsWith('.zry'));
  for (const file of present.filter(path => path.endsWith('-body.zry'))) {
    assert.ok(used.has(file.replace('-body.zry', '.zry')), 'dependency must have a selected entry');
    used.add(file);
  }
  assert.deepEqual([...used].sort(), present.sort());
  for (const file of used) {
    const source = await readFile(new URL(file, root), 'utf8');
    assert.match(source, file.endsWith('-body.zry') ? /export function text\(/ : /export function score\(/);
  }
});

test('capacity oracles distinguish universal limits from the fixed target arena', async () => {
  const abi = await readFile(new URL('../crates/zryna-ownership-runtime-abi/src/lib.rs', import.meta.url), 'utf8');
  for (const [name, expected] of [
    ['MAX_DYNAMIC_ALLOCATION_BYTES', capacity.limits.dynamicAllocationBytes],
    ['MAX_STRING_BYTES', capacity.limits.stringBytes],
    ['MAX_VEC_ELEMENTS', capacity.limits.vecElements],
  ]) {
    const declared = abi.match(new RegExp(`${name}: [^=]+ = ([\\d_]+);`));
    assert.ok(declared, `${name} must retain explicit ABI authority`);
    assert.equal(Number(declared[1].replaceAll('_', '')), expected);
  }
  assert.deepEqual(capacity.source.map(row => row.arguments[0]), [1048576, 1048577, 1048575, 1048576]);
  assert.deepEqual(capacity.source.map(row => row.result), [13, 'capacity', 13, 'bounds']);
  assert.deepEqual(capacity.webassemblyAllocator.map(row => [row.bytes, row.status]), [
    [16711680, 0], [16711681, 2], [67108863, 2], [67108864, 2],
    [67108865, 2], [2147483647, 2], [2147483648, 3],
  ]);
});

test('private inspection adds exports while retaining original storage and function behavior', () => {
  // Independent minimal module: echo(i32) -> i32, one fixed page, two mutable globals.
  const original = Uint8Array.from([
    0, 97, 115, 109, 1, 0, 0, 0,
    1, 6, 1, 96, 1, 127, 1, 127,
    3, 2, 1, 0,
    5, 4, 1, 1, 1, 1,
    6, 11, 2, 127, 1, 65, 0, 11, 127, 1, 65, 0, 11,
    7, 8, 1, 4, 101, 99, 104, 111, 0, 0,
    10, 6, 1, 4, 0, 32, 0, 11,
  ]);
  const saved = original.slice();
  const bytes = exposeTestExports(original, [
    { name: 'inspectionAllocate', kind: 0, index: 0 },
    { name: 'inspectionMemory', kind: 2, index: 0 },
    { name: 'inspectionArena', kind: 3, index: 0 },
    { name: 'inspectionStatus', kind: 3, index: 1 },
  ]);
  assert.deepEqual(original, saved);
  const untouched = new WebAssembly.Module(original);
  assert.deepEqual(WebAssembly.Module.exports(untouched), [{ name: 'echo', kind: 'function' }]);
  const { exports: e } = new WebAssembly.Instance(new WebAssembly.Module(bytes));
  assert.equal(e.echo(17), 17);
  assert.equal(e.inspectionAllocate(2147483648), -2147483648);
  assert.equal(e.inspectionMemory.buffer.byteLength, 65536);
  e.inspectionArena.value = 8;
  assert.equal(e.inspectionArena.value, 8);
  assert.equal(e.inspectionStatus.value, 0);
});
