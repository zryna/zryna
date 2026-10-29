import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import test from 'node:test';
import { acceptedDTO } from '../scripts/stability-gates/fuzz-dto.mjs';
import { descriptor } from '../scripts/stability-gates/fuzz-files.mjs';
import { checkObservation } from '../scripts/stability-gates/fuzz-replay.mjs';
import { replayWitness } from '../scripts/stability-gates/fuzz-witness.mjs';
import { acceptedFixtures, acceptedWitness } from './stability-fuzz-dto-fixture.mjs';
import { encode, registry, rustLog } from './stability-fuzz-fixture.mjs';

test('original independent accepted v2/v3/v4 DTOs satisfy detached structural/source-range checks', () => {
  const versions = new Set();
  for (const entry of acceptedFixtures()) {
    acceptedDTO(entry.raw, entry.source, entry.version);
    versions.add(entry.version);
  }
  assert.deepEqual([...versions], [2, 3, 4]);
});

test('accepted DTO rejects wrong schema/source/shape/ranges even when deterministic', () => {
  for (const entry of acceptedFixtures()) for (const mutate of [
    raw => { raw.files = [null]; },
    raw => { raw.schema_version = 99; },
    raw => { raw.files[0].id = 1; },
    raw => { raw.files[0].path = 'other.zry'; },
    raw => { raw.files[0].functions = null; },
    raw => { raw.files[0].functions[0].parameters = [null]; },
    raw => { raw.files[0].functions[0].body.expressions = [null]; },
    raw => { raw.files[0].functions[0].name.span.file = 1; },
    raw => { raw.files[0].functions[0].name.span.end = 0xffffffff; },
    raw => { raw.files[0].functions[0].name.span.start = -1; },
    raw => { delete raw.files[0].functions[0].body.span; },
    raw => { raw.diagnostics = [null]; },
    raw => { raw.diagnostics = [{ code: 'ZRYNA-F2002', severity: 'error', message: 'bad',
      guidance: 'bad', location: { kind: 'source', span: { file: 0, start: 0, end: 0xffffffff } } }]; },
  ]) {
    const raw = structuredClone(entry.raw);
    mutate(raw);
    assert.throws(() => acceptedDTO(raw, entry.source, entry.version));
  }
});

test('accepted diagnostics are closed and bounded and nested spans cannot split UTF-8', () => {
  const entry = acceptedFixtures().find(value => value.source.text.includes('π'));
  assert(entry);
  const bytes = Buffer.from(entry.source.text);
  const middle = [...bytes].findIndex(byte => (byte & 0xc0) === 0x80);
  for (const mutate of [
    raw => { raw.files[0].functions[0].name.span.start = middle; },
    raw => { raw.diagnostics[0].location.span.start = middle; },
    raw => { raw.diagnostics[0].severity = 'fatal'; },
    raw => { raw.diagnostics[0].guidance = 9; },
    raw => { raw.diagnostics[0].message = 'x'.repeat(4097); },
    raw => { raw.diagnostics[0].location = { kind: 'global', span: null }; },
    raw => { raw.diagnostics = Array.from({ length: 257 }, () => structuredClone(raw.diagnostics[0])); },
  ]) {
    const raw = structuredClone(entry.raw);
    mutate(raw);
    assert.throws(() => acceptedDTO(raw, entry.source, entry.version));
  }
});

test('self-consistent artifact hashes and totals cannot admit identical malformed accepted outputs', t => {
  const campaign = registry.campaigns[1];
  const directory = mkdtempSync(resolve(tmpdir(), 'zryna-fuzz-dto-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const baseline = acceptedWitness(campaign);
  const totals = replayWitness(encode(baseline.records), campaign);
  const stdout = rustLog(campaign);
  const stderr = Buffer.alloc(0);
  writeFileSync(resolve(directory, `${campaign.id}.stdout`), stdout);
  writeFileSync(resolve(directory, `${campaign.id}.stderr`), stderr);
  for (const mutate of [
    raw => { raw.files = [null]; },
    raw => { raw.schema_version = 3; },
    raw => { raw.files[0].path = 'replacement.zry'; },
    raw => { raw.files[0].functions[0].body.expressions[0].span.end = 0xffffffff; },
  ]) {
    const records = structuredClone(baseline.records);
    for (const index of [baseline.first, baseline.first + 1]) mutate(records[index].outcome.result);
    const witness = encode(records);
    writeFileSync(resolve(directory, `${campaign.id}.jsonl`), witness);
    const record = { id: campaign.id, executable: campaign.executable, args: campaign.args,
      attempt: { exitCode: 0, signal: null, error: null, elapsedMs: 1 }, totals,
      stdout: descriptor(`${campaign.id}.stdout`, stdout), stderr: descriptor(`${campaign.id}.stderr`, stderr),
      witness: descriptor(`${campaign.id}.jsonl`, witness) };
    assert.throws(() => checkObservation(directory, record, campaign));
  }
});
