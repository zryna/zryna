import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const bytes=readFileSync(new URL('./SOURCE_EXPECTED_FREEZE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'de7308f2600e480618f98c32713ba96ae9c571a396be39a33a8184ec27c3769e');
const expected=JSON.parse(bytes).cases.find(row=>row.case==='erased-parent-fixed-array-bounds');
const record=fields=>Object.freeze(Object.assign(Object.create(null),fields));
const state={live:new Map(),ordinal:0,acquired:[],released:[],fault:0};
function reset(fault=0){assert.equal(state.live.size,0);Object.assign(state,{ordinal:0,acquired:[],released:[],fault});}
function failed(){const result=Object.create(null);Object.defineProperty(result,'status',{value:1});for(const field of ['handle','pointer','value'])Object.defineProperty(result,field,{get(){throw new Error('UNWRITTEN-RESULT-READ');}});return Object.freeze(result);}
function unexpected(){throw new Error('UNEXPECTED-CANONICAL-OPERATION');}
const runtime=Object.freeze(Object.assign(Object.create(null),{
  stringFromUtf8Copy(bytes,length){
    assert(bytes instanceof Uint8Array);assert.equal(bytes.length,length);state.ordinal++;
    if(state.ordinal===state.fault)return failed();
    const id=state.acquired.length+1,hex=Buffer.from(bytes).toString('hex');
    assert.equal(hex,expected.acquisitions[id-1].utf8_hex);
    const handle=record({id});state.live.set(handle,hex);state.acquired.push(id);return record({status:0,handle});
  },
  stringClone:unexpected,stringConcat:unexpected,
  stringRelease(handle){assert(state.live.has(handle));state.live.delete(handle);state.released.push(handle.id);return 0;},
}));
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')]=runtime;
const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
function closed(ids){assert.deepEqual(state.released,ids);assert.equal(state.live.size,0);assert.deepEqual([...state.released].sort((a,b)=>a-b),state.acquired);}
function success(row){
  reset();
  if(row.trap)assert.throws(()=>module.entry(...row.arguments),error=>error.message===row.trap);
  else assert.equal(module.entry(...row.arguments),row.i32);
  assert.deepEqual(state.acquired,expected.acquisitions.map(value=>value.id));closed(row.release_acquisition_ids);
}
for(const row of expected.observations){success(row);success(expected.observations[0]);}
for(const fault of expected.allocation_faults){
  reset(fault.ordinal);assert.throws(()=>module.entry(0),error=>error.message==='zryna.trap.allocation-v1');
  assert.equal(state.acquired.length,fault.successful_acquisitions);closed(fault.release_acquisition_ids);success(expected.observations[0]);
}
reset();assert.throws(()=>module.entry(true));closed([]);success(expected.observations[1]);
console.log(JSON.stringify({target:'javascript-observed',rows:4,faults:2,exactBounds:true,exactReleases:true,pristineRetries:true,slotAddressNotObserved:true}));
