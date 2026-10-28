// Synthetic controls only; no collector, workload or platform calls.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { deriveProspectiveThresholds } from '../scripts/stability-gates/cli-performance-threshold-derive-v1.mjs';
import { catalogue, clone, fixture } from './cli-performance-numeric-fixture.mjs';
const policy = readFileSync(new URL('./cli-performance-prospective-policy-v1.json', import.meta.url));
const baseline = f => [f.rounds[0], f.rounds[1], f.rounds[3], f.rounds[4]];
const run = f => deriveProspectiveThresholds(f.profile, baseline(f), catalogue, policy);
test('derives all 44 cells from four pinned baselines with frozen ratios and no authority', () => {
  const f = fixture(), before = clone(f), result = run(f);
  assert.equal(result.status, 'derived-from-baseline-declarations');
  assert.equal(result.profile.cells.length, 44); assert.equal(result.inputs.length, 4);
  for (const cell of result.profile.cells) {
    assert.equal(cell.maximumWallNs, '1100000000'); assert.equal(cell.maximumMemoryNativeUnits, '1050');
    assert.deepEqual(cell.wallSpread, { numerator: '1', denominator: '5' });
    assert.deepEqual(cell.memoryRegressionRatio, { numerator: '21', denominator: '20' });
  }
  assert.equal(result.profileSha256, sha256(canonical(result.profile)));
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'baselineAdmitted'])
    assert.equal(result[key], false);
  assert.equal(result.thresholdAuthority, null); assert.equal(result.closureStatus, 'blocked');
  assert.deepEqual(f, before);
});
test('takes all ten measured baseline values while excluding the retained warmups', () => {
  for (const roundIndex of [0, 1]) for (let index = 1; index <= 5; index += 1) {
    const f = fixture();
    f.rounds[0].cells[0].attempts[0].wallNs = '14000000000';
    f.rounds[roundIndex].cells[0].attempts[index].wallNs = '1100000000';
    const result = run(f); assert.equal(result.profile.cells[0].maximumWallNs, '1210000000');
  }
});
test('QPC remains rational through max and multiplication and is ceiled exactly once', () => {
  const f = fixture();
  for (const round of baseline(f).slice(2)) for (const cell of round.cells) for (const attempt of cell.attempts) {
    attempt.wallTicks = '1'; attempt.qpcFrequency = '3';
  }
  const result = run(f);
  assert.equal(result.profile.cells.find(cell => cell.host === 'windows-x86_64').maximumWallNs, '366666667');
});
test('native memory beyond 2^53 keeps exact margin and rejects overflow', () => {
  const f = fixture();
  for (const round of baseline(f).slice(2)) for (const cell of round.cells) for (const attempt of cell.attempts)
    attempt.peakJobMemoryUsedBytes = '9007199254740993';
  assert.equal(run(f).profile.cells.find(cell => cell.host === 'windows-x86_64').maximumMemoryNativeUnits, '9457559217478043');
  for (const round of baseline(f).slice(2)) for (const attempt of round.cells[0].attempts)
    attempt.peakJobMemoryUsedBytes = '18446744073709551615';
  assert.throws(() => run(f), /native u64 range/u);
});
test('unstable spread and incompatible baseline medians remain nonpassing', () => {
  const f = fixture(); f.rounds[0].cells[0].attempts[1].wallNs = '700000000';
  for (const attempt of f.rounds[1].cells[1].attempts.slice(1)) attempt.peakRssKiB = '1100';
  const result = run(f); assert.equal(result.status, 'ineligible-baseline-declarations');
  assert(result.failures.some(row => row.rule === 'within-round-spread'));
  assert(result.failures.some(row => row.rule === 'cross-round-median-ratio'));
  assert.equal(result.baselineAdmitted, false);
});
test('refuses heldout contamination, changed policy, pins, class drift and nonserial chronology', () => {
  const f = fixture();
  assert.throws(() => deriveProspectiveThresholds(f.profile, [...baseline(f), f.rounds[2]], catalogue, policy));
  assert.throws(() => deriveProspectiveThresholds(f.profile, baseline(f), catalogue,
    Buffer.from(canonical({ ...JSON.parse(policy), scope: 'override' }))));
  for (const mutate of [
    g => { g.rounds[1].identity.artifactId = '999'; },
    g => { g.rounds[1].classAfter.kernel.value = 'changed'; },
    g => { g.rounds[1].startedAt = g.rounds[0].endedAt; },
    g => { g.profile.anchor.recordedAt = g.rounds[1].endedAt; },
    g => { g.rounds[1].cells[0].attempts.pop(); },
  ]) { const g = clone(f); mutate(g); assert.throws(() => run(g)); }
});
test('derived margin cannot expand the existing 15-second containment', () => {
  const f = fixture();
  for (const round of baseline(f).slice(0, 2)) for (const attempt of round.cells[0].attempts)
    attempt.wallNs = '14000000000';
  assert.throws(() => run(f), /methodology insufficient/u);
});
