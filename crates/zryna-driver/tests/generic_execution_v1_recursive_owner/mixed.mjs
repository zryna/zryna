// The complete source oracle was frozen before either provider or target execution.
import assert from 'node:assert/strict';
import {readFileSync, writeFileSync} from 'node:fs';
import {join} from 'node:path';
const [directory] = process.argv.slice(2);
const oracle = JSON.parse(readFileSync(join(directory, 'PRE_EXECUTION_ORACLE.json')));
function load(name, observed) {
  const module = new WebAssembly.Module(readFileSync(join(directory, name)));
  assert.deepEqual(WebAssembly.Module.imports(module), []);
  assert.deepEqual(WebAssembly.Module.exports(module), observed ? [
    {name: 'entry', kind: 'function'}, {name: '$zryna$observation', kind: 'function'}
  ] : [{name: 'entry', kind: 'function'}]);
  return new WebAssembly.Instance(module, {}).exports;
}
const ordinary = load('program.wasm', false);
const observed = load('program.observed.wasm', true);
const getter = observed.$zryna$observation;
const actual = [];
function trace(expected, fault) {
  const status = getter(0);
  const count = getter(0x40000000);
  assert.ok(count >= 0 && count <= 4096, 'complete bounded String projection');
  const words = Array.from({length: count}, (_, i) => getter(0x40000001 + i) >>> 0);
  actual.push({kind: fault?.kind ?? 'pristine', ordinal: fault?.ordinal ?? null, status, words});
  assert.equal(status, fault?.wasm_status_getter_0 ?? 0);
  const acquired = [], released = [], failures = [];
  const live = new Map();
  for (let at = 0; at < words.length;) {
    assert.ok(at + 5 <= words.length);
    const [kind, id, pointer, source, length] = words.slice(at, at + 5);
    at += 5;
    assert.ok(length <= words.length - at);
    const bytes = words.slice(at, at + length);
    at += length;
    assert.ok(bytes.every(b => b <= 255));
    if (kind === 1 || kind === 2) {
      const row = oracle.pristine_allocations[id - 1];
      assert.equal(row.kind, 'String');
      assert.deepEqual(bytes, row.utf8);
      assert.equal(kind, row.role === 'StringClone' ? 2 : 1);
      assert.equal(source, row.source_id ? live.get(row.source_id).pointer : 0);
      assert.ok(pointer > 0 && ![...live.values()].some(r => r.pointer === pointer));
      assert.ok(!live.has(id));
      live.set(id, {pointer, bytes});
      acquired.push(id);
    } else if (kind === 3) {
      assert.deepEqual(live.get(id), {pointer, bytes});
      assert.equal(source, 0);
      live.delete(id);
      released.push(id);
    } else {
      const row = fault?.failure_String_record;
      assert.ok(row, 'Vec/control failures do not invent String trace records');
      assert.deepEqual([kind, id, pointer], [row.kind, row.id, row.pointer]);
      assert.equal(source, row.source_id ? live.get(row.source_id).pointer : 0);
      assert.deepEqual(bytes, row.utf8);
      failures.push(kind);
    }
  }
  assert.deepEqual(acquired, expected.String_projection.acquired_ids);
  assert.deepEqual(released, expected.String_projection.released_ids);
  assert.equal(failures.length, Number(Boolean(fault?.failure_String_record)));
  assert.equal(live.size, 0);
}
function pristine() {
  assert.equal(getter(0x20000001), 0, 'prior busy and live state must be empty');
  assert.equal(observed.entry(), oracle.ordinary_return_i32);
  trace(oracle.pristine, null);
  assert.equal(getter(0x20000001), 0, 'pristine cleanup releases all counted storage');
}
try {
  for (let repeat = 0; repeat < 2; repeat++) {
    const returned = ordinary.entry();
    actual.push({kind: 'ordinary', returned});
    assert.equal(returned, oracle.ordinary_return_i32);
  }
  pristine();
  for (const fault of [...oracle.StringClone_faults, ...oracle.Allocate_faults]) {
    assert.equal(getter(Number.parseInt(fault.configured_wasm_getter_argument, 16)), 0);
    assert.throws(() => observed.entry(), WebAssembly.RuntimeError);
    trace(fault, fault);
    assert.equal(getter(0x20000001), 0, 'fault cleanup releases all counted storage');
    pristine();
  }
} finally {
  writeFileSync(join(directory, 'actual-observations.json'), JSON.stringify({actual,
    limits: 'Actual mixed recursive VM results/status/reentry and exact String projection; Vec/control release order is not recorded by this observer.'}), {flag: 'wx'});
  console.log(JSON.stringify({ordinaryCalls: actual.filter(r => r.kind === 'ordinary').length,
    observedCalls: actual.filter(r => r.kind !== 'ordinary').length,
    faults: actual.filter(r => r.ordinal !== null && r.ordinal !== undefined).length,
    retained: 'actual-observations.json'}));
}
