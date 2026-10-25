// Exact per-case observations copied from the independent frozen literal oracle.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import * as javascript from '../generic_struct_v4_nested/javascript.mjs';
import * as webassembly from '../generic_struct_v4_nested/webassembly.mjs';

const [target, directory, caseName] = process.argv.slice(2);
assert(['javascript', 'webassembly'].includes(target));
const all = JSON.parse(await readFile(new URL('./execution-oracle.json', import.meta.url), 'utf8'));
const oracle = all.cases[caseName];
assert(oracle && oracle.decision === 'admit');
assert(Number.isInteger(oracle.allocations) && oracle.allocations >= 0 && oracle.allocations <= 4);
assert.equal(oracle.result, 7); assert.equal(oracle.retryAfterEveryFailure, true);
assert.deepEqual(Object.keys(oracle.allocationFailures), Array.from({ length: oracle.allocations }, (_, index) => String(index + 1)));
const retained = JSON.parse(await readFile(join(directory, 'execution-oracle.json'), 'utf8'));
assert.deepEqual(retained, all, 'emission retains the same fixed runtime expectations');
const runtime = await (target === 'javascript' ? javascript : webassembly).load(
  join(directory, target === 'javascript' ? 'program.mjs' : 'program.wasm'), oracle.allocations);
const observations = [];
function positive(observation) {
  runtime.reset(0); const result = runtime.entry(); assert.equal(result, oracle.result);
  observations.push({ observation, fault: 0, outcome: 'return', result, ...runtime.check(oracle.success) });
}
positive('initial');
for (let fault = 1; fault <= oracle.allocations; fault++) {
  runtime.reset(fault);
  // A returned scalar fails assert.throws, even if its value happens to be 7.
  if (target === 'javascript') assert.throws(() => runtime.entry(), /ZRYNA-RT-STATUS-1/);
  else assert.throws(() => runtime.entry(), WebAssembly.RuntimeError);
  observations.push({ observation: `fault-${fault}`, fault, outcome: 'trap',
    ...runtime.check(oracle.allocationFailures[fault]) });
  positive(`retry-after-${fault}`);
}
assert.equal(observations.length, 1 + 2 * oracle.allocations);
console.log(JSON.stringify({ target, case: caseName, observations, actualAllocationsReleasedExactlyOnce: true }));
