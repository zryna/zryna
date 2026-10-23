// PRIVATE observed test injection only; ordinary execution uses its separately audited kernel.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {runtime} from './observer/runtime.mjs';
import {state,reset,closed} from './observer/state.mjs';

const bytes=readFileSync(new URL('./RUNTIME_ORACLE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'a3da293f678d07e7a9949cb541fdcd933ab02459c77838fbe5ccd295d4c17da8');
const expected=JSON.parse(bytes).cases.find(row=>row.case===process.argv[3]);assert(expected);
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')]=runtime;
const module=await import(pathToFileURL(process.argv[2]).href);
assert.deepEqual(Object.keys(module),['entry']);
function success(){
  reset();assert.equal(module.entry(),expected.observation);
  assert.deepEqual(state.acquired,expected.acquisitions);
  assert.equal(state.reserveCalls,expected.reserve_calls);
  assert.deepEqual(state.countEvents,expected.count_events);
  closed(expected.releases);
}
success();
for(const fault of expected.faults){
  reset(fault);assert.throws(()=>module.entry(),error=>error.message===fault.trap);
  assert.equal(state.sequence,fault.successful_acquisitions);
  assert.deepEqual(state.countEvents,fault.count_events??[]);closed(fault.releases);success();
}
reset();assert.throws(()=>module.entry(1));closed([]);success();
console.log(JSON.stringify({case:expected.case,target:'javascript-observed',faults:expected.faults.length,pristineRetries:true,actualAllocationIdentities:true,actualCountMaxNotClaimed:true}));
