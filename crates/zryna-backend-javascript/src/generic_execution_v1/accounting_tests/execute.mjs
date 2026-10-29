// Actual ordinary artifact and fourteen unchanged independent frontier rows; no event-sized log.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [path,caseName]=process.argv.slice(2);
const bytes=readFileSync(new URL('../../../../../tests/m7-generic-execution-native-accounting-string/RUNTIME_EXPECTED_FREEZE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'fe2636923a20fd8ca2c6fb176aedc0eaf91d607bcd584555be335c38c4dee490');
const oracle=JSON.parse(bytes);
assert.deepEqual(oracle.limits,{A:1048576,L:1048576,S:4194304});
const runs=oracle.runs.filter(row=>row.case===caseName);assert(runs.length>0);
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{
  get(){throw new Error('ordinary artifact attempted private test runtime injection');}
});
const module=await import(pathToFileURL(path).href);
assert.deepEqual(Object.keys(module),['entry']);
const retry=runs.find(row=>row.id==='small');assert(retry);
let invocations=0,denials=0;
for(const run of runs){
  const expected=run.expected_per_invocation;
  for(let invocation=0;invocation<run.invocations_in_same_process;invocation++){
    if(expected.result===null){
      assert.equal(expected.source_failure_status,2);
      assert.throws(()=>module.entry(...run.entry_arguments_per_invocation),error=>
        error instanceof Error&&error.message==='zryna.trap.capacity-v1',`${caseName}/${run.id}: exact source CAPACITY frontier`);
      denials++;
      // A retry on this same imported ordinary module must start a fresh invocation.
      assert.equal(module.entry(...retry.entry_arguments_per_invocation),retry.expected_per_invocation.result);
    }else{
      assert.equal(expected.source_failure_status,0);
      assert.equal(module.entry(...run.entry_arguments_per_invocation),expected.result,`${caseName}/${run.id}/${invocation}`);
    }
    invocations++;
  }
}
assert.equal(denials,1);
console.log(JSON.stringify({case:caseName,rows:runs.length,invocations,denials,
  ordinaryPublicArtifact:true,pristineRetryAfterDenial:true,
  evidence:'ordinary outcome and source CAPACITY identity only; private A/L/S/H and canonical helper-call counts require separate owning controls'}));
