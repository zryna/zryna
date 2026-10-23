// Existing Linear32 runtime layout; monotonically distinct real allocation pointers.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';

export async function load(path) {
  const memory = new WebAssembly.Memory({ initial: 1024, maximum: 1024 });
  const bytes = new Uint8Array(memory.buffer), view = new DataView(memory.buffer);
  const decoder = new TextDecoder('utf-8', { fatal: true }), live = new Map();
  let next = 32 * 1024 * 1024, calls = 0, fault = 0, events = [], allocated = [], released = [];
  function fresh(source, out) {
    calls++;
    if (calls === fault) { events.push(`fail:${calls}`); return 1; }
    const pointer = next, role = allocated.length + 1;
    next += Math.max(1, source.length); assert(next <= bytes.length);
    bytes.set(source, pointer); live.set(pointer, { length: source.length, role }); allocated.push(pointer);
    events.push(`alloc:${role}:${decoder.decode(source)}`);
    view.setUint32(out, pointer, true); view.setUint32(out + 4, source.length, true); view.setUint32(out + 8, source.length, true);
    return 0;
  }
  const runtime = {
    memory,
    stringFromUtf8Copy(pointer, length, out) {
      const source = bytes.slice(pointer, pointer + length); decoder.decode(source); return fresh(source, out);
    },
    stringClone(pointer, length, capacity, out) {
      assert.equal(live.get(pointer)?.length, length); assert.equal(capacity, length);
      return fresh(bytes.slice(pointer, pointer + length), out);
    },
    stringRelease(pointer, length, capacity) {
      assert(live.has(pointer), 'duplicate or forged release');
      const value = live.get(pointer); assert.equal(value.length, length); assert.equal(capacity, length);
      events.push(`drop:${value.role}:${decoder.decode(bytes.subarray(pointer, pointer + length))}`);
      bytes.fill(0, pointer, pointer + length); live.delete(pointer); released.push(pointer); return 0;
    },
  };
  const { instance } = await WebAssembly.instantiate(await readFile(path), { 'zryna.generic.ownership.runtime.v1': runtime });
  assert.deepEqual(Object.keys(instance.exports), ['entry']);
  return {
    entry: instance.exports.entry,
    reset(number) { assert.equal(live.size, 0); calls = 0; fault = number; events = []; allocated = []; released = []; },
    check(expected) {
      assert.deepEqual(events, expected); assert.equal(live.size, 0);
      assert.equal(new Set(released).size, released.length); assert.equal(released.length, allocated.length);
      for (const identity of allocated) assert(released.includes(identity));
    },
  };
}
