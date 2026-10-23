import assert from 'node:assert/strict';
import { join } from 'node:path';
import { oracle, check } from './portable-oracle.mjs';
import * as javascript from './javascript.mjs';
import * as webassembly from './webassembly.mjs';
const [target,directory,caseName]=process.argv.slice(2); assert(['javascript','webassembly'].includes(target));
const row=oracle(join(directory,'SOURCE_EXPECTED.json'),caseName);
const runtime=target==='javascript' ? await javascript.load(join(directory,'program.mjs')) : await webassembly.load(join(directory,'program-ordinary.wasm'),join(directory,'program-observed.wasm'));
const observations=[];
function positive(name) { runtime.reset(null); const result=runtime.entry(); assert.equal(`i32:${result}`,row.observation); const actual=runtime.finish(); observations.push({name,outcome:'return',result,...check(row,null,actual.records)}); }
positive('initial');
for (const fault of row.faults) {
  runtime.reset(fault);
  if(target==='javascript') assert.throws(()=>runtime.entry(),error=>error.message===fault.trap);
  else assert.throws(()=>runtime.entry(),WebAssembly.RuntimeError);
  const actual=runtime.finish(); observations.push({name:`${fault.kind}-${fault.ordinal}`,outcome:'trap',fault,...check(row,fault,actual.records)});
  positive(`retry-after-${fault.kind}-${fault.ordinal}`);
}
assert.equal(observations.length,1+2*row.faults.length);
console.log(JSON.stringify({target,case:caseName,observations,actualAcquisitionsReleasedExactlyOnce:true}));
