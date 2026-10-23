// Real object handles, actual copied UTF8, source liveness and independent clone/allocation faults.
import assert from 'node:assert/strict';
import { pathToFileURL } from 'node:url';
export async function load(path) {
  const live = new Map(); let nextIdentity = 0, successes = 0, allocations = 0, clones = 0, fault = null, records = [];
  const hex = bytes => Buffer.from(bytes).toString('hex');
  function record(tag,id,identity,sourceIdentity,bytes) { records.push({tag,id,identity,sourceIdentity,length:bytes.length,bytes_hex:hex(bytes)}); }
  function fresh(bytes, source = null) {
    allocations++;
    if (fault?.kind === 'allocation' && allocations === fault.ordinal) { record(4,allocations,'0',source?.identity ?? '0',bytes); return {status:1}; }
    const handle = Object.freeze({identity:`js:${++nextIdentity}`}), content = new Uint8Array(bytes), id = ++successes;
    live.set(handle,{id,content}); record(source ? 2 : 1,id,handle.identity,source?.identity ?? '0',content);
    return {status:0,handle};
  }
  globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')] = Object.freeze({
    stringFromUtf8Copy(bytes,length) { assert(bytes instanceof Uint8Array); assert.equal(bytes.length,length); new TextDecoder('utf-8',{fatal:true}).decode(bytes); return fresh(bytes); },
    stringClone(handle) { assert(live.has(handle),'actual Clone source still live'); clones++; const bytes=live.get(handle).content; if(fault?.kind === 'clone' && clones === fault.ordinal) { record(5,clones,'0',handle.identity,bytes); return {status:1}; } return fresh(bytes,handle); },
    stringRelease(handle) { assert(live.has(handle),'actual acquired handle released exactly once'); const value=live.get(handle); record(3,value.id,handle.identity,'0',value.content); value.content.fill(0); live.delete(handle); return 0; },
  });
  const module = await import(pathToFileURL(path).href); assert.deepEqual(Object.keys(module),['entry']);
  return { entry:module.entry,
    reset(value) { assert.equal(live.size,0); successes=allocations=clones=0; fault=value; records=[]; },
    finish() { assert.equal(live.size,0,'no leak after success or failure'); assert(successes<=2,'no source carrier or extra String acquisition'); return {records:structuredClone(records),allocations,clones}; },
  };
}
