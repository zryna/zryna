import assert from 'node:assert/strict';
import {state,record,poisoned,allocate,release} from './state.mjs';
import {counts} from './counts.mjs';

function element(layout) {
  assert.equal(Object.getPrototypeOf(layout),null);
  assert(Number.isSafeInteger(layout.id)&&layout.id>=0);
  assert.equal(layout.stride,4);assert.equal(layout.alignment,4);
}
function unexpected(){throw new Error('UNEXPECTED-RUNTIME-OPERATION');}
export const runtime=Object.freeze(Object.assign(Object.create(null),counts,{
  stringFromUtf8Copy:unexpected,stringClone:unexpected,stringConcat:unexpected,stringRelease:unexpected,
  grow:unexpected,release:unexpected,
  allocate(size,alignment){
    assert.equal(size,12);assert.equal(alignment,4);
    const pointer=allocate('Shared control',{size,alignment});
    return pointer?record({status:0,pointer}):poisoned(1);
  },
  vecAllocate(layout,capacity){
    element(layout);assert.equal(capacity,1);
    const handle=allocate('source Vec buffer',{layout,capacity});
    return handle?record({status:0,handle}):poisoned(1);
  },
  vecReserve(layout,storage,required){
    element(layout);const old=state.live.get(storage);assert(old);
    assert.equal(old.kind,'source Vec buffer');assert.equal(old.layout,layout);
    assert.equal(old.capacity,1);assert.equal(required,2);state.reserveCalls++;
    if(state.fault.kind==='reserve-capacity-status')return poisoned(2);
    const handle=allocate('replacement Vec buffer',{layout,capacity:2});
    if(!handle)return poisoned(1);
    release(storage,'source Vec buffer');return record({status:0,handle});
  },
  vecReleaseStorage(layout,storage){
    element(layout);const item=state.live.get(storage);assert(item);assert.equal(item.layout,layout);
    release(storage,item.kind);return 0;
  },
}));
