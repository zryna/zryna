import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';
const expected=JSON.parse(readFileSync(process.argv[3]));
const rows=[];
const receipt={case:expected.case,target:'javascript-ordinary',rows,complete:false};
try {
  Object.defineProperty(globalThis,Symbol.for('zryna.generic.ownership.runtime.v1'),{
    get(){throw new Error('PRIVATE-INJECTION-IS-NOT-ORDINARY-AUTHORITY');}
  });
  const module=await import(pathToFileURL(process.argv[2]).href);
  assert.deepEqual(Object.keys(module),['entry']);
  for(let repeat=0;repeat<2;repeat++) {
    for(const row of expected.observations) {
      let actual;
      try { actual={result:module.entry(...row.arguments)}; }
      catch(error) { actual={trap:error.message}; }
      rows.push({repeat,arguments:row.arguments,...actual});
      if(Object.hasOwn(row,'result'))assert.deepEqual(actual,{result:row.result});
      else assert.deepEqual(actual,{trap:row.trap});
    }
  }
  receipt.complete=true;
} catch(error) {
  receipt.failure=error.message;
  process.exitCode=1;
} finally {
  process.stdout.write(JSON.stringify(receipt)+'\n');
}
