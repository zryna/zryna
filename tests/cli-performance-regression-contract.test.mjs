// Input identity, host, raw-unit and authority controls; no benchmark execution.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { compareProspective } from '../scripts/stability-gates/cli-performance-regression-v1.mjs';
import { parseProfile } from '../scripts/stability-gates/cli-performance-threshold-profile-v1.mjs';
import { catalogue, clone, fixture } from './cli-performance-numeric-fixture.mjs';
const run = f => compareProspective(f.profile, f.rounds, catalogue);
const fail = mutate => {
  const pristine = fixture(), f = clone(pristine); mutate(f); assert.throws(() => run(f));
  assert.equal(run(pristine).status, 'within-declared-limits', 'pristine independent retry');
};
test('closed profile rejects missing cells, unsupported hosts and malformed or unauthorized parameters', () => {
  for (const mutate of [f => { f.profile.cells.pop(); }, f => { f.profile.cells[0].selector = 'arbitrary/run/native'; },
    f => { f.profile.hosts[1].host = 'macos-aarch64'; }, f => { f.profile.cells[0].maximumWallNs = '15000000001'; },
    f => { f.profile.cells[0].maximumMemoryNativeUnits = 1000; }, f => { f.profile.cells[0].wallSpread.denominator = '0'; },
    f => { f.profile.cells[0].wallSpread.numerator = '01'; }, f => { f.profile.cells[0].wallSpread.numerator = '1e3'; },
    f => { f.profile.cells[0].wallRegressionRatio = { numerator: '1', denominator: '2' }; },
    f => { f.profile.cells[0].maximumMemoryNativeUnits = '18446744073709551616'; },
    f => { f.profile.reviewApproved = true; }, f => { f.profile.thresholdAuthority = { copied: true }; },
    f => { f.profile.sourceAuthenticated = true; }, f => { f.profile.sources.reference.commit = 'not-a-hash'; }]) fail(mutate);
});
test('source, release bytes, tools, collector and frozen workload identity cannot drift', () => {
  for (const mutate of [f => { f.rounds[2].source.tree = 'f'.repeat(40); },
    f => { f.rounds[2].build.cliSha256 = 'f'.repeat(64); }, f => { f.rounds[2].build.class = 'debug'; },
    f => { f.rounds[2].toolchainSha256 = 'f'.repeat(64); }, f => { f.rounds[2].collectorSha256 = 'f'.repeat(64); },
    f => { f.rounds[2].catalogueSha256 = 'f'.repeat(64); }, f => { f.profile.catalogueSha256 = 'f'.repeat(64); }]) fail(mutate);
});
test('independent original build records remain per round while common release image stays fixed', () => {
  const f = fixture(), result = run(f);
  for (const offset of [0, 3]) {
    assert.equal(f.rounds[offset].build.cliSha256, f.rounds[offset + 1].build.cliSha256);
    assert.notEqual(f.rounds[offset].build.recordSha256, f.rounds[offset + 1].build.recordSha256);
    assert.equal(Object.hasOwn(f.profile.hosts[offset / 3].referenceBuild, 'recordSha256'), false);
  }
  assert.equal(result.status, 'within-declared-limits');
  const changed = clone(f); changed.rounds[1].build.recordSha256 = 'f'.repeat(64);
  const declared = run(changed);
  assert.notEqual(declared.inputs[1].normalizedSha256, result.inputs[1].normalizedSha256);
  assert.equal(declared.sourceAuthenticated, false, 'original-byte linkage still belongs to raw replay owner');
  fail(g => { delete g.rounds[0].build.recordSha256; });
  fail(g => { g.rounds[0].build.recordSha256 = 'not-a-hash'; });
  fail(g => { g.rounds[0].build.extra = true; });
  fail(g => { g.profile.hosts[0].referenceBuild.recordSha256 = 'f'.repeat(64); });
});
test('missing, pooled and changing host facts reject; named unknowns are never unlimited observations', () => {
  for (const mutate of [f => { f.rounds[2].classBefore.cpuModel.value = 'different CPU'; },
    f => { f.rounds[2].classAfter.osImage.value = 'different image'; }, f => { delete f.profile.hosts[0].facts.affinity; },
    f => { f.profile.hosts[0].facts.cpuModel = { status: 'unavailable', reason: 'unobserved' }; },
    f => { f.profile.hosts[0].acceptedUnknownFacts.pop(); },
    f => { f.rounds[2].classAfter.outerCpuQuota = { status: 'observed', value: 'unlimited' }; }]) fail(mutate);
});
test('required observed kernel strings bind both hosts and every baseline/heldout before/after class', () => {
  const pristine = fixture();
  assert.equal(pristine.profile.hosts[0].facts.kernel.value, 'synthetic-linux-kernel-v1');
  assert.equal(pristine.profile.hosts[1].facts.kernel.value, 'synthetic-windows-kernel-v1');
  assert.equal(run(pristine).status, 'within-declared-limits');
  for (const host of [0, 1]) {
    fail(f => { delete f.profile.hosts[host].facts.kernel; });
    fail(f => { f.profile.hosts[host].facts.kernel = { status: 'unavailable', reason: 'not observed' }; });
    fail(f => { f.profile.hosts[host].facts.kernel.value = ''; });
    fail(f => { f.profile.hosts[host].facts.kernel.value = 'different-kernel'; });
  }
  for (let round = 0; round < 6; round += 1) for (const phase of ['classBefore', 'classAfter']) {
    fail(f => { delete f.rounds[round][phase].kernel; });
    fail(f => { f.rounds[round][phase].kernel.value = 'changed-kernel'; });
  }
});
test('Linux wait4 KiB and Windows Job COMMIT bytes cannot be exchanged or relabeled', () => {
  fail(f => { f.rounds[5].memoryMethod = f.rounds[0].memoryMethod; });
  fail(f => { f.profile.hosts[1].memoryMethod = f.profile.hosts[0].memoryMethod; });
  fail(f => { const a = f.rounds[2].cells[0].attempts[1]; a.peakJobMemoryUsedBytes = a.peakRssKiB; delete a.peakRssKiB; });
});
test('Linux observed KiB accepts exact raw GNU 15-digit maximum but rejects its first extra', () => {
  const f = fixture(), maximum = '999999999999999';
  for (const limit of f.profile.cells.filter(row => row.host === 'linux-x86_64'))
    limit.maximumMemoryNativeUnits = maximum;
  for (const round of f.rounds.slice(0, 3)) for (const cell of round.cells) for (const attempt of cell.attempts)
    attempt.peakRssKiB = maximum;
  const result = run(f);
  assert.equal(result.status, 'within-declared-limits');
  assert.deepEqual(result.cells[0].observed.find(row => row.metric === 'memory').summaries[2].maximum,
    { numerator: maximum, denominator: '1' });
  const extra = clone(f);
  extra.profile.cells[0].maximumMemoryNativeUnits = '1000000000000000';
  extra.rounds[2].cells[0].attempts[5].peakRssKiB = '1000000000000000';
  assert.throws(() => run(extra), /Linux GNU marker permits at most 15 decimal digits/u);
  assert.equal(run(f).status, 'within-declared-limits', 'exact-bound pristine retry');
});
test('duplicate or omitted round roles and reused jobs, artifacts or raw identities reject', () => {
  fail(f => { f.rounds.pop(); }); fail(f => { f.rounds[1] = clone(f.rounds[0]); });
  fail(f => { f.profile.baselinePins[1].role = 'baseline-round-1'; });
  for (const key of ['jobId', 'artifactId', 'rawSha256']) fail(f => { f.rounds[2].identity[key] = f.rounds[0].identity[key]; });
  fail(f => { f.rounds[0].identity.rawSha256 = 'f'.repeat(64); });
});
test('late/equal declared anchor, overlapping baselines and malformed timestamps reject', () => {
  for (const mutate of [f => { f.profile.anchor.recordedAt = f.rounds[2].startedAt; },
    f => { f.profile.anchor.recordedAt = f.rounds[1].endedAt; },
    f => { f.rounds[1].startedAt = f.rounds[0].endedAt; },
    f => { f.rounds[2].endedAt = f.rounds[2].startedAt; }, f => { f.profile.anchor.recordedAt = '2030-02-30T00:00:00.000Z'; }]) fail(mutate);
});
test('invalid correctness/cleanup, class-free copied reports and authority promotion fail structurally', () => {
  fail(f => { f.rounds[2].cells[0].attempts[5].outcome = 'cleanup-unknown'; });
  fail(f => { delete f.rounds[2].classBefore; });
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified']) fail(f => { f.rounds[2][key] = true; });
  fail(f => { f.rounds[2].hostAuthenticated = true; });
  const f = fixture(), result = run(f);
  assert.throws(() => compareProspective(f.profile, result, catalogue));
  assert.equal(run({ profile: clone(f.profile), rounds: clone(f.rounds) }).sourceAuthenticated, false);
});
test('all 704 simultaneous numeric failures stay bounded without dropping cells or failure records', () => {
  const f = fixture();
  for (const limit of f.profile.cells) {
    limit.maximumWallNs = '1'; limit.maximumMemoryNativeUnits = '1';
    limit.wallSpread = { numerator: '1', denominator: '18446744073709551615' };
    limit.memorySpread = { numerator: '1', denominator: '18446744073709551615' };
  }
  f.rounds.forEach((round, roundIndex) => {
    const middle = [100, 200, 400][roundIndex % 3], values = [middle / 2, middle, middle, middle, middle + 1];
    for (const cell of round.cells) cell.attempts.slice(1).forEach((attempt, index) => {
      if (round.host === 'linux-x86_64') {
        attempt.wallNs = String(values[index]); attempt.peakRssKiB = String(values[index]);
      } else {
        attempt.wallTicks = String(values[index]); attempt.peakJobMemoryUsedBytes = String(values[index]);
      }
    });
    if (round.host === 'windows-x86_64') for (const cell of round.cells) for (const attempt of cell.attempts)
      attempt.qpcFrequency = '1000000000';
  });
  const result = run(f);
  assert.equal(result.cells.length, 44); assert.equal(result.failures.length, 704);
  assert.doesNotThrow(() => canonical(result), 'existing JSON budgets unchanged, no aliases or duplicate failure copies');
});
test('canonical closed reader rejects duplicate keys and formatted alternate encodings without issuing authority', () => {
  const { profile } = fixture();
  assert.deepEqual(parseProfile(Buffer.from(canonical(profile)), catalogue), profile);
  assert.throws(() => parseProfile(Buffer.from(JSON.stringify(profile, null, 2)), catalogue));
  assert.throws(() => parseProfile(Buffer.from('{"format":1,"format":2}\n'), catalogue));
});
test('frozen legacy v1/v4 and legacy comparator remain byte-identical, not threshold sources', () => {
  for (const [path, expected] of [
    ['./stability-gates-v1.json', 'f339c9edba6773d63a252908dbfa094875a5ecb7c66843dd8017142f547e787b'],
    ['./cli-performance-methodology-v4.json', 'b73dee72db94b65fb9e0786c43e807e416e8791bf650da2b5babd4c658294b7b'],
    ['../scripts/stability-gates/performance.mjs', 'd93b7924273f42ffe877dd781d36c3ccace7648cf5c42dfa13de628ce7ffd4b9'],
  ]) assert.equal(sha256(readFileSync(new URL(path, import.meta.url))), expected);
});
