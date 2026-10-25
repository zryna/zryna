// PRIVATE canonical17 observation provider. Ordinary modules execute separately without injection.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {pathToFileURL} from 'node:url';
import {runtime} from './observer/runtime.mjs';
import {state,reset,closed} from './observer/state.mjs';

const bytes=readFileSync(new URL('./RUNTIME_ORACLE.json',import.meta.url));
assert.equal(createHash('sha256').update(bytes).digest('hex'),'a772a260962503436a0d4fde7836f72db9a4fdc2857f79085b05f624a11d4264');
const expected=JSON.parse(bytes).cases.find(row=>row.case===process.argv[3]);assert(expected);
const metadata=JSON.parse(readFileSync(process.argv[4]));
globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')]=runtime(metadata);
const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
function success(){
  reset();assert.equal(module.entry(),expected.observation);
  assert.deepEqual(state.acquired,expected.acquisitions);assert.equal(state.clones,expected.string_clone_calls);
  assert.equal(state.concats,expected.string_concat_calls);assert.equal(state.reserves,expected.reserve_calls);
  assert.deepEqual(state.events,expected.count_events);closed(expected.releases);
}
success();
const faults=process.argv[5]==='faults'?expected.faults:[];
for(const fault of faults){
  reset(fault);assert.throws(()=>module.entry(),error=>error.message===fault.trap);
  assert.equal(state.sequence,fault.successful_acquisitions);
  assert.deepEqual(state.acquired,expected.acquisitions.slice(0,fault.successful_acquisitions));
  closed(fault.releases);success();
}
reset();assert.throws(()=>module.entry(1));closed([]);success();
console.log(JSON.stringify({case:expected.case,target:'javascript-observed',faults:faults.length,exactUtf8AndSourceIds:true,exactReleases:true,pristineRetries:true,callerSlotAddressNotObserved:true}));
