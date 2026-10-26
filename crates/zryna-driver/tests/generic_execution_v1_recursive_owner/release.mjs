// Independent observer allocations are not compiler continuation allocations or ordinary runtime authority.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {join} from 'node:path';
import {pathToFileURL} from 'node:url';
const [directory,kind,depthText]=process.argv.slice(2);
const bytes=readFileSync(join(directory,'SOURCE_EXPECTED_FREEZE.json'));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'eb01f8297eb221c683d264fa35ab4f046f1a3ca3f7b561637090d77fd9b6ee24');
const oracle=JSON.parse(bytes);
assert(['drop-only','controlled-cleanup','clone'].includes(kind));
const depths=depthText===undefined?oracle.depths:[Number(depthText)];
assert(depths.every(depth=>oracle.depths.includes(depth))); 
const key=Symbol.for('zryna.generic.ownership.runtime.v1');
Object.defineProperty(globalThis,key,{configurable:true,get(){throw Error('ordinary output may not use private injection');}});
const ordinary=await import(pathToFileURL(join(directory,'program.mjs')).href);
const summary=[];
for(const depth of depths) {
  for(let repeat=0;repeat<2;repeat++) {
    const returned=ordinary.entry(depth);
    assert.equal(returned,oracle.all_positive_i32);
    summary.push({mode:'ordinary',depth,repeat,returned});
  }
}
let live=new Map(),acquired=[],released=[],cloneCalls=0,failClone=false;
const record=fields=>Object.freeze(Object.assign(Object.create(null),fields));
function allocate(kind,fields) {
  assert.equal(released.length,0,'these source witnesses acquire every owner before cleanup');
  const token=record({identity:acquired.length+1});
  live.set(token,{kind,...fields});acquired.push(token.identity);return token;
}
function release(token,kind) {
  const row=live.get(token);assert(row,'exact live allocation, never duplicate or foreign');
  assert.equal(row.kind,kind);live.delete(token);released.push(token.identity);
}
function failed() {
  const result=Object.create(null);result.status=1;
  for(const name of ['handle','pointer','value']) Object.defineProperty(result,name,{get(){throw Error('failed result field must not be read');}});
  return Object.freeze(result);
}
const runtime=Object.freeze(Object.assign(Object.create(null),{
  stringFromUtf8Copy(source,length) {
    assert(source instanceof Uint8Array);assert.equal(source.length,length);
    assert(length===1 && [120,109].includes(source[0]));
    return record({status:0,handle:allocate('String',{bytes:new Uint8Array(source)})});
  },
  stringClone(source) {
    const row=live.get(source);assert.equal(row?.kind,'String');
    const before=new Uint8Array(row.bytes);cloneCalls++;
    if(failClone && cloneCalls===1) {assert.deepEqual(row.bytes,before);return failed();}
    const handle=allocate('String',{bytes:new Uint8Array(row.bytes)});
    assert.deepEqual(row.bytes,before);return record({status:0,handle});
  },
  stringRelease(source) {release(source,'String');return 0;},
  vecAllocate(layout,capacity) {
    assert.equal(Object.getPrototypeOf(layout),null);
    for(const field of ['id','stride','alignment']) assert(Number.isSafeInteger(layout[field])&&layout[field]>=0);
    assert(layout.stride>0 && layout.alignment>0);assert.equal(capacity,1);
    return record({status:0,handle:allocate('Vec',{layout,capacity})});
  },
  vecReleaseStorage(layout,storage) {
    const row=live.get(storage);assert.equal(row?.layout,layout);assert.equal(row?.capacity,1);
    release(storage,'Vec');return 0;
  },
}));
Object.defineProperty(globalThis,key,{configurable:true,value:runtime});
const observed=await import(pathToFileURL(join(directory,'program.observed.mjs')).href);
for(const depth of depths) {
  for(const route of kind==='drop-only'?['pristine']:['pristine','clone-fault','pristine-retry']) {
    assert.equal(live.size,0);live=new Map();acquired=[];released=[];cloneCalls=0;failClone=route==='clone-fault';
    let returned,trap;
    try{returned=observed.entry(depth);}catch(error){trap=error.message;}
    const actual={mode:'private-observed',depth,route,returned,trap,acquired:[...acquired],released:[...released],live:live.size};
    // Retain actual events BEFORE checking the independent source formulas.
    writeFileSync(join(directory,`release-actual-${depth}-${route}.json`),JSON.stringify(actual));
    if(failClone){assert.equal(trap,'zryna.trap.allocation-v1');assert.equal(returned,undefined);}
    else {assert.equal(trap,undefined);assert.equal(returned,oracle.all_positive_i32);}
    const count=kind==='drop-only'?depth+1:(kind==='clone'?2*depth+(failClone?1:2):depth+(failClone?2:3));
    assert.deepEqual(acquired,Array.from({length:count},(_,i)=>i+1));
    const outer=Array.from({length:depth+1},(_,i)=>i+1);
    const clonePrefix=Array.from({length:depth+(failClone?0:1)},(_,i)=>2*depth+(failClone?1:2)-i);
    const expected=kind==='drop-only'?outer:(kind==='clone'?[...clonePrefix,...outer]:(failClone?[depth+2,...outer]:[depth+3,depth+2,...outer]));
    assert.deepEqual(released,expected);assert.equal(live.size,0);
    summary.push({mode:'private-observed',depth,route,returned,trap,acquisitions:acquired.length,releases:released.length,exactSourceOrder:true});
  }
}
console.log(JSON.stringify({kind,summary,limits:'Finite JS recursive Clone/Drop and controlled leaf Clone failure; other targets and full public profile remain unqualified.'}));
