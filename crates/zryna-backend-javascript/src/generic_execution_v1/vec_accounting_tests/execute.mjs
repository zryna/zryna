// Unchanged fifteen Vec oracle rows. Public outcomes do not establish private logical counters.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [path,caseName]=process.argv.slice(2);
const bytes=readFileSync(new URL('./RUNTIME_EXPECTED_FREEZE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'74e37c21295455022586c9115c6c1ed3b9e6975c1864304241660d5b55949a64');
const oracle=JSON.parse(bytes);assert.deepEqual(oracle.limits,{A:1048576,L:1048576,S:4194304});
const record=oracle.cases.find(row=>row.case===caseName);assert(record);assert.equal(record.rows.length,5);
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{
 get(){throw new Error('ordinary artifact attempted ambient test injection');}
});
const module=await import(pathToFileURL(path).href);assert.deepEqual(Object.keys(module),['entry']);
const retry=record.rows.find(row=>row.label==='small');assert(retry);
let invocations=0,denials=0;
for(const row of record.rows){
 for(let i=0;i<row.invocations_same_process;i++){
  if(row.outcome==='trap'){
   assert.equal(row.source_status,2);
   assert.throws(()=>module.entry(...row.arguments),error=>error instanceof Error&&error.message==='zryna.trap.capacity-v1',`${caseName}/${row.label}`);
   denials++;assert.equal(module.entry(...retry.arguments),retry.scalar_result);
  }else{
   assert.equal(row.source_status,0);assert.equal(module.entry(...row.arguments),row.scalar_result,`${caseName}/${row.label}/${i}`);
  }
  invocations++;
 }
}
assert.equal(denials,1);
console.log(JSON.stringify({case:caseName,rows:record.rows.length,invocations,denials,
 ordinaryPublicArtifact:true,pristineRetryAfterDenial:true,
 evidence:'ordinary outcomes only; no inference of private counters, physical malloc observations or Wasm availability'}));
