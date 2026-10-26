// Actual allocation identity and release validation; String bytes are not owner identities.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';

export async function load(path) {
  const decoder = new TextDecoder('utf-8', { fatal: true });
  const live = new Map();
  let sequence = 0, calls = 0, fault = 0, events = [], allocated = [], released = [];
  function fresh(bytes) {
    calls++;
    if (calls === fault) { events.push(`fail:${calls}`); return { status: 1 }; }
    const handle = Object.freeze({ identity: ++sequence });
    const copy = new Uint8Array(bytes), role = allocated.length + 1;
    live.set(handle, { bytes: copy, role }); allocated.push(handle);
    events.push(`alloc:${role}:${decoder.decode(copy)}`);
    return { status: 0, handle };
  }
  globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')] = Object.freeze({
    stringFromUtf8Copy(bytes, length) {
      assert(bytes instanceof Uint8Array); assert.equal(bytes.length, length); decoder.decode(bytes);
      return fresh(bytes);
    },
    stringClone(handle) {
      assert(live.has(handle), 'clone of unavailable original allocation');
      return fresh(live.get(handle).bytes);
    },
    stringRelease(handle) {
      assert(live.has(handle), 'duplicate or forged release');
      const value = live.get(handle);
      events.push(`drop:${value.role}:${decoder.decode(value.bytes)}`);
      value.bytes.fill(0); live.delete(handle); released.push(handle);
      return 0;
    },
  });
  const module = await import(pathToFileURL(path).href);
  assert.deepEqual(Object.keys(module), ['entry']);
  return {
    entry: module.entry,
    reset(number) { assert.equal(live.size, 0); calls = 0; fault = number; events = []; allocated = []; released = []; },
    check(expected) {
      assert.deepEqual(events, expected); assert.equal(live.size, 0);
      assert.equal(new Set(released).size, released.length);
      assert.equal(released.length, allocated.length);
      for (const identity of allocated) assert(released.includes(identity));
    },
  };
}
