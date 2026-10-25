import assert from 'node:assert/strict';

export const record=fields=>Object.freeze(Object.assign(Object.create(null),fields));
export const state={ordinal:0,sequence:0,clones:0,concats:0,reserves:0,fault:{},acquired:[],released:[],events:[],live:new Map(),controls:new Map()};
export function reset(fault={}){
  assert.equal(state.live.size,0);assert.equal(state.controls.size,0);
  state.ordinal=state.sequence=state.clones=state.concats=state.reserves=0;
  state.fault=fault;state.acquired=[];state.released=[];state.events=[];
}
export function failed(){
  const result=Object.create(null);Object.defineProperty(result,'status',{value:1});
  for(const field of ['pointer','handle','value'])Object.defineProperty(result,field,{get(){throw new Error('UNWRITTEN-RESULT-READ');}});
  return Object.freeze(result);
}
export function acquire(event,metadata){
  state.ordinal++;
  if(state.fault.kind==='allocation'&&state.ordinal===state.fault.ordinal)return undefined;
  const token=record({identity:++state.sequence});
  state.live.set(token,{...metadata,kind:event.kind});state.acquired.push(event);return token;
}
export function release(token,kind){
  const row=state.live.get(token);assert(row,'exact successful live acquisition');assert.equal(row.kind,kind);
  state.live.delete(token);state.released.push(token.identity);
}
export function closed(releases){
  assert.deepEqual(state.released,releases);assert.equal(state.live.size,0);assert.equal(state.controls.size,0);
  assert.deepEqual([...state.released].sort((a,b)=>a-b),Array.from({length:state.sequence},(_,i)=>i+1));
}
