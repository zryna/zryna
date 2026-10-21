import assert from 'node:assert/strict';
import test from 'node:test';
import {verifyRustLog} from './verify-rust-log.mjs';

test('zero-match, ignored, missing, duplicate and failed Cargo targets cannot qualify',()=>{
  for(const text of [
    'test result: ok. 0 passed; 0 failed; 0 ignored; 60 filtered out;',
    'test result: ok. 1 passed; 0 failed; 1 ignored;',
    'test result: FAILED. 1 passed; 1 failed; 0 ignored;',
    'Compiling example\nFinished test profile',
    'test result: ok. 1 passed; 0 failed; 0 ignored;\ntest result: ok. 1 passed; 0 failed; 0 ignored;',
  ])assert.throws(()=>verifyRustLog(text));
  assert.throws(()=>verifyRustLog('test result: ok. 1 passed; 0 failed; 0 ignored;',2));
});
test('an actual nonempty complete successful target summary retains its executed count',()=>{
  assert.deepEqual(verifyRustLog('running 6 tests\ntest result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.25s\n',6),{passed:6,failed:0,ignored:0});
});
