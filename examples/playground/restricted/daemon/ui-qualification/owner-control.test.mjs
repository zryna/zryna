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
const acquisition=()=>({format:'zryna.ui-kernel-acquisition.v1',version:1,phase:'POST_PROC_AUDIT',
  status:'accepted',windowMs:1000,acceptedAttempt:2,attempts:[{
    number:1,status:'retired-task',failure:'OPEN_ENOENT_FIRST',task:323,
    before:{processes:[321,322],threads:[321,322,323],current:3},
    after:{processes:[321,322],threads:[321,322],current:2},
    targetBefore:{statAbsent:true,statusAbsent:true,state:null,ticks:null,tgid:null},
    targetAfter:{statAbsent:true,statusAbsent:true,state:null,ticks:null,tgid:null},
    initStable:true,leadersStable:true,visibleProcesses:3,unrelatedZombies:1,noEscapedOwnedDescendants:true,
    leaders:[{pid:321,ticks:'100',parent:10},{pid:322,ticks:'100',parent:321}],
  },{
    number:2,status:'accepted',failure:null,task:null,
    before:{processes:[321,322],threads:[321,322],current:2},after:{processes:[321,322],threads:[321,322],current:2},
    targetBefore:null,targetAfter:null,initStable:true,leadersStable:true,visibleProcesses:3,
    unrelatedZombies:1,noEscapedOwnedDescendants:true,
    leaders:[{pid:321,ticks:'100',parent:10},{pid:322,ticks:'100',parent:321}],
  }]});
test('bounded acquisition receipt binds every accepted and retired task epoch with typed counts',()=>{
  const value={...receipt(),acquisition:[acquisition()]};validateOwnerReceipt(value);
  const mutations=[phase=>phase.windowMs=2000,phase=>phase.version=true,phase=>phase.acceptedAttempt=true,
    phase=>phase.attempts[0].before.current=true,phase=>phase.attempts[0].visibleProcesses=false,
    phase=>phase.attempts[0].targetBefore.statAbsent=1,phase=>phase.attempts[0].leadersStable=false,
    phase=>phase.attempts[0].after.threads=[321,324],phase=>phase.attempts[0].after.current=3,
    phase=>phase.attempts[1].leaders[1].ticks='101',phase=>phase.attempts[0].failure='EACCES',
    phase=>phase.attempts[0].before.threads=[321,322,323,324],phase=>phase.attempts.push(phase.attempts[1]),
    phase=>phase.attempts[0].before.path='/proc/323/cgroup'];
  for(const mutate of mutations){const changed=structuredClone(value);mutate(changed.acquisition[0]);
    assert.throws(()=>validateOwnerReceipt(changed));}
  const second=acquisition();second.phase='POST_MATERIAL_AUDIT';
  validateOwnerReceipt({...receipt(),acquisition:[acquisition(),second]});
  second.attempts.forEach(row=>row.leaders[1].ticks='101');
  assert.throws(()=>validateOwnerReceipt({...receipt(),acquisition:[acquisition(),second]}));
});
test('retirement followed by an unobserved fatal epoch still preserves validated cleanup receipt',()=>{
  const phase=acquisition();phase.status='failed';phase.acceptedAttempt=null;
  Object.assign(phase.attempts[1],{status:'fatal',leaders:null,before:null,after:null,
    initStable:null,leadersStable:null,noEscapedOwnedDescendants:null,visibleProcesses:null,unrelatedZombies:null});
  validateOwnerReceipt({...receipt(),acquisition:[phase]});
  phase.attempts[1].status='accepted';assert.throws(()=>validateOwnerReceipt({...receipt(),acquisition:[phase]}));
});
test('optional escaping IO diagnostic has closed fields and captured numeric roles without authorizing acceptance',()=>{
  const phase=acquisition();phase.status='failed';phase.acceptedAttempt=null;phase.attempts=phase.attempts.slice(0,1);
  Object.assign(phase.attempts[0],{status:'fatal',failure:null,task:null,after:null,targetBefore:null,targetAfter:null,
    ioFailure:{errno:2,operation:'READ',kind:'PROC',field:'CGROUP',subject:'OWNED_TID',pid:323,site:'TREE_LAST_MEMBERSHIPS'}});
  const value={...receipt(),acquisition:[phase]};validateOwnerReceipt(value);
  const mutations=[io=>io.errno=true,io=>io.errno=4096,io=>io.operation='stat /secret',io=>io.kind='PATH',
    io=>io.field='private-path',io=>io.subject='FOREIGN',io=>io.pid=true,io=>io.pid=999,io=>io.pid=321,
    io=>io.site='raw private string',io=>io.path='/proc/323/cgroup',io=>{io.kind='CGROUP';io.field='PIDS_CURRENT';},
    io=>{io.subject='UNKNOWN';},io=>{io.subject='INIT';io.pid=999;}];
  for(const mutate of mutations){const changed=structuredClone(value);mutate(changed.acquisition[0].attempts[0].ioFailure);
    assert.throws(()=>validateOwnerReceipt(changed));}
  for(const io of [
    {errno:null,operation:'UNKNOWN',kind:'UNKNOWN',field:'UNKNOWN',subject:'UNKNOWN',pid:null,site:'UNKNOWN'},
    {errno:13,operation:'OPEN',kind:'PROC',field:'STATUS',subject:'OWNED_PID',pid:322,site:'TREE_LEADERS'},
    {errno:5,operation:'READ',kind:'CGROUP',field:'PIDS_CURRENT',subject:'UNKNOWN',pid:null,site:'TREE_FINAL_GROUP'},
  ]){const changed=structuredClone(value);changed.acquisition[0].attempts[0].ioFailure=io;validateOwnerReceipt(changed);}
  const initial=structuredClone(value);Object.assign(initial.acquisition[0].attempts[0].ioFailure,{subject:'INIT',pid:321});
  assert.throws(()=>validateOwnerReceipt(initial));initial.initialIdentity={pid:321};validateOwnerReceipt(initial);
  initial.initialIdentity.pid=322;assert.throws(()=>validateOwnerReceipt(initial));
  const wrongLeader=structuredClone(value);wrongLeader.initialIdentity={pid:322};
  Object.assign(wrongLeader.acquisition[0].attempts[0].ioFailure,{subject:'OWNED_PID',pid:322});
  assert.throws(()=>validateOwnerReceipt(wrongLeader));
  const accepted=acquisition();accepted.attempts.at(-1).ioFailure=value.acquisition[0].attempts[0].ioFailure;
  assert.throws(()=>validateOwnerReceipt({...receipt(),acquisition:[accepted]}));
});
