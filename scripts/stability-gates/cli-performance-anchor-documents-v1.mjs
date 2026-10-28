// Pure exact derivation. These four documents alone carry no table or performance authority.
import assert from 'node:assert/strict';
import { canonical, exact, parse, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { deriveProspectiveThresholds } from './cli-performance-threshold-derive-v1.mjs';
import { parseProspectivePolicy } from './cli-performance-prospective-policy-v1.mjs';
import { HOSTS, ROLES, time } from './cli-performance-threshold-profile-v1.mjs';
export const NUMERIC_ANCHOR_FILES = Object.freeze([
  'anchor-input.json', 'derivation.json', 'numeric-profile.json', 'rationale.json',
]);
export function deriveNumericAnchorDocuments(normalized, baseline, anchor, policyBytes) {
  assert.equal(arguments.length, 4);
  const policy = parseProspectivePolicy(policyBytes);
  assert.equal(normalized.format, 'zryna.cli-performance-normalized-baseline-declarations.v1');
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'baselineAdmitted'])
    assert.equal(normalized[key], false);
  assert.equal(normalized.thresholdAuthority, null); assert.equal(normalized.closureStatus, 'blocked');
  assert.equal(baseline.plan.phase, 'baselines');
  assert.equal(baseline.plan.policySha256, policy.policySha256);
  exact(anchor, ['runId', 'jobId', 'startedAt'], 'actual anchor job identity');
  for (const key of ['runId', 'jobId']) assert(Number.isSafeInteger(anchor[key]) && anchor[key] > 0);
  const recordedAt = new Date(anchor.startedAt).toISOString(); time(recordedAt);
  assert.deepEqual(normalized.source, { commit: baseline.plan.source.commit, tree: baseline.plan.source.tree });
  assert.equal(normalized.catalogueSha256, policy.methodologySha256);
  assert.equal(normalized.rounds.length, 4); assert.equal(normalized.hosts.length, 2);
  const input = { format: 'zryna.cli-performance-numeric-anchor-input.v1',
    source: baseline.plan.source, policySha256: policy.policySha256,
    baselineRunId: baseline.plan.runId, baselinePlanSha256: baseline.reviewedSha256,
    anchor: structuredClone(anchor), collector: normalized.collector,
    toolchainSha256: normalized.toolchainSha256, tools: normalized.tools, hosts: normalized.hosts,
    baselines: normalized.rounds.map((round, index) => {
      const host = HOSTS[Math.floor(index / 2)], role = ROLES[index % 2];
      assert.equal(round.host, host); assert.equal(round.role, role);
      const matches = baseline.jobs.filter(job => job.host === host && job.role === role);
      assert.equal(matches.length, 1); const job = matches[0];
      assert.equal(round.identity.jobId, String(job.jobId));
      assert.equal(round.identity.artifactId, String(job.artifacts[0].id));
      assert.equal(round.identity.rawSha256, job.artifacts[0].sha256);
      assert(time(round.endedAt) < time(recordedAt), 'all baselines precede observed anchor start');
      return { host, role, jobId: job.jobId, checkRunId: job.checkRunId,
        startedAt: job.startedAt, completedAt: job.completedAt, artifacts: job.artifacts,
        normalizedSha256: sha256(canonical(round)) };
    }), sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked' };
  const inputBytes = Buffer.from(canonical(input));
  const profile = { format: 'zryna.cli-performance-threshold-parameters.v1',
    scope: 'shared-hosted-vm-regression-only', catalogueSha256: normalized.catalogueSha256,
    sources: { reference: normalized.source, candidate: normalized.source },
    toolchainSha256: normalized.toolchainSha256, collectorSha256: normalized.collector.collectorSha256,
    hosts: normalized.hosts,
    baselinePins: normalized.rounds.map(round => ({ host: round.host, role: round.role, identity: round.identity })),
    anchor: { kind: 'declared-record-not-authentication', recordSha256: sha256(inputBytes),
      rationaleSha256: sha256(policyBytes), recordedAt },
    cells: HOSTS.flatMap(host => matrixFor(normalized.catalogue, host).map(row => ({ host, selector: row.id,
      maximumWallNs: '1', maximumMemoryNativeUnits: '1',
      wallSpread: policy.ratios.withinRoundSpread.wall, memorySpread: policy.ratios.withinRoundSpread.memory,
      wallCrossRoundRatio: policy.ratios.crossBaselineMedian.wall,
      memoryCrossRoundRatio: policy.ratios.crossBaselineMedian.memory,
      wallRegressionRatio: policy.ratios.heldoutOverWorseBaselineMedian.wall,
      memoryRegressionRatio: policy.ratios.heldoutOverWorseBaselineMedian.memory }))),
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    thresholdAuthority: null, closureStatus: 'blocked' };
  // Snapshot shared values before the bounded canonical traversal, which rejects object aliasing.
  const seed = JSON.parse(JSON.stringify(profile));
  const derivation = deriveProspectiveThresholds(seed, normalized.rounds, normalized.catalogue, policyBytes);
  assert.equal(derivation.status, 'derived-from-baseline-declarations', 'ineligible baselines cannot create a table');
  assert.deepEqual(derivation.failures, []);
  return new Map([['anchor-input.json', inputBytes], ['rationale.json', Buffer.from(policyBytes)],
    ['numeric-profile.json', Buffer.from(canonical(derivation.profile))],
    ['derivation.json', Buffer.from(canonical(derivation))]]);
}
export function decodeNumericAnchorInput(bytes) {
  const input = parse(bytes);
  exact(input, ['format', 'source', 'policySha256', 'baselineRunId', 'baselinePlanSha256', 'anchor', 'collector',
    'toolchainSha256', 'tools', 'hosts', 'baselines', 'sourceAuthenticated', 'proofQualified',
    'performanceQualified', 'baselineAdmitted', 'thresholdAuthority', 'closureStatus'], 'original anchor input');
  assert.equal(input.format, 'zryna.cli-performance-numeric-anchor-input.v1');
  assert(Number.isSafeInteger(input.baselineRunId) && input.baselineRunId > 0);
  return input; // Full semantic authority comes only from complete byte-for-byte rederivation.
}
