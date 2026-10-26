// Genuine source oracle predates emission; exercise the actual closed ordinary module.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {join} from 'node:path';
const [directory, sourceCase, depthText] = process.argv.slice(2);
const depth = Number(depthText);
const oracle = JSON.parse(readFileSync(join(directory, 'SOURCE_EXPECTED_FREEZE.json')));
assert.ok(oracle.depths.includes(depth));
assert.ok(Object.hasOwn(oracle, sourceCase));
const module = new WebAssembly.Module(readFileSync(join(directory, 'program.wasm')));
assert.deepEqual(WebAssembly.Module.imports(module), []);
assert.deepEqual(WebAssembly.Module.exports(module), [{name: 'entry', kind: 'function'}]);
const instance = new WebAssembly.Instance(module, {});
const calls = [];
const privateModule = new WebAssembly.Module(readFileSync(join(directory, 'program.observed.wasm')));
assert.deepEqual(WebAssembly.Module.imports(privateModule), []);
assert.deepEqual(WebAssembly.Module.exports(privateModule), [
  {name: 'entry', kind: 'function'}, {name: '$zryna$observation', kind: 'function'}
]);
const observed = new WebAssembly.Instance(privateModule, {}).exports;
const getter = observed.$zryna$observation;
function observe(route) {
  const status = getter(0);
  const traceWords = getter(0x40000000);
  const trace = traceWords <= 4096
    ? Array.from({length: traceWords}, (_, i) => getter(0x40000001 + i) >>> 0)
    : null;
  calls.push({mode: 'private-observed', route, depth, status, traceWords, trace,
    limits: trace === null ? 'Trace overflow retained; exact order unqualified.' : 'Bounded String projection only; Vec/control events are not recorded by this protocol.'});
  checkStrings(trace, route === 'clone-fault');
  return status;
}
function checkStrings(trace, fault) {
  assert.ok(trace !== null, 'these source cases have a constant number of String records');
  const originals = sourceCase === 'controlled-cleanup' ? [[1, 120], [depth + 2, 109]] : [[1, 120]];
  const clone = sourceCase === 'clone' ? [2 * depth + 2, 120] : [depth + 3, 109];
  const acquisitions = sourceCase !== 'drop-only' && !fault ? [...originals, clone] : originals;
  const releases = sourceCase === 'clone'
    ? (fault ? [1] : [2 * depth + 2, 1])
    : sourceCase === 'controlled-cleanup'
      ? (fault ? [depth + 2, 1] : [depth + 3, depth + 2, 1]) : [1];
  const live = new Map();
  let acquired = 0, released = 0, failed = 0;
  for (let at = 0; at < trace.length;) {
    assert.ok(at + 5 <= trace.length);
    const [kind, id, pointer, source, length] = trace.slice(at, at + 5);
    at += 5;
    assert.equal(length, 1);
    assert.ok(at < trace.length);
    const byte = trace[at++];
    if (kind === 1 || kind === 2) {
      assert.deepEqual([id, byte], acquisitions[acquired++]);
      assert.ok(pointer > 0 && ![...live.values()].some(r => r.pointer === pointer));
      if (kind === 2) {
        const from = originals.at(-1)[0];
        assert.equal(source, live.get(from).pointer);
      } else assert.equal(source, 0);
      live.set(id, {pointer, byte});
    } else if (kind === 3) {
      assert.equal(id, releases[released++]);
      assert.deepEqual(live.get(id), {pointer, byte});
      assert.equal(source, 0);
      live.delete(id);
    } else {
      assert.equal(kind, 5);
      assert.ok(fault && failed++ === 0);
      assert.equal(id, 1);
      assert.equal(pointer, 0);
      const from = live.get(originals.at(-1)[0]);
      assert.equal(source, from.pointer);
      assert.equal(byte, from.byte);
    }
  }
  assert.equal(acquired, acquisitions.length);
  assert.equal(released, releases.length);
  assert.equal(failed, Number(fault));
  assert.equal(live.size, 0);
}
try {
  for (let retry = 0; retry < 2; retry++) {
    const returned = instance.exports.entry(depth);
    calls.push({mode: 'ordinary', depth, retry, returned});
    assert.equal(returned, oracle.all_positive_i32);
  }
  assert.equal(getter(0x20000001), 0);
  assert.equal(observed.entry(depth), oracle.all_positive_i32);
  assert.equal(observe('pristine'), 0);
  // The existing private configuration gate rejects any live allocation or busy entry.
  // A successful next configuration therefore checks clean entry state after cleanup.
  assert.equal(getter(0x20000001), 0);
  if (sourceCase !== 'drop-only') {
    assert.equal(getter(0x26000001), 0);
    assert.throws(() => observed.entry(depth), WebAssembly.RuntimeError);
    assert.equal(observe('clone-fault'), 2);
    assert.equal(getter(0x20000001), 0);
    assert.equal(observed.entry(depth), oracle.all_positive_i32);
    assert.equal(observe('pristine-retry'), 0);
    assert.equal(getter(0x20000001), 0);
  }
} finally {
  console.log(JSON.stringify({sourceCase, calls,
    limits: 'Scalar success, controlled failure/retry and exact String projection. This protocol does not record Vec/control events; full deep release order remains unqualified.'}));
}
