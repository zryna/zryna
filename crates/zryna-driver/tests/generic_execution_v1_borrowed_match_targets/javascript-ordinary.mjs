import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const bytes=readFileSync(new URL('./RUNTIME_EXPECTED.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'d743e826942ba7b1f2a8cd064d332a5218c87e9e2fbd964aefcc88663fa1c1ec');
const expected=JSON.parse(bytes).cases.find(row=>row.case===process.argv[3]);assert(expected);
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{get(){throw new Error('PRIVATE-INJECTION-IS-NOT-ORDINARY-AUTHORITY');}});
const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
for(const run of expected.runs){
  for(let retry=0;retry<3;retry++)assert.equal(module.entry(...run.arguments),run.pristine.scalar_result);
  assert.throws(()=>module.entry(...run.arguments,0));
  if(run.arguments.length)assert.throws(()=>module.entry(typeof run.arguments[0]==='boolean'?0:true));
  assert.equal(module.entry(...run.arguments),run.pristine.scalar_result);
}
console.log(JSON.stringify({case:expected.case,target:'javascript-ordinary',rows:expected.runs.length,ambientInjection:false,pristineRetries:true}));
