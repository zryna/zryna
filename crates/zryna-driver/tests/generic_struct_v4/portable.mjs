// Expected results are read from the independently authored pre-compilation oracle.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import * as javascript from './javascript.mjs';
import * as webassembly from './webassembly.mjs';

const [target, directory, kind] = process.argv.slice(2);
assert(['javascript', 'webassembly'].includes(target)); assert(['copy', 'owned'].includes(kind));
const oracle = JSON.parse(await readFile(new URL('./execution-oracle.json', import.meta.url), 'utf8'));
const runtime = await (target === 'javascript' ? javascript : webassembly).load(join(directory, target === 'javascript' ? 'program.mjs' : 'program.wasm'));
let observations = 0;
function positive() {
  runtime.reset(0); assert.equal(runtime.entry(), oracle.scalar);
  runtime.check(kind === 'owned' ? oracle.success : oracle.construction); observations++;
}
positive();
if (kind === 'owned') {
  for (let fault = 1; fault <= 3; fault++) {
    runtime.reset(fault);
    if (target === 'javascript') assert.throws(() => runtime.entry(), /ZRYNA-RT-STATUS-1/);
    else assert.throws(() => runtime.entry(), WebAssembly.RuntimeError);
    runtime.check(oracle.failures[fault - 1]); observations++;
    positive();
  }
}
console.log(JSON.stringify({ target, observations, actualAllocationsReleasedExactlyOnce: true }));
