// Private canonical String observer; never the ordinary module's runtime authority.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const bytes=readFileSync(new URL('./RUNTIME_EXPECTED.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'e8a1761b0bd36f22dd78f90f00cf9ed777885f48b1026146fa97dbc22200060d');
const expected=JSON.parse(bytes).cases.find(row=>row.case===process.argv[3]);assert(expected);
const record=fields=>Object.freeze(Object.assign(Object.create(null),fields));
const state={live:new Map(),ordinal:0,sequence:0,clones:0,concats:0,acquired:[],events:[],releases:[],unchanged:[],fault:null,reenter:null,reentered:false};
function reset(fault=null,reenter=null){
  assert.equal(state.live.size,0);
  Object.assign(state,{ordinal:0,sequence:0,clones:0,concats:0,acquired:[],events:[],releases:[],unchanged:[],fault,reenter,reentered:false});
}
function source(handle){const bytes=state.live.get(handle);assert(bytes,'exact successful live String handle');return bytes;}
function reentry(){
  if(state.reenter&&!state.reentered){state.reentered=true;assert.throws(state.reenter,error=>error.message==='ZRYNA-RT-REENTRY');}
}
function failed(kind,ordinal,sources){
  state.events.push(`failure:${kind}:${ordinal}`);
  const before=sources.map(handle=>Buffer.from(source(handle)).toString('hex'));
  const result=Object.create(null);Object.defineProperty(result,'status',{value:1});
  for(const field of ['handle','pointer','value'])Object.defineProperty(result,field,{get(){throw new Error('UNWRITTEN-RESULT-READ');}});
  state.unchanged=sources.map((handle,index)=>{assert.equal(Buffer.from(source(handle)).toString('hex'),before[index]);return handle.id;});
  return Object.freeze(result);
}
function acquire(data,sources){
  state.ordinal++;
  if(state.fault?.kind==='allocation'&&state.ordinal===state.fault.ordinal)return failed('allocation',state.ordinal,sources);
  const bytes=new Uint8Array(data),handle=record({id:++state.sequence});
  state.live.set(handle,bytes);
  state.acquired.push({utf8_hex:Buffer.from(bytes).toString('hex'),source_ids:sources.map(handle=>handle.id)});
  state.events.push(`acquire:${handle.id}`);return record({status:0,handle});
}
function unexpected(){throw new Error('UNEXPECTED-CANONICAL-OPERATION');}
const runtime=Object.freeze(Object.assign(Object.create(null),{
  allocate:unexpected,grow:unexpected,release:unexpected,
  stringFromUtf8Copy(bytes,length){
    reentry();assert(bytes instanceof Uint8Array);assert.equal(bytes.length,length);
    new TextDecoder('utf8',{fatal:true}).decode(bytes);return acquire(bytes,[]);
  },
  stringClone(handle){
    reentry();state.clones++;const bytes=source(handle);
    if(state.fault?.kind==='stringClone'&&state.clones===state.fault.ordinal)return failed('stringClone',state.clones,[handle]);
    return acquire(bytes,[handle]);
  },
  stringConcat(left,right){
    reentry();state.concats++;const first=source(left),second=source(right);
    if(state.fault?.kind==='stringConcat'&&state.concats===state.fault.ordinal)return failed('stringConcat',state.concats,[left,right]);
    return acquire(Buffer.concat([first,second]),[left,right]);
  },
  stringRelease(handle){source(handle);state.live.delete(handle);state.releases.push(handle.id);state.events.push(`release:${handle.id}`);return 0;},
  vecAllocate:unexpected,vecReserve:unexpected,vecReleaseStorage:unexpected,
  strongClone:unexpected,weakDowngrade:unexpected,weakClone:unexpected,weakUpgrade:unexpected,
  strongReleaseBegin:unexpected,strongReleaseFinish:unexpected,weakRelease:unexpected,
}));
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')]=runtime;
const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
function closed(releases){
  assert.deepEqual(state.releases,releases);assert.equal(state.live.size,0);
  assert.deepEqual([...state.releases].sort((a,b)=>a-b),Array.from({length:state.sequence},(_,index)=>index+1));
}
function success(run,reenter=false){
  reset(null,reenter?()=>module.entry(...run.arguments):null);
  assert.equal(module.entry(...run.arguments),run.pristine.scalar_result);
  assert.deepEqual(state.acquired,run.acquisitions);assert.deepEqual(state.events,run.pristine.events);
  assert.equal(state.clones,run.pristine.string_clone_calls);assert.equal(state.concats,run.pristine.string_concat_calls);
  assert.equal(state.ordinal,run.pristine.language_string_allocations);closed(run.pristine.release_acquisition_ids);
  if(reenter)assert.equal(state.reentered,true,'actual in-progress canonical callback attempts entry');
}
let faults=0;
for(const run of expected.runs){
  success(run);
  if(run.acquisitions.length)success(run,true);
  if(process.argv[4]==='faults')for(const fault of run.faults){
    reset(fault);
    assert.throws(()=>module.entry(...run.arguments),error=>error.message==='zryna.trap.allocation-v1');
    assert.deepEqual(state.acquired,run.acquisitions.slice(0,state.sequence));
    assert.deepEqual(state.events,fault.events);assert.deepEqual(state.unchanged,fault.unchanged_source_ids);
    closed(fault.release_acquisition_ids);success(run);faults++;
  }
  reset();assert.throws(()=>module.entry(...run.arguments,0));closed([]);success(run);
  if(run.arguments.length){reset();assert.throws(()=>module.entry(typeof run.arguments[0]==='boolean'?0:true));closed([]);success(run);}
}
console.log(JSON.stringify({case:expected.case,target:'javascript-observed',rows:expected.runs.length,faults,exactUtf8AndSourceIds:true,exactEventOrder:true,pristineRetries:true,callerSlotAddressNotObserved:true}));
