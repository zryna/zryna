// Pure baseline derivation. Platform, source, host and temporal authentication remain separate.
import assert from 'node:assert/strict';
import { canonical, sha256 } from './input.mjs';
import { compare, divide, integer, maximum, MAX_WALL_NS } from './cli-performance-numeric-math-v1.mjs';
import { checkedRound } from './cli-performance-numeric-round-v1.mjs';
import { parseProspectivePolicy } from './cli-performance-prospective-policy-v1.mjs';
import { HOSTS, parseProfile, ROLES, time } from './cli-performance-threshold-profile-v1.mjs';

function ceiling(value, margin) {
  const numerator = BigInt(value.numerator) * BigInt(margin.numerator);
  const denominator = BigInt(value.denominator) * BigInt(margin.denominator);
  const result = String((numerator + denominator - 1n) / denominator);
  integer(result); // Preserve the existing record range; never widen or saturate.
  return result;
}

export function deriveProspectiveThresholds(profileValue, baselineValues, catalogue, policyBytes) {
  assert.equal(arguments.length, 4, 'fixed declaration inputs; no ratio or rounding overrides');
  const profile = parseProfile(Buffer.from(canonical(profileValue)), catalogue);
  const policy = parseProspectivePolicy(policyBytes);
  assert(Array.isArray(baselineValues) && baselineValues.length === 4, 'exactly four complete baselines, no heldout input');
  const checked = HOSTS.flatMap((host, hostIndex) => ROLES.slice(0, 2).map((role, roleIndex) =>
    checkedRound(baselineValues[hostIndex * 2 + roleIndex], profile, catalogue, profile.hosts[hostIndex], role)));
  for (const key of ['jobId', 'artifactId', 'rawSha256'])
    assert.equal(new Set(checked.map(({ round }) => round.identity[key])).size, 4, 'independent baseline identities');
  checked.forEach(({ round }, index) => {
    assert.deepEqual(round.identity, profile.baselinePins[index].identity, 'exact baseline pin');
    assert(time(round.endedAt) < time(profile.anchor.recordedAt), 'baseline precedes declared anchor');
    if (index % 2 === 1)
      assert(time(checked[index - 1].round.endedAt) < time(round.startedAt), 'serial baselines per host');
  });
  const failures = [];
  const cells = profile.cells.map(({ host, selector }) => {
    const records = checked.slice(HOSTS.indexOf(host) * 2, HOSTS.indexOf(host) * 2 + 2);
    const limits = { host, selector };
    for (const metric of ['wall', 'memory']) {
      const summaries = records.map(record => record.summaries.find(row => row.selector === selector)[metric]);
      const spread = policy.ratios.withinRoundSpread[metric];
      const cross = policy.ratios.crossBaselineMedian[metric];
      summaries.forEach((summary, index) => {
        if (compare(summary.spread, spread) > 0)
          failures.push({ host, selector, role: ROLES[index], metric, rule: 'within-round-spread' });
      });
      const upper = maximum(summaries[0].median, summaries[1].median);
      const lower = compare(summaries[0].median, summaries[1].median) <= 0 ? summaries[0].median : summaries[1].median;
      if (compare(divide(upper, lower), cross) > 0)
        failures.push({ host, selector, role: 'both-baselines', metric, rule: 'cross-round-median-ratio' });
      const limit = ceiling(maximum(summaries[0].maximum, summaries[1].maximum),
        policy.declaration.absoluteRule[`${metric}Ratio`]);
      if (metric === 'wall') assert(BigInt(limit) <= MAX_WALL_NS, 'derived margin exceeds unchanged containment; methodology insufficient');
      limits[metric === 'wall' ? 'maximumWallNs' : 'maximumMemoryNativeUnits'] = limit;
      limits[`${metric}Spread`] = structuredClone(spread);
      limits[`${metric}CrossRoundRatio`] = structuredClone(cross);
      limits[`${metric}RegressionRatio`] = structuredClone(policy.ratios.heldoutOverWorseBaselineMedian[metric]);
    }
    return limits;
  });
  const derived = parseProfile(Buffer.from(canonical({ ...profile, cells })), catalogue);
  return { format: 'zryna.cli-performance-threshold-derivation.v1',
    status: failures.length ? 'ineligible-baseline-declarations' : 'derived-from-baseline-declarations',
    policySha256: policy.policySha256, profile: derived, profileSha256: sha256(canonical(derived)),
    inputs: checked.map(({ round }) => ({ host: round.host, role: round.role,
      identity: round.identity, normalizedSha256: sha256(canonical(round)) })), failures,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked' };
}
