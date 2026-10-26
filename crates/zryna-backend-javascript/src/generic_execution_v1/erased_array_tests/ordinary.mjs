import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
const bytes=readFileSync(new URL('./SOURCE_EXPECTED_FREEZE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'de7308f2600e480618f98c32713ba96ae9c571a396be39a33a8184ec27c3769e');
const expected=JSON.parse(bytes).cases.find(row=>row.case==='erased-parent-fixed-array-bounds');
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{get(){throw new Error('PRIVATE-INJECTION-IS-NOT-ORDINARY-AUTHORITY');}});
const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
for(const row of expected.observations){
  for(let retry=0;retry<2;retry++){
    if(row.trap)assert.throws(()=>module.entry(...row.arguments),error=>error.message===row.trap);
    else assert.equal(module.entry(...row.arguments),row.i32);
    assert.equal(module.entry(0),expected.observations[0].i32);
  }
}
assert.throws(()=>module.entry(true));assert.throws(()=>module.entry(0,1));
assert.equal(module.entry(1),expected.observations[1].i32);
console.log(JSON.stringify({target:'javascript-ordinary',rows:4,exactBounds:true,pristineRetries:true,ambientRuntime:false}));
