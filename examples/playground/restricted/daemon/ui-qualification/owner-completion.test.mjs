import assert from 'node:assert/strict';
import test from 'node:test';
import { spawn } from 'node:child_process';
import { createOwnerControl } from './owner-control.mjs';
import { createOwnerCompletion } from './owner-completion.mjs';
const deferred=()=>{let resolve;const promise=new Promise(value=>{resolve=value;});return{promise,resolve};};
function fixture({failSession=false}={}){
  const closed=deferred(),childClosed=deferred(),counts={session:0,launcher:0,cleanup:0,stop:0,end:0};
  let receipt=null;
  const child={pid:123,stdio:[null,null,null,{destroyed:false,write(value){
    const record=JSON.parse(value);assert.deepEqual(Object.keys(record),['type','deadlineNs']);
    assert.equal(record.type,'stop');assert.match(record.deadlineNs,/^[0-9]+$/);counts.stop++;
  }}],
    stdin:{end(){counts.end++;}}};
  const completion=createOwnerCompletion({child,closed:closed.promise,childClosed:childClosed.promise,
    captured:{root:'/tmp/retained-owner',cleanup(){counts.cleanup++;}},
    launcher:{async stop(){counts.launcher++;},recovery(){return{host:null};}},
    session:()=>({async stop(){counts.session++;if(failSession)throw new Error('session-primary');}}),
    receipt:()=>receipt,readyRecord:()=>({containerId:'a'.repeat(64)})});
  return{completion,counts,closed,childClosed,ack(value={removed:true,ownershipUnconfirmed:false,cleanupFailure:null}){
    receipt=value;closed.resolve(value);
  }};
}
test('cleanup memoizes concurrent callers and waits for real child close after acknowledgement',async()=>{
  const f=fixture(),first=f.completion.stop();assert.equal(f.completion.stop(),first);
  f.ack();await new Promise(resolve=>setImmediate(resolve));assert.equal(f.counts.cleanup,0);
  f.childClosed.resolve({code:0,termination:null});await first;
  assert.deepEqual(f.counts,{session:1,launcher:1,cleanup:1,stop:0,end:1});
  assert.equal(f.completion.recovery().helperSnapshot,null);
});
test('session failure still removes owner, drains helper and retains primary cleanup error',async()=>{
  const f=fixture({failSession:true});const stopped=f.completion.stop();
  f.ack();f.childClosed.resolve({code:0,termination:null});await assert.rejects(stopped,/OWNER-CLEANUP/);
  assert.equal(f.counts.launcher,1);assert.equal(f.counts.cleanup,1);
});
test('unknown cleanup remains poisoned and never releases its helper snapshot',async()=>{
  const f=fixture();const stopped=f.completion.stop();
  f.ack({removed:false,ownershipUnconfirmed:true,cleanupFailure:'TimeoutError'});
  f.childClosed.resolve({code:70,termination:null});await assert.rejects(stopped);
  assert.equal(f.completion.stop(),stopped);assert.equal(f.counts.cleanup,0);
  assert.equal(f.completion.recovery().ownershipUnconfirmed,true);
  assert.equal(f.completion.recovery().helperSnapshot,'/tmp/retained-owner');
});
test('missing real closed acknowledgement has one shared deadline and cannot release late',async()=>{
  const f=fixture();const start=performance.now(),stopped=f.completion.stop();
  f.childClosed.resolve({code:0,termination:null});await assert.rejects(stopped,/OWNER-CLEANUP/);
  assert.ok(performance.now()-start<2600);assert.equal(f.completion.stop(),stopped);
  f.ack();await new Promise(resolve=>setImmediate(resolve));assert.equal(f.counts.cleanup,0);
  assert.equal(f.completion.recovery().ownershipUnconfirmed,true);
});
test('actual inert helper closes with parent stdin open and cleanup waits for drained child close',async()=>{
  const closed=deferred(),childClosed=deferred();let acknowledged=null,drained=false,released=false;
  const record={format:'zryna.playground-ui-owner-observation.v1',version:1,qualificationOnly:true,
    name:'zryna410-ui-'+'a'.repeat(32),nonce:'a'.repeat(32),containerId:null,audit:null,kernel:[],inspect:[],
    wait:null,removed:true,dockerRemoved:false,kernelReapProven:false,reap:{noCreateAttempt:true},
    failure:null,cleanupFailure:null,ownershipUnconfirmed:false};
  // This is a fixed inert test child, never the Engine owner or acquired code.
  const script="const fs=require('node:fs');fs.writeSync(3,process.argv[1]+'\\n');process.stdin.pause();";
  const reserved={type:'reserved',nonce:record.nonce,name:record.name};
  const child=spawn(process.execPath,['-e',script,JSON.stringify(reserved)+'\n'+JSON.stringify({type:'closed',receipt:record})],
    {env:{},stdio:['pipe','pipe','pipe','pipe'],shell:false});
  const control=createOwnerControl({onReady(){assert.fail();},onClosed(value){acknowledged=value;closed.resolve(value);}});
  child.stdio[3].on('data',bytes=>control.push(bytes));child.stdio[3].on('end',()=>control.end());
  child.stdout.resume();child.stderr.resume();child.once('close',(code,termination)=>{
    drained=true;childClosed.resolve({code,termination});
  });
  await closed.promise;assert.equal(child.stdin.writableEnded,false);
  const completion=createOwnerCompletion({child,closed:closed.promise,childClosed:childClosed.promise,
    captured:{root:'/tmp/inert-child-fixture',cleanup(){assert.equal(drained,true);released=true;}},
    launcher:{async stop(){},recovery(){return{host:null};}},session:()=>null,
    receipt:()=>acknowledged,readyRecord:()=>null});
  await completion.stop();assert.equal(released,true);
});
