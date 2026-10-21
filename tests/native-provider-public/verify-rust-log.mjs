import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {pathToFileURL} from 'node:url';

// Cargo accepts a filter with zero matches. It is not qualification evidence.
export function verifyRustLog(text, minimum=1) {
  assert(Number.isSafeInteger(minimum)&&minimum>0,'positive required test census');
  const results=[...text.matchAll(/^test result: (\w+)\. (\d+) passed; (\d+) failed; (\d+) ignored;/gm)];
  assert.equal(results.length,1,'exactly one actual Rust target summary required');
  const [,status,passed,failed,ignored]=results[0];
  assert.equal(status,'ok');
  assert(Number(passed)>=minimum,'required tests actually executed');
  assert.equal(Number(failed),0);
  assert.equal(Number(ignored),0,'required selected tests must execute');
  return {passed:Number(passed),failed:0,ignored:0};
}
if(process.argv[1]&&import.meta.url===pathToFileURL(process.argv[1]).href) {
  assert.equal(process.argv.length,4,'actual log path and minimum census required');
  console.log(JSON.stringify(verifyRustLog(readFileSync(process.argv[2],'utf8'),Number(process.argv[3]))));
}
