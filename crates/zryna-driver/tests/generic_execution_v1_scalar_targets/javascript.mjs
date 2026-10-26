import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname,join} from 'node:path';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
const [path,caseName]=process.argv.slice(2);
const bytes=readFileSync(join(dirname(path),'SOURCE_EXPECTED_FREEZE.json'));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'82f4d49efba2890e1c821298060628812e146d6bd1fb01db5124e7fd5dc91452');
const row=JSON.parse(bytes).cases.find(row=>row.case===caseName);assert.equal(row.expect,'whole-seal');
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{get(){throw new Error('ORDINARY-USES-PRIVATE-KERNEL');}});
const module=await import(pathToFileURL(path).href);assert.deepEqual(Object.keys(module),['entry']);
const observations=[];
for(const run of row.runs) {
  for(let retry=0;retry<3;retry++)assert.equal(module.entry(...run.arguments),run.i32);
  assert.throws(()=>module.entry(...run.arguments,0));
  if(run.arguments.length){const wrong=[...run.arguments];wrong[0]=typeof wrong[0]==='boolean'?0:true;assert.throws(()=>module.entry(...wrong));}
  assert.equal(module.entry(...run.arguments),run.i32);
  observations.push({arguments:run.arguments,i32:run.i32,sameModulePristineRetries:4});
}
console.log(JSON.stringify({case:caseName,target:'ordinary-javascript',observations,ambientInjection:false}));
