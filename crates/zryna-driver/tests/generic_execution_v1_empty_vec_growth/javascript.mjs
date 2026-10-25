import assert from 'node:assert/strict';
import {pathToFileURL} from 'node:url';
Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{get(){throw new Error('ORDINARY-USES-PRIVATE-KERNEL');}});
const module=await import(pathToFileURL(process.argv[2]).href);
assert.deepEqual(Object.keys(module),['entry']);
for(let retry=0;retry<3;retry++)assert.equal(module.entry(),18);
assert.throws(()=>module.entry(0));assert.equal(module.entry(),18);
console.log('result:18');
