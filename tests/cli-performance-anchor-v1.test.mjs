// Pure derivation/current-metadata controls only; no download, program or measurement runs.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { catalogue, fixture } from './cli-performance-numeric-fixture.mjs';
import { campaignFixture } from './cli-performance-campaign-fixture.mjs';
import { deriveNumericAnchorDocuments, decodeNumericAnchorInput, NUMERIC_ANCHOR_FILES }
  from '../scripts/stability-gates/cli-performance-anchor-documents-v1.mjs';
import { observeCurrentNumericJob, numericRunId, NUMERIC_API_PREFIX }
  from '../scripts/stability-gates/cli-performance-anchor-context-v1.mjs';
import { inspectNumericCampaignAnchor, closeNumericCampaignAnchor }
  from '../scripts/stability-gates/cli-performance-campaign-anchor-v1.mjs';
import { numericAnchorArguments } from '../scripts/stability-gates/run-cli-performance-numeric-anchor-v1.mjs';
const policy = readFileSync(new URL('./cli-performance-prospective-policy-v1.json', import.meta.url));
function declarations() {
  const { profile, rounds } = fixture();
  const baselines = [rounds[0], rounds[1], rounds[3], rounds[4]];
  for (const host of profile.hosts) host.candidateBuild = { ...host.referenceBuild };
  const normalized = { format: 'zryna.cli-performance-normalized-baseline-declarations.v1',
    source: profile.sources.reference, catalogue, catalogueSha256: profile.catalogueSha256,
    collector: { collectorSha256: profile.collectorSha256 }, toolchainSha256: profile.toolchainSha256,
    tools: [], hosts: profile.hosts, rounds: baselines, sourceAuthenticated: false, proofQualified: false,
    performanceQualified: false, baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked' };
  const baseline = { reviewedSha256: 'e'.repeat(64), plan: { phase: 'baselines', runId: 111,
    policySha256: sha256(policy), source: { ...normalized.source, ref: 'refs/heads/fix/synthetic-anchor' } },
  jobs: baselines.map(round => ({ host: round.host, role: round.role,
    jobId: Number(round.identity.jobId), checkRunId: Number(round.identity.jobId),
    startedAt: round.startedAt, completedAt: round.endedAt,
    artifacts: [{ id: Number(round.identity.artifactId), sha256: round.identity.rawSha256 }] })) };
  return { normalized: structuredClone(normalized), baseline, anchor: { runId: 222, jobId: 333,
    startedAt: '2030-01-04T00:00:00Z' } };
}
function derive(value) {
  return deriveNumericAnchorDocuments(value.normalized, value.baseline, value.anchor, policy);
}
test('four acyclic original files derive44cells and preserve every original normalized identity without authority', () => {
  const files = derive(declarations()); assert.deepEqual([...files.keys()].sort(), NUMERIC_ANCHOR_FILES);
  const input = decodeNumericAnchorInput(files.get('anchor-input.json'));
  const profile = JSON.parse(files.get('numeric-profile.json')), derivation = JSON.parse(files.get('derivation.json'));
  assert.equal(input.baselines.length, 4); assert.equal(profile.cells.length, 44);
  assert.equal(profile.anchor.recordSha256, sha256(files.get('anchor-input.json')));
  assert.equal(profile.anchor.rationaleSha256, sha256(policy)); assert(files.get('rationale.json').equals(policy));
  assert.equal(profile.cells[0].maximumWallNs, '1100000000');
  assert.equal(profile.cells[0].maximumMemoryNativeUnits, '1050');
  assert.equal(derivation.status, 'derived-from-baseline-declarations'); assert.deepEqual(derivation.failures, []);
  assert(!Object.hasOwn(input, 'profileSha256')); assert(!Object.hasOwn(input, 'archiveSha256'));
  for (const value of [input, profile, derivation]) {
    for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified']) assert.equal(value[key], false);
    assert.equal(value.thresholdAuthority, null); assert.equal(value.closureStatus, 'blocked');
  }
});
test('retained warmup changes input identity but never substitutes for all10measured baseline maxima', () => {
  const f = declarations(), before = derive(f);
  f.normalized.rounds[0].cells[0].attempts[0].wallNs = '14000000000';
  const after = derive(f);
  assert.notEqual(sha256(before.get('anchor-input.json')), sha256(after.get('anchor-input.json')));
  assert.deepEqual(JSON.parse(before.get('numeric-profile.json')).cells, JSON.parse(after.get('numeric-profile.json')).cells);
  const changed = decodeNumericAnchorInput(after.get('anchor-input.json'));
  assert.equal(changed.baselines[0].normalizedSha256, sha256(canonical(f.normalized.rounds[0])));
});
test('late anchors crossed original pins heldout contamination and unstable baseline spreads cannot produce a table', () => {
  for (const mutate of [
    f => { f.anchor.startedAt = '2030-01-02T00:00:00Z'; },
    f => { f.baseline.jobs[0].artifacts[0].sha256 = '0'.repeat(64); },
    f => { f.normalized.rounds[0].role = 'held-out-candidate'; },
    f => { f.normalized.rounds[0].cells[0].attempts[1].wallNs = '1400000000'; },
    f => { f.normalized.performanceQualified = true; },
  ]) { const f = declarations(); mutate(f); assert.throws(() => derive(f)); }
  assert.throws(() => deriveNumericAnchorDocuments(...Object.values(declarations()), Buffer.from('{}\n')));
});
function currentFixture() {
  const f = campaignFixture('anchor', { commit: 'a'.repeat(40), tree: 'b'.repeat(40),
    ref: 'refs/heads/fix/synthetic-anchor' });
  const { run } = f, job = f.jobs[0], check = f.responses.get(`${NUMERIC_API_PREFIX}/check-runs/${job.id}`);
  for (const value of [run, job, check]) { value.status = 'in_progress'; value.conclusion = null; }
  run.created_at = '2029-12-31T23:59:58Z'; run.run_started_at = '2029-12-31T23:59:59Z';
  for (const [index, step] of job.steps.entries()) if (index >= 3) {
    step.status = index === 3 ? 'in_progress' : 'pending'; step.conclusion = null;
  }
  return { context: { phase: 'anchor', host: 'linux-x86_64', source: f.plan.source, runId: run.id },
    run, job, check, api: f.api };
}
test('in-progress anchor context binds actual job guards runner source and active fixed step without requiring future completion', async () => {
  const f = currentFixture(), actual = await observeCurrentNumericJob(f.context, f.api);
  assert.equal(actual.runId, f.run.id); assert.equal(actual.jobId, f.job.id); assert.equal(actual.startedAt, f.job.started_at);
  assert.deepEqual(actual.source, f.context.source);
});
test('current job cannot be replaced by copied success arbitrary active commands skipped guards or another source', async () => {
  for (const mutate of [f => { f.run.status = 'completed'; }, f => { f.job.head_sha = 'c'.repeat(40); },
    f => { f.job.steps[0].conclusion = 'skipped'; }, f => { f.job.steps[3].name = 'arbitrary command'; },
    f => { f.check.app.slug = 'another-app'; }, f => { f.job.runner_group_id = 1; },
  ]) { const f = currentFixture(); mutate(f); await assert.rejects(observeCurrentNumericJob(f.context, f.api)); }
});
test('strict command IDs and closed argument vectors cannot select another source reader or workflow', () => {
  assert.equal(numericRunId('123'), 123);
  for (const value of ['0', '001', '123;echo', '9007199254740993']) assert.throws(() => numericRunId(value));
  assert.throws(() => numericAnchorArguments(['--mode', 'verify-anchor', '--anchor-run-id', '123',
    '--platform', 'linux-x86_64', '--output', '/tmp/anchor', '--reader', 'override']));
  assert.throws(() => numericAnchorArguments(['--mode', 'build-anchor', '--baseline-run-id', '123', '--output', 'relative']));
});
test('copied verification reports and arbitrary keys cannot grant or revoke original table ownership', () => {
  for (const key of [{}, { tableOriginalBytesVerified: true, profile: {} }, Object.freeze({})]) {
    assert.throws(() => inspectNumericCampaignAnchor(key)); assert.throws(() => closeNumericCampaignAnchor(key));
  }
});
