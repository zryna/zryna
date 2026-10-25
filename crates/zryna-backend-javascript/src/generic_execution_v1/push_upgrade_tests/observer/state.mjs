import assert from 'node:assert/strict';

export const record = fields => Object.freeze(Object.assign(Object.create(null),fields));
export const state = {ordinal:0,sequence:0,fault:{},acquired:[],released:[],countEvents:[],reserveCalls:0,live:new Map(),controls:new Map()};
export function reset(fault={}) {
  assert.equal(state.live.size,0); assert.equal(state.controls.size,0);
  state.ordinal=state.sequence=state.reserveCalls=0;
  state.fault=fault; state.acquired=[]; state.released=[]; state.countEvents=[];
}
export function poisoned(status) {
  const result=Object.create(null);
  Object.defineProperty(result,'status',{value:status});
  for(const field of ['handle','pointer','value']) Object.defineProperty(result,field,{get(){throw new Error('UNWRITTEN-RESULT-READ');}});
  return Object.freeze(result);
}
export function allocate(kind,metadata) {
  state.ordinal++;
  if(state.fault.kind==='allocation'&&state.ordinal===state.fault.ordinal)return undefined;
  const token=record({identity:++state.sequence});
  state.live.set(token,{kind,...metadata});state.acquired.push(kind);return token;
}
export function release(token,kind) {
  const item=state.live.get(token);assert(item,'one exact live allocation');assert.equal(item.kind,kind);
  state.live.delete(token);state.released.push(token.identity);
}
export function closed(releases) {
  assert.deepEqual(state.released,releases);assert.equal(state.live.size,0);assert.equal(state.controls.size,0);
  assert.deepEqual([...state.released].sort((a,b)=>a-b),Array.from({length:state.sequence},(_,i)=>i+1));
}
