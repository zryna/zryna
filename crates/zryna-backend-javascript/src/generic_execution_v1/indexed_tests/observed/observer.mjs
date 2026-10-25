import assert from 'node:assert/strict';
const record=fields=>Object.freeze(Object.assign(Object.create(null),fields));
const live=new Map();
let fault=0,identity=0;
export const state={trace:[],acquired:[],released:[],physicalAttempts:0};
export function reset(nextFault=0){
  assert.equal(live.size,0,'pristine entry cannot hide a prior leaked owner');
  fault=nextFault;identity=0;state.trace=[];state.acquired=[];state.released=[];state.physicalAttempts=0;
}
function failed(){
  const result=Object.create(null);Object.defineProperty(result,'status',{value:1});
  for(const field of ['pointer','handle','value'])Object.defineProperty(result,field,{
    get(){throw new Error('UNWRITTEN-RESULT-READ');}
  });
  return Object.freeze(result);
}
function event(row){assert(state.trace.length<11,'finite fixture canonical-call bound');state.trace.push(row);}
function allocate(row,metadata,kind){
  assert(state.physicalAttempts<5,'finite fixture physical-attempt bound');state.physicalAttempts++;
  if(state.physicalAttempts===fault){event({...row,status:1});return undefined;}
  const owner=++identity;const token=record({identity:owner});live.set(token,{...metadata,kind});
  state.acquired.push({owner,kind,...(kind==='String'?{utf8:row.utf8,source:row.source??0}:{capacity:row.capacity})});
  event({...row,status:0,owner});return token;
}
function drop(token,kind){
  const owner=live.get(token);assert(owner,'only a successful live owner can be released');assert.equal(owner.kind,kind);
  live.delete(token);state.released.push(token.identity);
}
function source(token){const row=live.get(token);assert.equal(row?.kind,'String');return row.bytes;}
export function closed(){
  assert.equal(live.size,0,'all initialized source owners cleaned');
  assert.deepEqual([...state.released].sort((a,b)=>a-b),state.acquired.map(row=>row.owner));
}
export function runtime(metadata){
  function element(layout){
    assert.equal(Object.getPrototypeOf(layout),null);
    const expected=metadata.elements.find(row=>row.id===layout.id);assert(expected,'sealed element identity');
    assert.equal(layout.stride,expected.stride);assert.equal(layout.alignment,expected.alignment);
  }
  function unexpected(){throw new Error('UNEXPECTED-CANONICAL-ROLE');}
  return record({
    allocate:unexpected,grow:unexpected,release:unexpected,stringConcat:unexpected,
    strongClone:unexpected,weakDowngrade:unexpected,weakClone:unexpected,weakUpgrade:unexpected,
    strongReleaseBegin:unexpected,strongReleaseFinish:unexpected,weakRelease:unexpected,
    stringFromUtf8Copy(bytes,length){
      assert(bytes instanceof Uint8Array);assert.equal(bytes.length,length);
      new TextDecoder('utf8',{fatal:true}).decode(bytes);
      const copy=new Uint8Array(bytes);
      const handle=allocate({op:'stringFromUtf8Copy',utf8:Buffer.from(copy).toString('hex')},{bytes:copy},'String');
      return handle?record({status:0,handle}):failed();
    },
    stringClone(handle){
      const copy=new Uint8Array(source(handle));
      const cloned=allocate({op:'stringClone',source:handle.identity,utf8:Buffer.from(copy).toString('hex')},{bytes:copy},'String');
      return cloned?record({status:0,handle:cloned}):failed();
    },
    stringRelease(handle){
      source(handle);event({op:'stringRelease',owner:handle.identity,status:0});drop(handle,'String');return 0;
    },
    vecAllocate(layout,capacity){
      element(layout);assert(Number.isSafeInteger(capacity)&&capacity>=0&&capacity<=1048576);
      if(capacity===0){event({op:'vecAllocate',capacity:0,status:0,owner:0});return record({status:0,handle:0});}
      const handle=allocate({op:'vecAllocate',capacity},{layout,capacity},'Vec buffer');
      return handle?record({status:0,handle}):failed();
    },
    vecReserve(layout,storage,required){
      element(layout);const old=storage===0?{capacity:0}:live.get(storage);
      assert(old&&(!storage||old.layout===layout));assert(Number.isSafeInteger(required)&&required>old.capacity);
      let capacity=old.capacity===0?4:old.capacity*2;while(capacity<required)capacity*=2;
      if(capacity>1048576||capacity*layout.stride>2147483647)capacity=required;
      const handle=allocate({op:'vecReserve',old:storage===0?0:storage.identity,required,capacity},{layout,capacity},'Vec buffer');
      if(!handle)return failed();if(storage!==0)drop(storage,'Vec buffer');return record({status:0,handle});
    },
    vecReleaseStorage(layout,storage){
      element(layout);if(storage!==0)assert.equal(live.get(storage)?.layout,layout);
      event({op:'vecReleaseStorage',owner:storage===0?0:storage.identity,status:0});
      if(storage!==0)drop(storage,'Vec buffer');return 0;
    }
  });
}
