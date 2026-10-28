// Pure comparison of declarations only. Copied reports can NEVER acquire authentication here.
import assert from 'node:assert/strict';
import { canonical, sha256 } from './input.mjs';
import { compare, divide, fraction, integer, maximum } from './cli-performance-numeric-math-v1.mjs';
import { checkedRound } from './cli-performance-numeric-round-v1.mjs';
import { HOSTS, parseProfile, ROLES, time } from './cli-performance-threshold-profile-v1.mjs';
function unique(records) {
  for (const key of ['jobId', 'artifactId', 'rawSha256']) {
    const identities = records.map(({ round }) => round.identity[key]);
    assert.equal(new Set(identities).size, identities.length, 'no reused baseline/held-out job, artifact or raw identity');
  }
}
function comparison(host, limit, records) {
  const failures = [], observed = [];
  const fail = (role, metric, rule) => failures.push({ host, selector: limit.selector, role, metric, rule });
  for (const metric of ['wall', 'memory']) {
    const summaries = records.map(record => record.summaries.find(row => row.selector === limit.selector)[metric]);
    const absolute = fraction(integer(metric === 'wall' ? limit.maximumWallNs : limit.maximumMemoryNativeUnits));
    const spread = limit[`${metric}Spread`];
    summaries.forEach((summary, index) => {
      // Maximum <= absolute is exactly equivalent to all five <= absolute; retain all five inputs below.
      if (compare(summary.maximum, absolute) > 0) fail(ROLES[index], metric, 'absolute-all-five');
      if (compare(summary.spread, spread) > 0) fail(ROLES[index], metric, 'within-round-spread');
    });
    const upper = maximum(summaries[0].median, summaries[1].median);
    const lower = compare(summaries[0].median, summaries[1].median) <= 0 ? summaries[0].median : summaries[1].median;
    const crossRound = divide(upper, lower), regression = divide(summaries[2].median, upper);
    if (compare(crossRound, limit[`${metric}CrossRoundRatio`]) > 0) fail('both-baselines', metric, 'cross-round-median-ratio');
    if (compare(regression, limit[`${metric}RegressionRatio`]) > 0) fail('held-out-candidate', metric, 'relative-to-worse-baseline-median');
    observed.push({ metric, summaries, crossRound, regression });
  }
  return { host, selector: limit.selector, failures, observed };
}
export function compareProspective(profileValue, roundValues, catalogue) {
  const profile = parseProfile(Buffer.from(canonical(profileValue)), catalogue);
  assert(Array.isArray(roundValues) && roundValues.length === 6, 'exactly two baselines and one held-out round for each host');
  const checked = HOSTS.flatMap((host, hostIndex) => ROLES.map((role, roleIndex) =>
    checkedRound(roundValues[hostIndex * 3 + roleIndex], profile, catalogue, profile.hosts[hostIndex], role)));
  unique(checked);
  const anchor = time(profile.anchor.recordedAt);
  for (let hostIndex = 0; hostIndex < 2; hostIndex += 1) {
    const records = checked.slice(hostIndex * 3, hostIndex * 3 + 3);
    records.slice(0, 2).forEach(({ round }, index) => {
      const pin = profile.baselinePins[hostIndex * 2 + index];
      assert.deepEqual(round.identity, pin.identity, 'both exact baseline raw declarations retained');
      assert(time(round.endedAt) < anchor, 'baseline must precede declared parameter anchor');
    });
    assert(time(records[0].round.endedAt) < time(records[1].round.startedAt), 'distinct ordered baseline round intervals');
    assert(anchor < time(records[2].round.startedAt), 'declared threshold anchor must strictly precede held-out start');
  }
  const cells = profile.cells.map(limit => comparison(limit.host, limit,
    checked.slice(HOSTS.indexOf(limit.host) * 3, HOSTS.indexOf(limit.host) * 3 + 3)));
  const failures = cells.flatMap(cell => cell.failures);
  return { format: 'zryna.cli-performance-numeric-comparison.v1',
    status: failures.length ? 'outside-declared-limits' : 'within-declared-limits',
    profileSha256: sha256(canonical(profile)), declaredTemporalOrderChecked: true,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    closureStatus: 'blocked', thresholdAuthority: null, baselineAdmitted: false,
    pending: ['independent policy and numeric rationale review', 'opaque source/platform/raw replay linkage',
      'independent host/fact admission and authentic pre-held-out threshold anchor'],
    cells: cells.map(({ host, selector, observed }) => ({ host, selector, observed })), failures,
    inputs: checked.map(({ round }) => ({ host: round.host, role: round.role, identity: round.identity,
      normalizedSha256: sha256(canonical(round)), cells: round.cells.length,
      warmups: round.cells.length, measuredAttempts: round.cells.length * 5 })) };
}
