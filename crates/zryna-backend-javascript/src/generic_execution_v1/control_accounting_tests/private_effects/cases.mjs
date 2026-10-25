// Fixed20 source-independent counter oracle;19 fixed-debt wrapper rows and one model-only request.
import assert from 'node:assert/strict';
import {writeFileSync,appendFileSync} from 'node:fs';
const actualTracePath=process.argv[1]+'.events-before-checks.jsonl';writeFileSync(actualTracePath,'');
const oracle=JSON.parse($heldSource.replaceAll('18446744073709551615','"18446744073709551615"'));
const state=ctx=>[ctx.A,ctx.L,ctx.S,ctx.H];
const control=(strong=1,weak=1)=>Object.assign(Object.create(null),{$ptr:{},$strong:strong,$weak:weak,$pending:false,$payload:7});
let current=null,entered=0,reserved=null,actualRoles=[],trace=[];
function entry(role){entered++;actualRoles.push(role);reserved=state($accountActive);
if(current.role==='actual-retained-shared-drop')assert.ok(['StrongReleaseBegin','StrongReleaseFinish'].includes(role));
else assert.ok(current.role.startsWith(role),'actual canonical role '+role+' expected '+current.role);
}
function scalarStatus(code){if(code===0)return;if(code===1)throw new Error('zryna.trap.allocation-v1');if(code===2)throw new Error('zryna.trap.capacity-v1');if(code===3)throw new Error('zryna.trap.refcount-v1');throw new Error('ZRYNA-RT-ABI');}
const $status_controlAlloc=scalarStatus,$status_strongClone=scalarStatus,$status_downgrade=scalarStatus,$status_weakClone=scalarStatus,$status_strongBegin=scalarStatus,$status_strongFinish=scalarStatus,$status_weakFree=scalarStatus;
function $status_upgrade(code){if(code===5)return;scalarStatus(code);}
function $controlAlloc(){entry('RawAllocate-control');if(current.canonical_status!==0)return {status:current.canonical_status,get pointer(){throw new Error('failed output read');}};return {status:0,pointer:{}};}
function $strongClone(source){entry('StrongClone');if(current.canonical_status===0)source.$strong++;return current.canonical_status;}
function $downgrade(source){entry('WeakDowngrade');if(current.canonical_status===0)source.$weak++;return current.canonical_status;}
function $weakClone(source){entry('WeakClone');if(current.canonical_status===0)source.$weak++;return current.canonical_status;}
function $upgrade(source){entry('WeakUpgrade');if(current.canonical_status===0)source.$strong++;return current.canonical_status;}
function $strongBegin(source){entry('StrongReleaseBegin');source.$strong--;source.$pending=source.$strong===0;return {status:0,value:source.$pending};}
function $strongFinish(source){entry('StrongReleaseFinish');assert.equal(source.$pending,true);assert.equal(source.$strong,0);source.$pending=false;source.$weak--;return 0;}
function $weakFree(source){entry('WeakRelease');assert.equal(source.$pending,false);source.$weak--;return {status:0,value:source.$strong===0&&source.$weak===0};}
let fixedRows=0,overflowRows=0;
for(const row of oracle.cases){
 const ctx=$accountStart();[ctx.A,ctx.L,ctx.S,ctx.H]=row.pre;
 let owner=control(),payload=control(0,1);
 if(['weak-clone-additional-explicit-debt','weak-upgrade-live-adds-strong-begin-debt','exhausted-forward-budget-keeps-exact-cleanup-infallible'].includes(row.id))owner.$weak=2;
 if(row.id==='known-expired-boundary-no-false-owner-reservation')owner=control(0,1);
 if(row.id==='expired-two-weak-last-physical-free')owner=control(0,2);
 if(row.id==='invalid-free-without-live-block')owner=control(0,1);
 if(row.id==='known-refcount-boundary-no-false-owner-reservation')owner.$strong=4294967295;
 if(row.id==='prospective-debt-add-overflow-is-atomic-denial'){
  // This generic model request is outside every generated H0/H1/H2 control recipe.
  // Preserve its Denied model oracle; the private fixed-debt API separately rejects the invalid type/domain.
  assert.equal(row.events[0].request.new_hold,'18446744073709551615');assert.equal(row.events[0].fault,'Denied');
  assert.throws(()=>$accountAdmit(ctx,0,18446744073709551615n),e=>e.message==='ZRYNA-RT-ABI');
  assert.deepEqual(state(ctx),row.pre);overflowRows++;$accountActive=null;continue;
 }
 fixedRows++;
 for(const event of row.events){
  current=event;entered=0;reserved=null;actualRoles=[];let caught=null,value;
  // Synthetic max-count seed is not a reachable closed owner graph; restore the next model release seed explicitly.
  if(row.id==='known-refcount-boundary-no-false-owner-reservation'&&event.role.startsWith('StrongReleaseBegin'))owner.$strong=1;
  const target=event.role.includes('payload-distinct')?payload:owner;
  try{
   if(event.role==='RawAllocate-control')owner=$controlNew($controlLayout,7,ctx);
   else if(event.role.startsWith('StrongClone'))value=$sharedClone(owner,ctx);
   else if(event.role.startsWith('WeakDowngrade'))value=$countDowngrade(owner,ctx);
   else if(event.role.startsWith('WeakClone'))value=$retainedWeakClone(owner,ctx);
   else if(event.role.startsWith('WeakUpgrade'))value=$accountUpgrade(owner,ctx);
   else if(event.role.startsWith('StrongReleaseBegin'))value=$accountStrongBegin(target,ctx);
   else if(event.role.startsWith('StrongReleaseFinish'))$accountStrongFinish(target,ctx);
   else if(event.role.startsWith('WeakRelease'))$accountWeakFree(target,ctx);
   else if(event.role==='corrupt-private-context')$sharedClone(owner,ctx);
   else throw new Error('unhandled frozen role '+event.role);
  }catch(error){caught=error.message;}
  trace.push({row:row.id,role:event.role,entered,after:state(ctx),caught});
  appendFileSync(actualTracePath,JSON.stringify(trace[trace.length-1])+'\n');
  if(event.fault==='Denied'){assert.equal(caught,'zryna.trap.capacity-v1');assert.equal(entered,0);}
  else if(event.fault==='Invariant'){assert.equal(caught,'ZRYNA-RT-ABI');assert.equal(entered,0);}
  else {
   assert.equal(entered,1);assert.equal(actualRoles.length,1);assert.ok(event.role.startsWith(actualRoles[0]));
   if(event.reserved!==undefined)assert.deepEqual(reserved,event.reserved);
   if(event.canonical_status===1)assert.equal(caught,'zryna.trap.allocation-v1');
   else if(event.canonical_status===3)assert.equal(caught,'zryna.trap.refcount-v1');
   else {assert.equal(caught,null);if(event.canonical_status===5)assert.equal(value,5);}
  }
  assert.deepEqual(state(ctx),event.after,row.id+'/'+event.role);
 }
 if(ctx.H===0&&ctx.L===0)$accountExit(ctx);else $accountActive=null;
}
// Invoke the actual retained Shared destructor as one composition, not just individual wrappers.
const ctx=$accountStart();ctx.A=1;ctx.L=1;ctx.S=1;ctx.H=2;
const owner=control();current={role:'actual-retained-shared-drop'};entered=0;reserved=null;actualRoles=[];
$sharedDrop(owner,ctx);assert.equal(entered,2);assert.deepEqual(actualRoles,['StrongReleaseBegin','StrongReleaseFinish']);assert.deepEqual(state(ctx),[1,0,3,0]);$accountExit(ctx);
console.log(JSON.stringify({fixedDebtWrapperRows:fixedRows,genericOverflowModelRows:overflowRows,actualRetainedCloneDrop:true,seededPrivateEvidence:true,trace,
 boundary:'controlled canonical doubles and count/context seeds; no ordinary physical/free or closed max-count ownership inference'}));
