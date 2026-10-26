import assert from 'node:assert/strict';
import {state,record,failed,acquire,release} from './state.mjs';
import {counts} from './counts.mjs';

function source(handle){const row=state.live.get(handle);assert(row&&row.kind==='String','exact initialized readable String');return row.bytes;}
function string(bytes,event){
  const data=new Uint8Array(bytes);const handle=acquire({kind:'String',utf8_hex:Buffer.from(data).toString('hex'),...event},{bytes:data});
  return handle?record({status:0,handle}):failed();
}
export function runtime(metadata){
  function element(layout){
    assert.equal(Object.getPrototypeOf(layout),null);
    const expected=metadata.elements.find(row=>row.id===layout.id);assert(expected,'actual sealed element identity');
    assert.equal(layout.stride,expected.stride);assert.equal(layout.alignment,expected.alignment);
  }
  function unexpected(){throw new Error('UNEXPECTED-RAW-RUNTIME-OPERATION');}
  return Object.freeze(Object.assign(Object.create(null),counts,{
    grow:unexpected,release:unexpected,
    stringFromUtf8Copy(bytes,length){
      assert(bytes instanceof Uint8Array);assert.equal(bytes.length,length);
      new TextDecoder('utf8',{fatal:true}).decode(bytes);return string(bytes,{source_id:0});
    },
    stringClone(handle){
      state.clones++;const before=source(handle);
      if(state.fault.kind==='stringClone'&&state.clones===state.fault.ordinal)return failed();
      return string(before,{source_id:handle.identity});
    },
    stringConcat(left,right){
      state.concats++;const first=source(left),second=source(right);
      if(state.fault.kind==='stringConcat'&&state.concats===state.fault.ordinal)return failed();
      return string(Buffer.concat([first,second]),{source_ids:[left.identity,right.identity]});
    },
    stringRelease(handle){
      source(handle);if([...state.controls.values()].some(value=>value.$pending))state.events.push('payloadDrop');
      release(handle,'String');return 0;
    },
    vecAllocate(layout,capacity){
      element(layout);assert(Number.isSafeInteger(capacity)&&capacity>=0&&capacity<=1048576);
      if(capacity===0)return record({status:0,handle:0});
      const handle=acquire({kind:'Vec buffer'},{layout,capacity});
      return handle?record({status:0,handle}):failed();
    },
    vecReserve(layout,storage,required){
      element(layout);state.reserves++;const old=storage===0?{capacity:0}:state.live.get(storage);
      assert(old&&(!storage||old.layout===layout));
      if(required<=old.capacity)return record({status:0,handle:storage});
      assert(Number.isSafeInteger(required)&&required<=1048576&&required*layout.stride<=2147483647);
      let capacity=old.capacity===0?4:old.capacity*2;while(capacity<required)capacity*=2;
      if(capacity>1048576||capacity*layout.stride>2147483647)capacity=required;
      const handle=acquire({kind:'Vec buffer'},{layout,capacity});if(!handle)return failed();
      if(storage!==0)release(storage,'Vec buffer');return record({status:0,handle});
    },
    vecReleaseStorage(layout,storage){
      element(layout);if(storage===0)return 0;assert.equal(state.live.get(storage)?.layout,layout);
      release(storage,'Vec buffer');return 0;
    },
    allocate(size,alignment){
      assert(metadata.controls.some(row=>row.size===size&&row.alignment===alignment),'actual sealed control metadata');
      const pointer=acquire({kind:'Shared control'},{size,alignment});return pointer?record({status:0,pointer}):failed();
    },
  }));
}
