import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { fixture } from './stability-fuzz-producer-fixture.mjs';

// Synthetic failed command outcomes exercise collector retention, not Rust execution.
test('each already completed original survives determinism, reconstruction and verification failure', async t => {
  const cases = [
    { gate: 'lexer-generated', rows: 4, mutate: records => {
      records[3].outcome.result.files[0].lexemes[0].kind = 'token:Identifier';
    }, outputCount: 2 },
    { gate: 'lexer-generated', rows: 4, mutate: records => {
      for (const index of [2, 3]) records[index].outcome.result.files[0].lexemes[0].end -= 1;
    }, outputCount: 2 },
    { gate: 'parser-token-generated', rows: 3, mutate: records => {
      records[2].outcome = { result: { schema_version: 2, files: [null], diagnostics: [] } };
    }, outputCount: 1 },
    { gate: 'parser-token-generated', rows: 4, mutate: records => {
      records[3].outcome.error.message = 'different second result';
    }, outputCount: 2 },
  ];
  for (const failure of cases) {
    const value = fixture(t, failure);
    await assert.rejects(value.collect);
    const path = resolve(value.output, `${failure.gate}.jsonl`);
    const retained = readFileSync(path, 'utf8').trimEnd().split('\n').map(line => JSON.parse(line));
    assert.equal(retained.filter(record => record.event === 'output').length, failure.outputCount);
    assert(!retained.some(record => record.event === 'complete'));
    assert(existsSync(resolve(value.output, `observed-${failure.gate}.json`)));
    assert(!existsSync(resolve(value.output, 'receipt.json')));
    assert(!existsSync(resolve(value.output, 'started-security-authorities.json')));
    assert.equal(value.calls.filter(call => call.witness?.endsWith(`${failure.gate}.jsonl`)).length, 1);
    if (failure.gate === 'lexer-generated') assert(!existsSync(resolve(value.output, 'started-parser-token-generated.json')));
    if (failure.outputCount === 1) assert.deepEqual(retained[2].outcome.result.files, [null]);
    else assert.deepEqual(retained.map(record => record.execution).filter(index => index !== undefined), [0, 1]);
  }
});
