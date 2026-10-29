import assert from 'node:assert/strict';
import test from 'node:test';
import { MAX_LOG } from '../scripts/stability-gates/input.mjs';
import { replayWitness } from '../scripts/stability-gates/fuzz-witness.mjs';
import { namedProof, rustProof } from '../scripts/stability-gates/fuzz-replay.mjs';
import { childEnvironment, inspectWithoutTokens } from '../scripts/stability-gates/fuzz-files.mjs';
import { encode, registry, rustLog, witnessRecords } from './stability-fuzz-fixture.mjs';

// These generated transcripts are synthetic mutation controls, never an actual campaign.
test('full finite witness shape replays while closure is still blocked', () => {
  for (const campaign of registry.campaigns) {
    const totals = replayWitness(encode(witnessRecords(campaign)), campaign);
    assert.equal(totals.inputs, campaign.cases);
    assert.equal(totals.outputs, campaign.cases * (campaign.id === 'lexer-generated' ? 2 : 6));
  }
  assert.equal(registry.globalM7, false);
});

test('changed originals, seeds and corpus cannot select replacement inputs', () => {
  for (const campaign of registry.campaigns) {
    for (const mutate of [
      records => { records[0].seed = '0x1'; },
      records => { (records[0].fragments ?? records[0].vocabulary).reverse(); },
      records => { records[1].text += ' '; },
      records => { records[1].path = 'replacement.zry'; },
    ]) {
      const records = witnessRecords(campaign);
      mutate(records);
      assert.throws(() => replayWitness(encode(records), campaign));
    }
  }
});

test('missing, duplicated, reordered and incomplete events fail closed', () => {
  for (const campaign of registry.campaigns) {
    for (const mutate of [
      records => { records.pop(); },
      records => { records.splice(2, 0, records[2]); },
      records => { [records[1], records[2]] = [records[2], records[1]]; },
      records => { records.at(-1).cases -= 1; },
      records => { records[2].extra = true; },
      records => { records[2].case += 1; },
      records => { records[3].outcome = null; },
    ]) {
      const records = witnessRecords(campaign);
      mutate(records);
      assert.throws(() => replayWitness(encode(records), campaign));
    }
    const bytes = encode(witnessRecords(campaign));
    assert.throws(() => replayWitness(bytes.subarray(0, -1), campaign));
    assert.throws(() => replayWitness(Buffer.concat([Buffer.from([0xff]), bytes]), campaign));
  }
});

test('lossless ranges reject gaps, overlap, empty and past-end slices', () => {
  const campaign = registry.campaigns[0];
  for (const mutate of [
    value => { value.lexemes[0].start = 1; },
    value => { value.lexemes[0].end += 1; },
    value => { value.lexemes[0].end = 0; },
    value => { value.lexemes.push({ ...value.lexemes[0] }); },
    value => { value.diagnostics.push({ code: 'ZRYNA-F2002', message: 'bad', severity: 'error', guidance: 'bad', span: { file: 0, start: 0, end: 99999 } }); },
  ]) {
    const records = witnessRecords(campaign);
    mutate({ lexemes: records[2].outcome.result.files[0].lexemes, diagnostics: records[2].outcome.result.diagnostics });
    records[3].outcome = structuredClone(records[2].outcome);
    assert.throws(() => replayWitness(encode(records), campaign));
  }
});

test('UTF-8 splits and parser version substitution cannot qualify', () => {
  const campaign = registry.campaigns[0];
  const records = witnessRecords(campaign);
  const index = records.findIndex(record => record.event === 'input' && /[^\x00-\x7f]/.test(record.text));
  assert(index > 0);
  const bytes = Buffer.from(records[index].text);
  const middle = [...bytes].findIndex(byte => (byte & 0xc0) === 0x80);
  assert(middle > 0);
  records[index + 1].outcome.result.files[0].lexemes[0].end = middle;
  records[index + 2].outcome = structuredClone(records[index + 1].outcome);
  assert.throws(() => replayWitness(encode(records), campaign));
  const parser = registry.campaigns[1];
  const grammar = witnessRecords(parser);
  grammar[2].version = 5;
  assert.throws(() => replayWitness(encode(grammar), parser));
});

test('byte budgets and duplicate JSON keys reject rather than truncate', () => {
  const campaign = registry.campaigns[0];
  assert.throws(() => replayWitness(Buffer.alloc(MAX_LOG + 1), campaign));
  const bytes = encode(witnessRecords(campaign));
  const text = bytes.toString().replace('"event":"header"', '"event":"header","event":"header"');
  assert.throws(() => replayWitness(Buffer.from(text), campaign));
});

test('exact named proof rejects zero, skipped, duplicate and failed runs', () => {
  const gate = registry.campaigns[0];
  const log = rustLog(gate);
  rustProof(log, gate);
  for (const bytes of [Buffer.from(''), Buffer.from(log.toString().replace('1 passed', '0 passed')),
    Buffer.from(log.toString().replace('0 ignored', '1 ignored')),
    Buffer.from(log.toString().replace(' ... ok', ' ... FAILED')),
    Buffer.concat([log, log]), Buffer.from(log.toString().replace(gate.test, 'replacement'))]) {
    assert.throws(() => rustProof(bytes, gate));
  }
  assert.throws(() => namedProof(`test ${gate.test} ... ok\ntest ${gate.test} ... ok\n`, gate.test));
});

test('child environment removes token and inherited witness keys while preserving proxy and CA', () => {
  const env = { GH_TOKEN: 'private', github_token: 'private', ZRYNA_M7_FUZZ_WITNESS: 'old',
    HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/trusted/ca', PATH: '/bin' };
  assert.deepEqual(childEnvironment(env), { HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/trusted/ca', PATH: '/bin' });
  assert.equal(childEnvironment(env, '/fresh/original.jsonl').ZRYNA_M7_FUZZ_WITNESS, '/fresh/original.jsonl');
});

test('synchronous Git inspection filters and restores credentials even on rejection', () => {
  const environment = { GH_TOKEN: 'private', github_token: 'private', HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/ca' };
  const original = { ...environment };
  assert.throws(() => inspectWithoutTokens(() => {
    assert.deepEqual(environment, { HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/ca' });
    throw new Error('source rejection');
  }, '/source', environment), /source rejection/);
  assert.deepEqual(environment, original);
});
