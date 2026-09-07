import assert from 'node:assert/strict';
import { readFile, readdir } from 'node:fs/promises';
import test from 'node:test';

const root = new URL('./m4-fixtures/allocation-core/', import.meta.url);
const cases = JSON.parse(await readFile(new URL('cases.json', root), 'utf8'));
const negatives = JSON.parse(await readFile(new URL('negatives.json', root), 'utf8'));

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
  const used = new Set([...cases, ...negatives].map(row => `${row.fixture}.zry`));
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
