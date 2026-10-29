import assert from 'node:assert/strict';
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { canonical } from '../scripts/stability-gates/input.mjs';
import { collectFuzz } from '../scripts/stability-gates/fuzz-produce.mjs';
import { replayFuzz } from '../scripts/stability-gates/fuzz-replay.mjs';
import { ROOT } from '../scripts/stability-gates/fuzz-registry.mjs';
import { encode, registry, rustLog, witnessRecords } from './stability-fuzz-fixture.mjs';

import { commit, fixture, source } from './stability-fuzz-producer-fixture.mjs';

test('synthetic complete producer records replay; full owner never receives witness authority', async t => {
  const value = fixture(t);
  await value.collect();
  assert.equal(value.calls.length, 7);
  const pnpm = value.calls[2];
  assert.equal(pnpm.executable, process.execPath);
  assert.deepEqual(pnpm.args, [resolve(value.parent, 'bin/pnpm.cjs'), '--version']);
  assert.equal(value.calls.filter(call => call.witness).length, 2);
  assert.deepEqual(value.calls.at(-1).args, registry.fullOwner.args);
  assert(value.calls.at(-1).args.includes('--include-ignored'));
  const receipt = replayFuzz(value.output, commit);
  assert.equal(receipt.globalM7, false);
  assert.equal(receipt.closureStatus, 'blocked');
});

test('timeout and interruption retain partial originals and stop without retries', async t => {
  for (const failure of ['ETIMEDOUT', 'ECANCELED', 'CLEANUP', 'MAXOUTPUT']) {
    const value = fixture(t, failure);
    await assert.rejects(value.collect);
    assert.equal(value.calls.length, 5, 'four tools then first campaign exactly once');
    assert(existsSync(resolve(value.output, 'lexer-generated.jsonl')));
    assert(existsSync(resolve(value.output, 'lexer-generated.stdout')));
    assert(existsSync(resolve(value.output, 'observed-lexer-generated.json')));
    assert(existsSync(resolve(value.output, 'failure.json')));
    assert(!existsSync(resolve(value.output, 'receipt.json')));
    assert(!existsSync(resolve(value.output, 'started-parser-token-generated.json')));
    const partial = readFileSync(resolve(value.output, 'lexer-generated.jsonl'));
    assert.equal(partial.toString().split('\n').length - 1, 2);
    assert.throws(() => replayFuzz(value.output, commit));
  }
});

test('create-only output refuses existing content and retains it unchanged', async t => {
  const value = fixture(t);
  writeFileSync(resolve(value.parent, 'sentinel'), 'original');
  await assert.rejects(() => collectFuzz(value.parent, commit, { root: ROOT }));
  assert.equal(readFileSync(resolve(value.parent, 'sentinel'), 'utf8'), 'original');
  assert.equal(value.calls.length, 0);
});

test('source drift after a campaign retains all existing originals and blocks publication', async t => {
  const value = fixture(t);
  let inspections = 0;
  await assert.rejects(() => collectFuzz(value.output, commit, {
    root: ROOT, interruption: value.interruption,
    resolvePnpmScript: () => resolve(value.parent, 'bin/pnpm.cjs'),
    inspect: () => ({ ...source, tree: ++inspections > 2 ? 'd'.repeat(40) : source.tree }),
    run: async (executable, args, options) => {
      if (args.at(-1) === '--version') {
        const name = executable === process.execPath ? 'pnpm' : executable;
        const version = registry.toolchains[name];
        return { exitCode: 0, signal: null, error: null, elapsedMs: 1,
          stdout: Buffer.from(['cargo', 'rustc'].includes(name) ? `${name} ${version}` : version), stderr: Buffer.alloc(0) };
      }
      const gate = registry.campaigns[0];
      writeFileSync(options.env.ZRYNA_M7_FUZZ_WITNESS, encode(witnessRecords(gate)), { flag: 'wx' });
      return { exitCode: 0, signal: null, error: null, elapsedMs: 1, stdout: rustLog(gate), stderr: Buffer.alloc(0) };
    },
  }));
  assert(existsSync(resolve(value.output, 'result-lexer-generated.json')));
  assert(!existsSync(resolve(value.output, 'receipt.json')));
});

test('retained receipt mutations cannot waive execution, source, tools, outputs or closure', async t => {
  const value = fixture(t);
  await value.collect();
  const path = resolve(value.output, 'receipt.json');
  const original = JSON.parse(readFileSync(path, 'utf8'));
  for (const mutate of [
    receipt => { receipt.globalM7 = true; },
    receipt => { receipt.closureStatus = 'passed'; },
    receipt => { receipt.source.commit = 'd'.repeat(40); },
    receipt => { receipt.source.tree = 'd'.repeat(40); },
    receipt => { receipt.host = 'unknown'; },
    receipt => { receipt.tools[0].version = 'replacement'; },
    receipt => { receipt.tools.find(tool => tool.name === 'pnpm').executable = 'cmd.exe'; },
    receipt => { receipt.tools.find(tool => tool.name === 'pnpm').args[0] = 'pnpm.cmd'; },
    receipt => { receipt.tools.find(tool => tool.name === 'pnpm').args.push('--ignored'); },
    receipt => { receipt.results[0].attempt.signal = 'SIGTERM'; },
    receipt => { receipt.results[0].attempt.error = 'CLEANUP'; },
    receipt => { receipt.results[0].attempt.exitCode = 1; },
    receipt => { receipt.results[0].attempt.elapsedMs = 900001; },
    receipt => { receipt.results[0].args.push('--ignored'); },
    receipt => { receipt.results[0].stdout.sha256 = 'd'.repeat(64); },
    receipt => { receipt.results[0].totals.inputs -= 1; },
    receipt => { receipt.results.reverse(); },
    receipt => { receipt.results.pop(); },
    receipt => { receipt.results.push(structuredClone(receipt.results[0])); },
    receipt => { receipt.waiver = true; },
  ]) {
    const receipt = structuredClone(original);
    mutate(receipt);
    writeFileSync(path, canonical(receipt));
    assert.throws(() => replayFuzz(value.output, commit));
  }
  writeFileSync(path, canonical(original));
  writeFileSync(resolve(value.output, 'lexer-generated.stdout'), 'substituted raw log');
  assert.throws(() => replayFuzz(value.output, commit));
});
