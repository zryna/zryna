// Actual ledger and actual retained Vec Clone/drop glue; canonical callbacks are private controlled doubles.
import assert from 'node:assert/strict';
const oracle=JSON.parse($heldSource.replaceAll('18446744073709551615','"18446744073709551615"'));
const layout=Object.freeze({stride:4,alignment:4,id:900});
const storage=(capacity,name='old storage')=>capacity===0?0:Object.freeze({capacity,name});
const string=name=>Object.freeze({length:1,name});
let entered=0,mode='',stringClones=0,post=null,cleanupOrder=[],peak=0;
function entry(){entered++;peak=Math.max(peak,$accountActive.L+$accountActive.positive);}
const failure=code=>Object.freeze({status:code,get handle(){throw new Error('failed output read');}});
function $vecAlloc(_layout,count){entry();if(mode==='allocate-failure')return failure(1);return Object.freeze({status:0,handle:storage(count,mode==='clone-prefix'?'new Vec storage':'new storage')});}
function $vecReserve(_layout,old,required){entry();if(required>1048576||required*_layout.stride>2147483647)return failure(2);if(mode==='reserve-failure')return failure(1);const capacity=old===0?0:old.capacity;return Object.freeze({status:0,handle:required<=capacity?old:storage(Math.max(required,capacity===0?4:capacity*2),'grown storage')});}
function $vecFree(_layout,value){entry();cleanupOrder.push(value===0?'empty storage':value.name);return 0;}
function $clone(source){entry();stringClones++;if(mode==='clone-prefix'&&stringClones===2)return failure(1);return Object.freeze({status:0,handle:string('new copied String A')});}
function $from(_bytes,length){entry();return Object.freeze({status:0,handle:Object.freeze({length})});}
function $concat(left,right){entry();return Object.freeze({status:0,handle:Object.freeze({length:left.length+right.length})});}
function $release(source){entry();cleanupOrder.push(source.name);return 0;}
function state(ctx){return Object.fromEntries(['A','L','S','H'].map(key=>[key,typeof ctx[key]==='bigint'?ctx[key].toString():ctx[key]]));}
function status(code){if(code===0)return;post=state($accountActive);throw new Error(code===1?'zryna.trap.allocation-v1':code===2?'zryna.trap.capacity-v1':'ZRYNA-RT-ABI');}
const $status_vecAlloc=status,$status_vecReserve=status,$status_vecFree=status,$status_from=status,$status_clone=status,$status_concat=status,$status_release=status;
function $handle(result,check){check(result.status);assert.equal(typeof result.handle,'object');return result.handle;}
function cleanup(ctx,target){while(ctx.H>target.H){const positive=ctx.L>target.L?1:0;$vecStorageFree(layout,storage(positive),ctx);}}
for(const row of oracle.cases){
 const ctx=$accountStart();for(const key of ['A','L','S','H'])ctx[key]=typeof row.pre[key]==='string'?BigInt(row.pre[key]):row.pre[key];
 entered=0;mode='';stringClones=0;post=null;cleanupOrder=[];peak=0;
 let caught=null,old=storage(1),vector=null,rhs=null;
 const prefix=row.id==='clone-prefix-second-string-failure',prepared=row.id==='prepared-RHS-survives-reserve-failure';
 try{
  switch(row.id){
   case 'empty-allocate-exact-held-credit':case 'empty-allocate-prospective-credit-extra':$vecNew(layout,[],0,ctx);break;
   case 'positive-allocate-attempt-exact':case 'positive-allocate-attempt-extra':$vecNew(layout,[7],1,ctx);break;
   case 'within-capacity-at-attempt-ceiling':case 'within-capacity-status-extra':old=storage(4);assert.equal($accountVecReserve(layout,old,1,ctx),old);break;
   case 'first-growth-from-empty-exact':$accountVecReserve(layout,0,1,ctx);break;
   case 'replacement-growth-exact':case 'positive-reserve-attempt-extra':$accountVecReserve(layout,old,2,ctx);break;
   case 'reserve-admitted-target-allocation-failure':mode='reserve-failure';$accountVecReserve(layout,old,2,ctx);break;
   case 'canonical-size-capacity-only-forward':$accountVecReserve(layout,old,1048577,ctx);break;
   case 'positive-storage-release-at-total-ceiling':case 'release-without-storage-debt':$vecStorageFree(layout,old,ctx);break;
   case 'empty-storage-release-spends-credit':$vecStorageFree(layout,0,ctx);break;
   case 'forged-overflow-context':$vecNew(layout,[],0,ctx);break;
   case 'zero-reserve-noop':assert.equal($accountVecReserve(layout,0,0,ctx),0);break;
   case 'clone-prefix-second-string-failure':
    mode='clone-prefix';vector={$storage:storage(2,'original Vec storage'),$items:[string('original String A'),string('original String B')],$length:2};
    $vectorClone(vector,ctx);break;
   case 'prepared-RHS-survives-reserve-failure':
    mode='reserve-failure';vector={$storage:storage(1,'old Vec storage'),$items:[string('old Vec String')],$length:1};rhs=string('pending RHS String');
    $vecPush($stringElement,vector,rhs,ctx);break;
   default:throw new Error('unhandled frozen row '+row.id);
  }
 }catch(error){caught=error;}
 const admission=row.expected.admission;
 if(admission.startsWith('DENIED')){assert.equal(caught?.message,'zryna.trap.capacity-v1',row.id);assert.equal(entered,0);}
 else if(admission.includes('ABI255')){assert.equal(caught?.message,'ZRYNA-RT-ABI',row.id);assert.equal(entered,0);}
 else if(admission.includes('ALLOCATION1'))assert.equal(caught?.message,'zryna.trap.allocation-v1',row.id);
 else if(admission.includes('canonical CAPACITY2'))assert.equal(caught?.message,'zryna.trap.capacity-v1',row.id);
 else assert.equal(caught,null,row.id);
 assert.deepEqual(prefix?post:state(ctx),row.expected.post,row.id);
 if(row.expected.temporary_live_peak!==undefined)assert.equal(peak,row.expected.temporary_live_peak,row.id);
 if(prefix){$vectorDrop(vector,ctx);assert.deepEqual(cleanupOrder,row.expected.exact_cleanup_sequence);}
 else if(prepared){assert.equal(vector.$length,1);assert.equal(vector.$items.length,1);assert.equal(vector.$items[0].name,'old Vec String');assert.equal(rhs.name,'pending RHS String');$stringDrop(rhs,ctx);$vectorDrop(vector,ctx);assert.deepEqual(cleanupOrder,row.expected.exact_cleanup_sequence);}
 else if(row.expected.after_cleanup!==null)cleanup(ctx,row.expected.after_cleanup);
 if(row.expected.after_cleanup!==null)assert.deepEqual(state(ctx),row.expected.after_cleanup,row.id);
 if(ctx.H===0&&ctx.L===0&&typeof ctx.S==='number')$accountExit(ctx);else $accountActive=null;
}
const pristine=$accountStart();assert.deepEqual(state(pristine),{A:0,L:0,S:0,H:0});$accountExit(pristine);
console.log(JSON.stringify({cases:oracle.cases.length,seededPrivateCounterEvidence:true,actualRetainedCloneAndDropGlue:true,
 boundary:'controlled canonical callbacks and seeded prestates, no source threshold or physical malloc inference'}));
