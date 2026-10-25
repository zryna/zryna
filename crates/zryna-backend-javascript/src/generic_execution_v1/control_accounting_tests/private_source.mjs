// Existing audited ordinary body plus private test projection, no replacement runtime/counter limit.
function probe(pre){
 let value=null,error=null,exitError=null,after=null,ctx;
 try{
  $runtimeEnter();ctx=$accountStart();[ctx.A,ctx.L,ctx.S,ctx.H]=pre;
  try{value=$sourceStep(ctx);}catch(failure){error=failure.message;}
  after=[ctx.A,ctx.L,ctx.S,ctx.H];
 }catch(failure){error=failure.message;}
 finally{try{$runtimeExit();if(ctx)$accountExit(ctx);}catch(failure){exitError=failure.message;}}
 return {value,error,after,exitError};
}
const observations=$record.private_source_rows.map(row=>probe(row.pre));
const retry=probe([0,0,0,0]);
console.log(JSON.stringify({observations,retry,seededPrivateSourceEvidence:true,
 boundary:'real same-source private bodies with seeded counters; no public or million-allocation inference'}));
