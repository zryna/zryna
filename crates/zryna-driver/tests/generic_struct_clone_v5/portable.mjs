// Project exact original pre-provider attempt/release expectations; never observe to author them.
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { closeSync, openSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import * as javascript from './javascript.mjs';
import * as webassembly from './webassembly.mjs';
const [target, directory, caseName] = process.argv.slice(2);
assert(['javascript', 'webassembly'].includes(target));
const journal = openSync(join(directory, `${target}-progress.jsonl`), 'wx', 0o600);
let journalBytes = 0;
function progress(phase, detail = {}) {
  const bytes = Buffer.from(`${JSON.stringify({ target, case: caseName, phase, ...detail })}\n`);
  assert(journalBytes + bytes.length <= 65_536, 'bounded portable progress journal');
  writeFileSync(journal, bytes); journalBytes += bytes.length;
}
progress('scriptstart');
progress('oracle', { state: 'start' });
const all = JSON.parse(await readFile(new URL('./execution-oracle.json', import.meta.url), 'utf8'));
const oracle = all.cases[caseName]; assert(oracle && oracle.accept === true);
assert.equal(oracle.result, 7); assert.equal(oracle.structCarrierAllocations, 0);
const count = oracle.allocationAttempts.length; assert(count <= 7);
assert.deepEqual(JSON.parse(await readFile(join(directory, 'execution-oracle.json'), 'utf8')), all);
progress('oracle', { state: 'checked', allocationAttempts: count });
progress('load', { state: 'start' });
const runtime = await (target === 'javascript' ? javascript : webassembly).load(
  join(directory, target === 'javascript' ? 'program.mjs' : 'program.wasm'), count);
progress('load', { state: 'complete' });
function events(fault) {
  const succeeded = fault === 0 ? count : fault - 1;
  const trace = oracle.allocationAttempts.slice(0, succeeded).map(row => `alloc:${row.attempt}:${row.bytes}`);
  if (fault) trace.push(`fail:${fault}`);
  const releases = fault ? oracle.faults[fault - 1].releaseAttemptIds : oracle.successReleaseAttemptIds;
  for (const id of releases) trace.push(`drop:${id}:${oracle.allocationAttempts[id - 1].bytes}`);
  return trace;
}
function sourceRole(attempt) {
  if (attempt.operation === 'literal') return 0;
  const [owner, ...path] = attempt.role.split('.');
  const sourceOwner = ['copied', 'first', 'second'].includes(owner) ? 'source' : owner === 'activeCopy' ? 'active' : null;
  assert(sourceOwner, 'clone role outside exact frozen literals');
  const original = oracle.allocationAttempts.find(row => row.role === [sourceOwner, ...path].join('.'));
  assert(original && original.operation === 'literal'); return original.attempt;
}
function check(fault) {
  const actual = runtime.check(events(fault));
  assert.equal(actual.operations.length, fault || count);
  for (const [index, row] of actual.operations.entries()) {
    const expected = oracle.allocationAttempts[index];
    assert.equal(row.attempt, expected.attempt); assert.equal(row.effect, expected.operation);
    assert.equal(row.sourceRole, sourceRole(expected));
    assert.equal(row.success, row.attempt !== fault);
    assert.equal(row.sourceIdentity, row.sourceRole === 0 ? 0 : actual.allocated[row.sourceRole - 1]);
    if (row.success && row.sourceRole) assert.notEqual(actual.allocated[index], row.sourceIdentity);
  }
  return actual;
}
const observations = [];
function positive(observation) {
  runtime.reset(0);
  progress('entry', { observation, fault: 0, state: 'start' });
  const result = runtime.entry(); assert.equal(result, oracle.result);
  observations.push({ observation, fault: 0, outcome: 'return', result, ...check(0) });
  progress('checked', { observation, fault: 0 });
}
positive('initial');
for (const row of oracle.faults) {
  progress('fault', { fault: row.failAttempt, state: 'start' });
  const fault = row.failAttempt; assert.equal(row.returnsScalar, false); runtime.reset(fault);
  progress('entry', { observation: `fault-${fault}`, fault, state: 'start' });
  if (target === 'javascript') assert.throws(() => runtime.entry(), /ZRYNA-RT-STATUS-1/);
  else assert.throws(() => runtime.entry(), WebAssembly.RuntimeError);
  observations.push({ observation: `fault-${fault}`, fault, outcome: 'trap', ...check(fault) });
  progress('checked', { observation: `fault-${fault}`, fault });
  progress('retry', { afterFault: fault });
  positive(`retry-after-${fault}`);
}
assert.equal(observations.length, 1 + 2 * count);
progress('complete', { observations: observations.length });
closeSync(journal);
console.log(JSON.stringify({ target, case: caseName, observations, actualAllocationsReleasedExactlyOnce: true }));
