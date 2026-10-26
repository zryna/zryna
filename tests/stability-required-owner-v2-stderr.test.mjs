import assert from 'node:assert/strict';
import test from 'node:test';
import { reconcileRequiredOwnerLogs } from '../scripts/stability-gates/required-owner-logs.mjs';
import { fixture, index } from './stability-required-owner-support.mjs';
const run = f => reconcileRequiredOwnerLogs(index, f.observation, f.context);
const failedBlock = 'running 1 test\ntest hidden_child ... FAILED\ntest result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.01s\n';
function validFixture() { const f = fixture(); assert.equal(run(f).coverageCardinality, 3); return f; }
function pristineRetry(f) {
  f.rewrite(f.observation.targets[0].stderr, '');
  assert.equal(run(f).coverageCardinality, 3);
}
test('stderr guard preserves pristine empty and ordinary CRLF diagnostics', () => {
  const f = validFixture();
  f.rewrite(f.observation.targets[0].stderr, 'compiler note\r\nordinary diagnostic\r\n');
  assert.equal(run(f).coverageCardinality, 3); pristineRetry(f);
});
test('stderr ANSI-prefixed accounting is rejected after valid baseline and pristine retry passes', () => {
  for (const section of ['defaults', 'targets']) {
    const f = validFixture();
    const hidden = failedBlock.split('\n').filter(Boolean).map(line => `\u001b[31m${line}\u001b[0m`).join('\n') + '\n';
    f.rewrite(f.observation[section][0].stderr, hidden);
    assert.throws(() => run(f), /unambiguous Rust stderr transcript/u);
    f.rewrite(f.observation[section][0].stderr, ''); pristineRetry(f);
  }
});
test('stderr bare-CR-prefixed accounting is rejected after valid baseline and pristine retry passes', () => {
  for (const section of ['defaults', 'targets']) {
    const f = validFixture();
    f.rewrite(f.observation[section][0].stderr, failedBlock.split('\n').filter(Boolean).map(line => line.replace(/^(running|test)/u, keyword => `${keyword[0]}\r${keyword.slice(1)}`)).join('\n') + '\n');
    assert.throws(() => run(f), /unambiguous Rust stderr transcript/u);
    f.rewrite(f.observation[section][0].stderr, ''); pristineRetry(f);
  }
});
