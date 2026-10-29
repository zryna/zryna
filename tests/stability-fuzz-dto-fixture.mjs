// Original independent frozen DTO/source bytes; no candidate or expected-output generation.
import assert from 'node:assert/strict';
import { readSafe, sha256 } from '../scripts/stability-gates/input.mjs';
import { ROOT } from '../scripts/stability-gates/fuzz-registry.mjs';
import { witnessRecords } from './stability-fuzz-fixture.mjs';

export function acceptedFixtures() {
  const descriptors = JSON.parse(readSafe(ROOT, 'tests/stability-fuzz-accepted-fixtures.json'));
  const read = pin => {
    const bytes = readSafe(ROOT, pin.path, 2 * 1024 * 1024);
    assert.equal(bytes.length, pin.bytes);
    assert.equal(sha256(bytes), pin.sha256);
    return bytes;
  };
  return descriptors.map(entry => ({ version: entry.version,
    source: { path: entry.sourcePath, text: read(entry.source).toString('utf8') },
    raw: JSON.parse(read(entry.dto)) }));
}

export function acceptedWitness(campaign, version = 4) {
  const original = acceptedFixtures().find(entry => entry.version === version);
  const records = witnessRecords(campaign);
  // Synthetic transplant for ledger controls only; not authentic Rust verification.
  const raw = structuredClone(original.raw);
  raw.files[0].path = 'src/main.zry';
  const index = records.findIndex(record => record.event === 'input' &&
    Buffer.byteLength(record.text) >= Buffer.byteLength(original.source.text));
  assert(index > 0);
  const first = index + 1 + (version - 2) * 2;
  for (const offset of [first, first + 1]) records[offset].outcome = { result: structuredClone(raw) };
  return { records, first };
}
