import assert from 'node:assert/strict';
import test from 'node:test';
import { createOwnerControl,validateOwnerReceipt } from './owner-control.mjs';
const receipt=()=>({format:'zryna.playground-ui-owner-observation.v1',version:1,qualificationOnly:true,
  name:'zryna410-ui-'+'a'.repeat(32),nonce:'a'.repeat(32),containerId:'b'.repeat(64),audit:null,kernel:[],inspect:[],
  wait:{StatusCode:0},removed:true,dockerRemoved:true,kernelReapProven:true,
  reap:{originalInitAbsent:true,cgroup:'removed'},failure:null,cleanupFailure:null,ownershipUnconfirmed:false});
const encode=value=>Buffer.from(JSON.stringify(value)+'\n');
const reserve=control=>control.push(encode({type:'reserved',nonce:'a'.repeat(32),name:'zryna410-ui-'+'a'.repeat(32)}));
test('closed owner acknowledgement is canonical, bounded and independently validated',()=>{
  let value;const control=createOwnerControl({onReady(){assert.fail();},onClosed(record){value=record;}});
  reserve(control);
  const bytes=encode({type:'closed',receipt:receipt()});
  for(const byte of bytes)control.push(Buffer.from([byte]));control.end();assert.deepEqual(value,receipt());
  for(const changed of [{removed:'true'},{ownershipUnconfirmed:true},{cleanupFailure:'TimeoutError'},
    {unexpected:true},{containerId:'short'},{kernelReapProven:false},{dockerRemoved:false},
    {wait:{StatusCode:0,Error:{Message:'failure'}}}]){
    assert.throws(()=>validateOwnerReceipt({...receipt(),...changed}));
  }
});
test('closed record cannot hide trailing records, BOMs, partial input or missing closure',()=>{
  for(const bytes of [Buffer.concat([encode({type:'closed',receipt:receipt()}),Buffer.from('x')]),
    Buffer.concat([Buffer.from([239,187,191]),encode({type:'closed',receipt:receipt()})]),Buffer.from('{')]){
    const control=createOwnerControl({onReady(){},onClosed(){}});
    reserve(control);
    assert.throws(()=>{control.push(bytes);control.end();});
  }
  assert.throws(()=>createOwnerControl({onReady(){},onClosed(){}}).end());
});
test('duplicate ready and records after closure are rejected',()=>{
  const ready={type:'ready',nonce:'a'.repeat(32),containerId:'b'.repeat(64),
    audit:{version:1,rootSha256:'c'.repeat(64),files:419,bytes:548525078}};
  const control=createOwnerControl({onReady(){},onClosed(){}});reserve(control);control.push(encode(ready));
  assert.throws(()=>control.push(encode(ready)));
  const closed=createOwnerControl({onReady(){},onClosed(){}});reserve(closed);closed.push(encode({type:'closed',receipt:receipt()}));
  assert.throws(()=>closed.push(encode(ready)));
});
test('reservation precedes ownership and every later record binds to its exact nonce',()=>{
  const control=createOwnerControl({onReady(){},onClosed(){}});
  assert.throws(()=>control.push(encode({type:'closed',receipt:receipt()})));
  const reserved=createOwnerControl({onReady(){},onClosed(){}});reserve(reserved);
  assert.throws(()=>reserved.push(encode({type:'closed',receipt:{...receipt(),nonce:'d'.repeat(32)}})));
});
