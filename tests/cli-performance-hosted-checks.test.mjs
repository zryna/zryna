import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { runArchitectureCheck, verifySourceAfterCollection } from '../scripts/stability-gates/cli-performance-hosted-checks.mjs';
const result = changes => ({ exitCode: 0, error: null, signal: null, elapsedMs: 1,
  stdout: Buffer.from('Architecture checked\n'), stderr: Buffer.alloc(0), ...changes });
function fixture(t) {
  const output = mkdtempSync(join(tmpdir(), 'm418-hosted-checks-'));
  t.after(() => rmSync(output, { recursive: true, force: true }));
  return output;
}
test('mandatory architecture check invokes the actual selected CLI with fixed argv outside measurement', async t => {
  const output = fixture(t), cli = join(output, 'zryna'), environment = { PATH: 'explicit' };
  await runArchitectureCheck({ root: output, output, cli, environment }, async (file, args, options) => {
    assert.equal(file, cli); assert.deepEqual(args, ['architecture', 'check']);
    assert.equal(options.root, output); assert.equal(options.env, environment); assert.equal(options.timeoutMs, 60000);
    return result();
  });
  assert.equal(JSON.parse(readFileSync(join(output, 'architecture.result.json'))).exitCode, 0);
});
test('failed architecture checks retain real raw output and status before rejecting the operator', async t => {
  const parent = fixture(t);
  for (const [index, failure] of [{ exitCode: 17 }, { error: 'ETIMEDOUT', exitCode: null },
    { signal: 'SIGKILL', exitCode: null }].entries()) {
    const output = join(parent, String(index)); mkdirSync(output);
    await assert.rejects(runArchitectureCheck({ root: output, output, cli: 'selected', environment: {} },
      async () => result(failure)));
    const saved = JSON.parse(readFileSync(join(output, 'architecture.result.json')));
    for (const [key, value] of Object.entries(failure)) assert.equal(saved[key], value);
    assert.equal(readFileSync(join(output, 'architecture.stdout.raw'), 'utf8'), 'Architecture checked\n');
  }
});
test('source postflight retains and rejects changed complete census rather than accepting the round', t => {
  const output = fixture(t), before = { commit: 'a'.repeat(40), inventorySha256: '1'.repeat(64) };
  const options = { root: output, output, tools: { git: 'selected-git' }, environment: {}, before };
  assert.throws(() => verifySourceAfterCollection(options, () => ({ ...before, inventorySha256: '2'.repeat(64) })), /source census changed/u);
  assert.equal(JSON.parse(readFileSync(join(output, 'source-after-round.json'))).inventorySha256, '2'.repeat(64));
  const second = join(output, 'matching'); mkdirSync(second);
  assert.deepEqual(verifySourceAfterCollection({ ...options, output: second }, () => ({ ...before })), before);
});
