import './cli-performance-regression-contract.test.mjs';
// Exact numeric boundary controls; no collector, process, guest or benchmark calls.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { compareProspective } from '../scripts/stability-gates/cli-performance-regression-v1.mjs';
import { catalogue, clone, fixture } from './cli-performance-numeric-fixture.mjs';
const run = f => compareProspective(f.profile, f.rounds, catalogue);
const fail = mutate => {
  const pristine = fixture(), f = clone(pristine); mutate(f); assert.throws(() => run(f));
  assert.equal(run(pristine).status, 'within-declared-limits', 'pristine independent retry');
};
const rules = result => result.failures.map(row => `${row.metric}/${row.rule}`);
const setMeasured = (round, metric, value) => {
  for (const cell of round.cells) for (const attempt of cell.attempts.slice(1)) attempt[metric] = value;
};
test('synthetic equality boundary keeps all 44 cells, 792 attempts and every unqualified flag', () => {
  const f = fixture(), before = clone(f), result = run(f);
  assert.equal(result.status, 'within-declared-limits'); assert.equal(result.cells.length, 44);
  assert.equal(result.inputs.reduce((n, row) => n + row.warmups + row.measuredAttempts, 0), 792);
  assert.equal(result.inputs.length, 6);
  result.inputs.forEach((row, index) => assert.equal(row.normalizedSha256, sha256(canonical(f.rounds[index]))));
  assert.deepEqual(f, before); assert.doesNotThrow(() => canonical(result)); assert.equal(result.baselineAdmitted, false);
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified']) assert.equal(result[key], false);
  assert.equal(result.thresholdAuthority, null); assert.equal(result.closureStatus, 'blocked');
  assert.equal(result.declaredTemporalOrderChecked, true); assert.equal(result.pending.length, 3);
});
test('first-extra absolute wall and memory at EACH of five positions fails without dropping a value', () => {
  for (let index = 1; index <= 5; index += 1) for (const [key, value, expected] of
    [['wallNs', '1000000001', 'wall/absolute-all-five'], ['peakRssKiB', '1001', 'memory/absolute-all-five']]) {
    const f = fixture(); f.rounds[2].cells[0].attempts[index][key] = value;
    const result = run(f); assert.equal(result.status, 'outside-declared-limits');
    assert(rules(result).includes(expected));
    assert.equal(result.inputs[2].normalizedSha256, sha256(canonical(f.rounds[2])));
  }
});
test('warmup remains retained and validated but cannot be substituted into the five measurements', () => {
  const f = fixture(); f.rounds[2].cells[0].attempts[0].wallNs = '2000000000';
  assert.equal(run(f).status, 'within-declared-limits');
  fail(g => { g.rounds[2].cells[0].attempts[1].role = 'warmup'; });
  fail(g => { g.rounds[2].cells[0].attempts.pop(); });
  fail(g => { g.rounds[2].cells[0].attempts[5] = clone(g.rounds[2].cells[0].attempts[4]); });
});
test('independent wall/memory spread checks include slow and unusually fast samples in every round', () => {
  for (const roundIndex of [0, 1, 2]) for (const [metric, key, value] of
    [['wall', 'wallNs', '800000000'], ['memory', 'peakRssKiB', '800']]) {
    const f = fixture(); f.profile.cells[0][`${metric}Spread`] = { numerator: '1', denominator: '10' };
    f.rounds[roundIndex].cells[0].attempts[1][key] = value;
    assert(rules(run(f)).includes(`${metric}/within-round-spread`));
  }
});
test('cross-round consistency rejects two individually noiseless but incompatible baseline medians', () => {
  const f = fixture();
  for (const cell of f.profile.cells.filter(row => row.host === 'linux-x86_64')) {
    cell.maximumWallNs = '2000000000'; cell.maximumMemoryNativeUnits = '2000';
  }
  setMeasured(f.rounds[1], 'wallNs', '1200000000'); setMeasured(f.rounds[1], 'peakRssKiB', '1200');
  const result = run(f);
  assert(rules(result).includes('wall/cross-round-median-ratio'));
  assert(rules(result).includes('memory/cross-round-median-ratio'));
});
test('relative medians use WORSE of BOTH baselines, never the best baseline', () => {
  const f = fixture();
  for (const cell of f.profile.cells.filter(row => row.host === 'linux-x86_64')) {
    cell.maximumWallNs = '2000000000'; cell.maximumMemoryNativeUnits = '2000';
    cell.wallCrossRoundRatio = { numerator: '2', denominator: '1' };
    cell.memoryCrossRoundRatio = { numerator: '2', denominator: '1' };
  }
  for (const index of [1, 2]) {
    setMeasured(f.rounds[index], 'wallNs', '2000000000'); setMeasured(f.rounds[index], 'peakRssKiB', '2000');
  }
  assert.equal(run(f).status, 'within-declared-limits');
});
test('relative regression checks are separate from a permissive absolute ceiling', () => {
  const f = fixture();
  for (const cell of f.profile.cells.filter(row => row.host === 'linux-x86_64')) {
    cell.maximumWallNs = '2000000000'; cell.maximumMemoryNativeUnits = '2000';
    cell.wallRegressionRatio = { numerator: '11', denominator: '10' };
    cell.memoryRegressionRatio = { numerator: '11', denominator: '10' };
  }
  setMeasured(f.rounds[2], 'wallNs', '1200000000'); setMeasured(f.rounds[2], 'peakRssKiB', '1200');
  const result = run(f);
  assert(rules(result).includes('wall/relative-to-worse-baseline-median'));
  assert(rules(result).includes('memory/relative-to-worse-baseline-median'));
});
test('QPC first-extra smaller than one ns beyond Number precision cannot round down to a pass', () => {
  const f = fixture(), frequency = '9007199254740992';
  for (const round of f.rounds.slice(3)) for (const cell of round.cells) for (const attempt of cell.attempts) {
    attempt.qpcFrequency = frequency; attempt.wallTicks = frequency;
  }
  assert.equal(run(f).status, 'within-declared-limits');
  f.rounds[5].cells[0].attempts[5].wallTicks = '9007199254740993';
  assert(rules(run(f)).includes('wall/absolute-all-five'));
});
test('exact QPC relative ratio and native COMMIT first-extra above 2^53 remain distinguishable', () => {
  const f = fixture();
  for (const cell of f.profile.cells.filter(row => row.host === 'windows-x86_64')) {
    cell.maximumWallNs = '2000000000'; cell.maximumMemoryNativeUnits = '9007199254740992';
    cell.wallRegressionRatio = { numerator: '18446744073709551615', denominator: '18446744073709551614' };
  }
  for (const round of f.rounds.slice(3)) for (const cell of round.cells) for (const attempt of cell.attempts) {
    attempt.qpcFrequency = '9007199254740992'; attempt.wallTicks = '9007199254740992';
    attempt.peakJobMemoryUsedBytes = '9007199254740992';
  }
  for (const attempt of f.rounds[5].cells[0].attempts.slice(1)) attempt.wallTicks = '9007199254740993';
  f.rounds[5].cells[0].attempts[5].peakJobMemoryUsedBytes = '9007199254740993';
  const result = run(f);
  assert(rules(result).includes('wall/relative-to-worse-baseline-median'));
  assert(rules(result).includes('memory/absolute-all-five'));
});
test('QPC frequency is retained across rounds and must be positive and uniform within each cell', () => {
  const f = fixture();
  for (const cell of f.rounds[5].cells) for (const attempt of cell.attempts) {
    attempt.qpcFrequency = '18446744073709551615'; attempt.wallTicks = '18446744073709551615';
  }
  assert.equal(run(f).status, 'within-declared-limits');
  fail(g => { g.rounds[5].cells[0].attempts[3].qpcFrequency = '20000000'; });
  fail(g => { g.rounds[5].cells[0].attempts[0].qpcFrequency = '0'; });
});
