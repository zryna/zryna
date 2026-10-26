// Original audited module plus an explicitly untrusted private source/counter projection.
import assert from 'node:assert/strict';
const state=ctx=>[ctx.A,ctx.L,ctx.S,ctx.H];
function probe(pre){
 let ctx,value=null,error=null,after=null,exitError=null;
 try{
  $runtimeEnter();ctx=$accountStart();[ctx.A,ctx.L,ctx.S,ctx.H]=pre;
  try{value=$aggregateEntry(ctx);}catch(failure){error=failure.message;}
  after=state(ctx);
 }catch(failure){error=failure.message;}
 finally{try{$runtimeExit();if(ctx)$accountExit(ctx);}catch(failure){exitError=failure.message;}}
 return {value,error,after,exitError};
}
const observations=$aggregateRecord.rows.map(row=>{
 const actual=probe(row.pre);assert.deepEqual(actual,{value:row.value,error:row.error,after:row.after,exitError:null},row.id);return actual;
});
// Exercise the unchanged actual public entry/reset after seeded private failures.
const publicResults=[$aggregatePublic(),$aggregatePublic()];
assert.deepEqual(publicResults,[$aggregateRecord.rows[0].value,$aggregateRecord.rows[0].value]);
const retry=probe([0,0,0,0]);
assert.deepEqual(retry,{value:$aggregateRecord.rows[0].value,error:null,after:$aggregateRecord.rows[0].after,exitError:null});
console.log(JSON.stringify({case:$aggregateRecord.case,observations,publicResults,retry,
 actualSourceAndCanonicalKernel:true,seededPrivateProjection:true,productionAuditAcceptsProjection:false,
 boundary:'same source/NEW owner and actual private entry; seeded counters are not a public max-owner-graph or physical trace proof'}));
