// Actual retained two-child Clone/drop helpers with real ordinary canonical String acquisition.
import assert from 'node:assert/strict';
const state=ctx=>[ctx.A,ctx.L,ctx.S,ctx.H];
function probe(row){
 let ctx,error=null,exitError=null,prepared=null,after=null,copy;
 try{
  $runtimeEnter();ctx=$accountStart();[ctx.A,ctx.L,ctx.S,ctx.H]=row.pre;
  const left=$fresh([97],ctx),right=$fresh([98],ctx),root=[left,right];
  try{copy=$aggregateArrayClone(root,ctx);}catch(failure){error=failure.message;}
  // A failed second child has already released the successful first child here.
  prepared=state(ctx);assert.equal(root[0],left);assert.equal(root[1],right);
  assert.equal(left.length,1);assert.equal(right.length,1);
  if(copy!==undefined){assert.equal(copy.length,2);assert.notEqual(copy[0],left);assert.notEqual(copy[1],right);assert.notEqual(copy[0].pointer,left.pointer);assert.notEqual(copy[1].pointer,right.pointer);$aggregateArrayDrop(copy,ctx);}
  $aggregateArrayDrop(root,ctx);after=state(ctx);
 }catch(failure){error=failure.message;}
 finally{try{$runtimeExit();if(ctx)$accountExit(ctx);}catch(failure){exitError=failure.message;}}
 return {error,prepared,after,exitError};
}
const observations=$aggregatePrefixRows.map(row=>{
 const actual=probe(row);assert.deepEqual(actual,{error:row.error,prepared:row.prepared,after:row.after,exitError:null},row.id);return actual;
});
const retry=probe($aggregatePrefixRows[0]);
assert.deepEqual(retry,{error:null,prepared:$aggregatePrefixRows[0].prepared,after:$aggregatePrefixRows[0].after,exitError:null});
console.log(JSON.stringify({observations,retry,actualRetainedArrayCloneDrop:true,actualCanonicalKernel:true,
 seededPrivateProjection:true,productionAuditAcceptsProjection:false,
 boundary:'retained genuine source recipe plus actual canonical acquired handles; seeded counters are not a new source or public ownership proof'}));
