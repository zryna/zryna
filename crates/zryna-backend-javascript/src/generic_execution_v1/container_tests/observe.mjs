// PRIVATE test injection adapter. This is not the final module-private production runtime ABI.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { pathToFileURL } from 'node:url';
const bytes = readFileSync(process.argv[4]);
assert.equal(createHash('sha256').update(bytes).digest('hex'), 'dc59b80c362ac4344e5252a9329d7544322e496b1066a3c21281ef6e3d20c6db');
const expected = JSON.parse(bytes).cases.find(row => row.case === process.argv[3]);
assert(expected);
let ordinal = 0, sequence = 0, cloneCalls = 0, fault = {}, acquired = [], released = [], countEvents = [];
const live = new Map(), controls = new Map();
const record = fields => Object.freeze(Object.assign(Object.create(null), fields));
function failed() {
  const value = Object.create(null);
  Object.defineProperty(value, 'status', { value: 1 });
  for (const key of ['handle', 'pointer', 'value']) Object.defineProperty(value, key, { get() { throw new Error('UNWRITTEN-RESULT-READ'); } });
  return Object.freeze(value);
}
function allocate(kind, payload) {
  ordinal++;
  if (fault.kind === 'allocation' && ordinal === fault.ordinal) return undefined;
  const token = record({ identity: ++sequence });
  live.set(token, { kind, ...payload });
  acquired.push(kind);
  return token;
}
function release(token, kind) {
  const value = live.get(token);
  assert(value, 'release must consume one actual live acquisition');
  assert.equal(value.kind, kind);
  live.delete(token);
  released.push(token.identity);
}
function control(c) {
  assert.equal(Object.getPrototypeOf(c), null, 'private control carrier has no prototype');
  assert(live.has(c.$ptr), 'control retains exact actual raw allocation');
  assert.equal(live.get(c.$ptr).kind, 'Shared control');
  assert(Number.isInteger(c.$strong) && c.$strong >= 0 && c.$strong <= 0xffffffff);
  assert(Number.isInteger(c.$weak) && c.$weak >= 0 && c.$weak <= 0xffffffff);
  const prior = controls.get(c.$ptr);
  if (prior) assert.equal(prior, c, 'one count state per actual control allocation');
  else controls.set(c.$ptr, c);
  assert.equal(c.$weak > 0, true, 'allocated control retains implicit or explicit weak');
  return c;
}
function count(c, operation, strong) {
  control(c);
  assert(!c.$pending, 'no count transition during last-strong payload cleanup');
  if (strong) assert(c.$strong > 0, 'cannot clone dead Shared');
  const field = strong ? '$strong' : '$weak';
  if (c[field] === 0xffffffff) return 3;
  c[field]++;
  countEvents.push(operation);
  return 0;
}
const runtime = Object.assign(Object.create(null), {
  stringFromUtf8Copy(source, length) {
    assert(source instanceof Uint8Array); assert.equal(source.length, length);
    new TextDecoder('utf-8', { fatal: true }).decode(source);
    const handle = allocate(`String ${Buffer.from(source).toString()}`, { bytes: new Uint8Array(source) });
    return handle ? record({ status: 0, handle }) : failed();
  },
  stringClone(source) {
    cloneCalls++;
    const item = live.get(source); assert(item && item.bytes, 'clone source remains live');
    if (fault.kind === 'string-clone' && cloneCalls === fault.ordinal) return failed();
    const handle = allocate(`cloned ${item.kind}`, { bytes: new Uint8Array(item.bytes) });
    return handle ? record({ status: 0, handle }) : failed();
  },
  stringRelease(source) {
    const item = live.get(source); assert(item && item.bytes);
    if ([...controls.values()].some(c => c.$pending)) countEvents.push('payloadDrop');
    release(source, item.kind); return 0;
  },
  vecAllocate(layout, length) {
    assert.equal(Object.getPrototypeOf(layout), null);
    for (const field of ['id', 'stride', 'alignment']) assert(Number.isSafeInteger(layout[field]) && layout[field] >= 0);
    assert(layout.stride > 0 && layout.alignment > 0);
    assert(Number.isInteger(length) && length >= 0 && length <= 1048576);
    assert(length * layout.stride <= 67108864);
    if (length === 0) return record({ status: 0, handle: 0 });
    const handle = allocate(acquired.includes('source Vec') ? 'destination Vec' : 'source Vec', { layout, length });
    return handle ? record({ status: 0, handle }) : failed();
  },
  vecReleaseStorage(layout, storage) {
    if (storage === 0) return 0;
    const item = live.get(storage); assert(item && item.layout === layout);
    release(storage, item.kind); return 0;
  },
  allocate(size, alignment) {
    assert(Number.isSafeInteger(size) && size >= 8);
    assert(Number.isSafeInteger(alignment) && alignment >= 4);
    const pointer = allocate('Shared control', { size, alignment });
    return pointer ? record({ status: 0, pointer }) : failed();
  },
  strongClone(c) { return count(c, 'strongClone', true); },
  weakDowngrade(c) { control(c); assert(c.$strong > 0); return count(c, 'weakDowngrade', false); },
  weakClone(c) { return count(c, 'weakClone', false); },
  strongReleaseBegin(c) {
    control(c); assert(!c.$pending && c.$strong > 0);
    c.$strong--; c.$pending = c.$strong === 0;
    countEvents.push(`strongReleaseBegin:${c.$pending ? 'last' : 'notLast'}`);
    return record({ status: 0, value: c.$pending });
  },
  strongReleaseFinish(c) {
    control(c); assert(c.$pending && c.$strong === 0);
    c.$pending = false; c.$weak--;
    if (c.$weak === 0) { release(c.$ptr, 'Shared control'); controls.delete(c.$ptr); }
    countEvents.push(`strongReleaseFinish:${c.$weak === 0 ? 'deallocated' : 'retained'}`);
    return 0;
  },
  weakRelease(c) {
    control(c); assert(!c.$pending && c.$weak > (c.$strong > 0 ? 1 : 0));
    c.$weak--; countEvents.push('weakRelease');
    const deallocated = c.$weak === 0;
    if (deallocated) { assert.equal(c.$strong, 0); release(c.$ptr, 'Shared control'); controls.delete(c.$ptr); }
    return record({ status: 0, value: deallocated });
  },
});
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')] = Object.freeze(runtime);
const module = await import(pathToFileURL(process.argv[2]).href);
assert.deepEqual(Object.keys(module), ['entry']);
function reset(next = {}) {
  assert.equal(live.size, 0); assert.equal(controls.size, 0);
  ordinal = sequence = cloneCalls = 0; fault = next; acquired = []; released = []; countEvents = [];
}
function closed(releases) {
  assert.deepEqual(released, releases); assert.equal(live.size, 0); assert.equal(controls.size, 0);
  assert.deepEqual([...released].sort((a,b) => a-b), Array.from({ length: sequence }, (_, i) => i + 1));
}
function success() {
  reset(); assert.equal(module.entry(), expected.observation);
  assert.deepEqual(acquired, expected.acquisitions);
  assert.equal(cloneCalls, expected.string_clone_calls);
  if (expected.count_events) assert.deepEqual(countEvents, expected.count_events);
  closed(expected.releases);
}
success();
for (const next of expected.faults) {
  reset(next);
  assert.throws(() => module.entry(), error => error.message === next.trap);
  closed(next.releases); success();
}
reset(); assert.throws(() => module.entry(1)); closed([]); success();
console.log(JSON.stringify({ case: expected.case, target: 'javascript', actualAllocatorIdentities: true, frozenOracle: true, controlledFaults: expected.faults.length, pristineRetries: true, productionPrivateRuntimeAbiStillOpen: true }));
