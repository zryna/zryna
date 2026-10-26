import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
import {createHash} from 'node:crypto';
import {runtime,state,reset,closed} from './observer.mjs';
const expected=JSON.parse(readFileSync(process.argv[3]));
const metadata=JSON.parse(readFileSync(process.argv[4]));
const rows=[];const receipt={case:expected.case,target:'javascript-observed',rows,complete:false};
const hash=value=>createHash('sha256').update(JSON.stringify(value)).digest('hex');
try {
  globalThis[Symbol.for('zryna.generic.ownership.runtime.v1')]=runtime(metadata);
  const module=await import(pathToFileURL(process.argv[2]).href);assert.deepEqual(Object.keys(module),['entry']);
  function invoke(row,label){
    reset(row.fault?.ordinal??0);
    let outcome;try{outcome={result:module.entry(...row.arguments)};}catch(error){outcome={trap:error.message};}
    const actual={label,arguments:row.arguments,fault:row.fault??null,...outcome,
      trace:state.trace,acquired:state.acquired,released:state.released,physicalAttempts:state.physicalAttempts};
    rows.push(actual);
    assert.deepEqual(outcome,Object.hasOwn(row,'result')?{result:row.result}:{trap:row.trap});
    assert.deepEqual(state.trace,row.trace);assert.deepEqual(state.acquired,row.acquired);
    assert.deepEqual(state.released,row.released);assert.equal(state.physicalAttempts,row.physicalAttempts);closed();
  }
  for(let repeat=0;repeat<2;repeat++)for(const row of expected.normal)invoke(row,`normal:${repeat}`);
  for(const row of expected.faults){
    invoke(row,'allocation-fault');
    const pristine=expected.normal.find(value=>JSON.stringify(value.arguments)===JSON.stringify(row.arguments));
    assert(pristine);invoke(pristine,'pristine-after-fault');
  }
  receipt.complete=true;receipt.exactCanonicalEvents=true;receipt.exactUtf8AndCloneSource=true;
  receipt.exactReleaseIdentityOrder=true;receipt.unpublishedFailureOutputUnread=true;receipt.pristineRetries=true;
} catch(error){receipt.failure=error.message;process.exitCode=1;}
finally {receipt.rowsSha256=hash(rows);process.stdout.write(JSON.stringify(receipt)+'\n');}
