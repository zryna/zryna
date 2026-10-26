// Ordinary fixed-memory core and distinct current observed artifact; no imports or memory export.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {dirname,join} from 'node:path';
import {createHash} from 'node:crypto';
import {check} from './webassembly-trace.mjs';
const [path,caseName]=process.argv.slice(2),directory=dirname(path);
const bytes=readFileSync(new URL('./RUNTIME_EXPECTED_CORRECTED_V2.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'5523df410a9f3d760f9684467f33f1f03fa16b9017874c62f1ba98d50296c5d1');
const row=JSON.parse(bytes).cases.find(row=>row.case===caseName);assert(row);
function instantiate(file,observed) {
  const module=new WebAssembly.Module(readFileSync(file));
  assert.deepEqual(WebAssembly.Module.imports(module),[]);
  assert.deepEqual(WebAssembly.Module.exports(module).map(row=>[row.name,row.kind]),observed
    ? [['entry','function'],['$zryna$observation','function']] : [['entry','function']]);
  return new WebAssembly.Instance(module,{}).exports;
}
const ordinary=instantiate(path,false),observed=instantiate(join(directory,'program-observed.wasm'),true);
const getter=observed.$zryna$observation,observations=[];
function reset(fault=null) {
  const kind=fault?.kind==='allocation'?2:fault?.kind==='stringClone'?6:0;
  assert.equal(getter((0x20000000|(kind<<24)|(fault?.ordinal??1))|0),0);
}
function success(run,label) {
  reset();assert.equal(observed.entry(...run.arguments.map(Number)),run.pristine.scalar_result);
  observations.push({label,outcome:'return',...check(getter,run,null)});
}
let faults=0;
for(const run of row.runs) {
  for(let retry=0;retry<3;retry++)assert.equal(ordinary.entry(...run.arguments.map(Number)),run.pristine.scalar_result);
  success(run,`${run.label}-initial`);
  for(const fault of run.faults.filter(fault=>['allocation','stringClone'].includes(fault.kind))) {
    reset(fault);assert.throws(()=>observed.entry(...run.arguments.map(Number)),WebAssembly.RuntimeError);
    observations.push({label:run.label,outcome:'trap',fault,...check(getter,run,fault)});
    success(run,`${run.label}-retry-after-${fault.kind}-${fault.ordinal}`);faults++;
  }
}
console.log(JSON.stringify({case:caseName,target:'webassembly-ordinary-and-observed',rows:row.runs.length,faults,observations,pristineRetries:true,currentProtocolOnly:true,dualConcatSourcePointerNotClaimed:true}));
