import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';

const expectedBytes = readFileSync(process.argv[4]);
assert.equal(createHash('sha256').update(expectedBytes).digest('hex'), '99570cb50f7c75af51c88339584817f65069495e5f25eb6c04b78a724ffde16d');
const expected = JSON.parse(expectedBytes).cases.find(row => row.name === process.argv[3]);
assert.equal(expected.semantic, 'accept');
let allocation = 0, clones = 0, sequence = 0, failAllocation = 0, failClone = 0;
let released = [], acquired = [], hex = [];
const live = new Map();
function fresh(bytes) {
  allocation++;
  if (allocation === failAllocation) return { status: 1 };
  const handle = Object.freeze({ identity: ++sequence });
  live.set(handle, new Uint8Array(bytes));
  acquired.push(handle.identity);
  hex.push(Buffer.from(bytes).toString('hex'));
  return { status: 0, handle };
}
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')] = Object.freeze({
  stringFromUtf8Copy(bytes, length) {
    assert(bytes instanceof Uint8Array);
    assert.equal(bytes.length, length);
    new TextDecoder('utf-8', { fatal: true }).decode(bytes);
    return fresh(bytes);
  },
  stringClone(handle) {
    clones++;
    assert(live.has(handle), 'Clone source is still live');
    if (clones === failClone) return { status: 1 };
    return fresh(live.get(handle));
  },
  stringRelease(handle) {
    assert(live.has(handle), 'exactly one release per actual acquisition');
    released.push(handle.identity);
    live.get(handle).fill(0);
    live.delete(handle);
    return 0;
  },
});
const module = await import(pathToFileURL(process.argv[2]).href);
assert.deepEqual(Object.keys(module), ['entry']);
function reset(fault = {}) {
  assert.equal(live.size, 0);
  allocation = clones = sequence = 0;
  failAllocation = fault.kind === 'allocation' ? fault.ordinal : 0;
  failClone = fault.kind === 'clone' ? fault.ordinal : 0;
  released = []; acquired = []; hex = [];
}
function closed(releases) {
  assert.deepEqual(released, releases);
  assert.equal(live.size, 0, 'no surviving actual resource after success or failure');
  assert.deepEqual([...new Set(released)].sort((a,b) => a-b), acquired.slice().sort((a,b) => a-b));
  assert.equal(released.length, acquired.length);
}
function success() {
  reset();
  assert.equal(`i32:${module.entry()}`, expected.observation);
  assert.equal(allocation, expected.allocations);
  assert.equal(clones, expected.clone_calls);
  assert.deepEqual(hex, expected.string_utf8_hex);
  closed(expected.string_releases);
}
success();
for (const fault of expected.faults) {
  reset(fault);
  assert.throws(() => module.entry(), error => error.message === fault.trap);
  closed(fault.releases);
  success(); // A failed invocation must leave both source resources and reentry state reusable.
}
reset();
assert.throws(() => module.entry(1));
closed([]);
success();
console.log(JSON.stringify({case:expected.name,target:'javascript',observation:expected.observation,exactFrozenOracle:true,faults:expected.faults.length,actualAcquisitionsReleasedExactlyOnce:true}));
