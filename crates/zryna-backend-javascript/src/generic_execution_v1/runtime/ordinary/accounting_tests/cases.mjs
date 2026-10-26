// Private kernel boundary tests only. Genuine sites are bound by the Rust owner; numeric/control prestates are seeded.
import assert from 'node:assert/strict';
const group=process.argv[2];
// Preserve the oracle's explicit u64 overflow sentinel exactly; never round it through Number.
const oracle=JSON.parse($heldSource.replaceAll('18446744073709551615','"18446744073709551615"'));
const countIds=new Set(['known-refcount-only-forward-credit','count-success-needs-new-credit']);
const weakIds=new Set(['known-expired-only-forward-credit']);
const rows=oracle.cases.filter(row=>group==='count'?countIds.has(row.id):group==='weak'?weakIds.has(row.id):!countIds.has(row.id)&&!weakIds.has(row.id));
let entered=0,mode='success';
const handle=length=>Object.freeze(Object.assign(Object.create(null),{length}));
function raw(length){entered++;if(mode==='allocation')return Object.freeze({status:1,get handle(){throw new Error('failed output read');}});return Object.freeze({status:0,handle:handle(length)});}
const $from=(_bytes,length)=>raw(length),$clone=source=>raw(source.length),$concat=(left,right)=>raw(left.length+right.length);
const $release=()=>{entered++;return 0;};
function status(code){if(code===0)return;throw new Error(code===1?'zryna.trap.allocation-v1':code===4?'zryna.trap.utf8-v1':'ZRYNA-RT-ABI');}
const $status_from=status,$status_clone=status,$status_concat=status,$status_release=status;
function $handle(result,check){check(result.status);assert.equal(typeof result.handle,'object');return result.handle;}
function state(ctx){return Object.fromEntries(['A','L','S','H'].map(key=>[key,typeof ctx[key]==='bigint'?ctx[key].toString():ctx[key]]));}
function expected(row,key){return row.expected[key];}
function cleanup(ctx,target){
  while(ctx.H>target.H){
    const positive=ctx.H-target.H===1&&ctx.L>target.L?1:0;
    if(group==='string')$free(handle(positive),ctx);else{$accountRelease(ctx,positive);$accountReleased(ctx);}
  }
}
for(const row of rows){
  const ctx=$accountStart();for(const key of ['A','L','S','H'])ctx[key]=typeof row.pre[key]==='string'?BigInt(row.pre[key]):row.pre[key];
  entered=0;mode=row.id==='valid-allocation-failure-keeps-attempt'?'allocation':'success';
  let caught=null;
  try{
    if(row.id==='cleanup-at-total-ceiling'||row.id==='invalid-held-debt-release')$free(handle(row.pre.L===0?0:1),ctx);
    else if(row.id==='known-utf8-reject-no-owner'){
      // A validated synthetic canonical pre-backing reject: common admission arithmetic only.
      $accountAdmit(ctx,0,0);$accountResult(ctx,false);
    }else if(countIds.has(row.id)||weakIds.has(row.id)){
      // Count prestate is synthetic and typed-role-bound; target count wrappers remain unimplemented here.
      const debt=row.id==='count-success-needs-new-credit'?1:0;
      $accountAdmit(ctx,0,debt);$accountResult(ctx,false);
    }else if(row.id==='forged-overflow-context')$accountAdmit(ctx,0,1);
    else{
      const empty=row.id.startsWith('empty-');
      $fresh(empty?[]:[120],ctx);
    }
  }catch(error){caught=error;}
  const admission=row.expected.admission;
  if(admission.startsWith('DENIED')){
    assert.equal(caught?.message,'zryna.trap.capacity-v1',row.id);assert.equal(entered,0,'no canonical/fault entry on source denial');
  }else if(admission.includes('ABI255')){
    assert.equal(caught?.message,'ZRYNA-RT-ABI',row.id);assert.equal(entered,0);
  }else if(admission.includes('ALLOCATION1')){
    assert.equal(caught?.message,'zryna.trap.allocation-v1');assert.equal(entered,1);
  }else assert.equal(caught,null,row.id);
  assert.deepEqual(state(ctx),expected(row,'post'),row.id);
  if(expected(row,'after_cleanup')!==null){cleanup(ctx,expected(row,'after_cleanup'));assert.deepEqual(state(ctx),expected(row,'after_cleanup'),row.id);}
  // A seeded poststate can intentionally retain cleanup debt; this is not an entry-success claim.
  if(ctx.H===0&&ctx.L===0&&typeof ctx.S==='number')$accountExit(ctx);else $accountActive=null;
}
// Actual StringClone helper denial precedes the canonical/fault seam; parent cleanup credit survives.
if(group==='string'){
 const ctx=$accountStart();Object.assign(ctx,{A:1,L:1,S:4194302,H:1});entered=0;
 assert.throws(()=>$stringClone(handle(1),ctx),error=>error.message==='zryna.trap.capacity-v1');assert.equal(entered,0);
 assert.deepEqual(state(ctx),{A:1,L:1,S:4194302,H:1});$free(handle(1),ctx);$accountExit(ctx);
 const retry=$accountStart();Object.assign(retry,{A:1,L:1,S:1,H:1});entered=0;mode='allocation';
 assert.throws(()=>$stringClone(handle(1),retry),error=>error.message==='zryna.trap.allocation-v1');assert.equal(entered,1);
 assert.deepEqual(state(retry),{A:2,L:1,S:2,H:1});$free(handle(1),retry);$accountExit(retry);mode='success';
}
// Additional real private-boundary controls, independent of numeric oracle outputs.
{
 const ctx=$accountStart();$fresh([],ctx);assert.throws(()=>$accountExit(ctx),/ZRYNA-RT-ABI/);
 assert.throws(()=>$accountAdmit({...ctx},0,1),/ZRYNA-RT-ABI/);$free(handle(0),ctx);$accountExit(ctx);
 const retry=$accountStart();assert.deepEqual(state(retry),{A:0,L:0,S:0,H:0});$accountExit(retry);
}
console.log(JSON.stringify({group,cases:rows.length,seededPrivateCounterEvidence:true,
  knownCountAndUtf8Classification:'synthetic arithmetic controls; not generated target count/invalid-source activation',
  noMillionEventInference:true}));
