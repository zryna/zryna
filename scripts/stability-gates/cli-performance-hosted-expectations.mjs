// Reviewed external expectations, not source approval or baseline/threshold authority.
import assert from 'node:assert/strict';
import { canonical, exact, MAX_DOCUMENT, parse, sha256 } from './input.mjs';

export const PERFORMANCE_REPOSITORY = 'zryna/zryna';
export const PERFORMANCE_REPOSITORY_ID = 1350402316;
export const PERFORMANCE_CALLER = '.github/workflows/cli-performance.yml';
export const PERFORMANCE_CALLEE = '.github/workflows/cli-performance-collector.yml';
export const PERFORMANCE_HOSTS = Object.freeze(['linux-x86_64', 'windows-x86_64']);
export const PERFORMANCE_ROUNDS = Object.freeze(['baseline-round-1', 'baseline-round-2']);
const runners = Object.freeze({ 'linux-x86_64': 'ubuntu-24.04', 'windows-x86_64': 'windows-2022' });
const sha1 = value => assert(typeof value === 'string' && /^[a-f0-9]{40}$/u.test(value), 'S418 hosted: Git identity');
const sha = value => assert(typeof value === 'string' && /^[a-f0-9]{64}$/u.test(value), 'S418 hosted: SHA-256');
const positive = value => assert(Number.isSafeInteger(value) && value > 0, 'S418 hosted: positive integer identity');

export function performanceJobName(host, round) {
  assert(PERFORMANCE_HOSTS.includes(host) && PERFORMANCE_ROUNDS.includes(round));
  return `observe / ${runners[host]} / ${round} (unqualified)`;
}
export function performanceArtifactName(host, round, runId, provider = false) {
  assert(PERFORMANCE_HOSTS.includes(host) && PERFORMANCE_ROUNDS.includes(round));
  positive(runId); assert(!provider || host === 'linux-x86_64');
  return provider ? `cli-performance-linux-provider-${round}-${runId}-1`
    : `cli-performance-${runners[host]}-${round}-${runId}-1`;
}
function artifact(value, name, maximum) {
  exact(value, ['id', 'name', 'bytes', 'sha256', 'expiresAt'], 'hosted artifact expectation');
  positive(value.id); positive(value.bytes); assert(value.bytes <= maximum, 'S418 hosted: archive byte bound');
  assert.equal(value.name, name); sha(value.sha256);
  assert(typeof value.expiresAt === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/u.test(value.expiresAt)
    && Number.isFinite(Date.parse(value.expiresAt)), 'S418 hosted: explicit artifact expiry');
  assert.equal(new Date(value.expiresAt).toISOString(), value.expiresAt.replace('Z', '.000Z'));
}
export function validateHostedPerformanceExpectations(value) {
  canonical(value);
  exact(value, ['format', 'repository', 'repositoryId', 'source', 'execution', 'run', 'workflows', 'jobs'], 'performance expectations');
  assert.equal(value.format, 'zryna.cli-performance-hosted-expectations.v1');
  assert.equal(value.repository, PERFORMANCE_REPOSITORY); assert.equal(value.repositoryId, PERFORMANCE_REPOSITORY_ID);
  exact(value.source, ['commit', 'tree', 'ref'], 'candidate source');
  sha1(value.source.commit); sha1(value.source.tree);
  assert(typeof value.source.ref === 'string' && value.source.ref.length <= 256
    && /^refs\/heads\/[A-Za-z0-9][A-Za-z0-9_./-]*$/u.test(value.source.ref)
    && !value.source.ref.includes('..') && !value.source.ref.endsWith('/'), 'S418 hosted: explicit candidate branch');
  assert(value.source.ref.slice('refs/heads/'.length).split('/').every(part =>
    /^[A-Za-z0-9_][A-Za-z0-9_.-]*$/u.test(part) && !part.endsWith('.') && !part.endsWith('.lock')));
  exact(value.execution, ['commit', 'tree', 'parents'], 'workflow execution source');
  sha1(value.execution.commit); sha1(value.execution.tree);
  assert(Array.isArray(value.execution.parents) && value.execution.parents.length <= 2);
  value.execution.parents.forEach(sha1);
  assert.equal(new Set(value.execution.parents).size, value.execution.parents.length);
  assert(!value.execution.parents.includes(value.execution.commit));
  exact(value.run, ['id', 'attempt', 'event', 'pullRequest'], 'workflow run');
  positive(value.run.id); assert.equal(value.run.attempt, 1, 'S418 hosted: fresh run only, no retry selection');
  assert(['pull_request', 'push', 'workflow_dispatch'].includes(value.run.event));
  if (value.run.event === 'pull_request') {
    positive(value.run.pullRequest);
    assert.equal(value.execution.parents.length, 2, 'S418 hosted: explicit synthetic merge parents');
    assert.equal(value.execution.parents[1], value.source.commit);
    assert.notEqual(value.execution.commit, value.source.commit);
  } else {
    assert.equal(value.run.pullRequest, null);
    assert.equal(value.execution.commit, value.source.commit);
    assert.equal(value.execution.tree, value.source.tree);
  }
  assert(Array.isArray(value.workflows) && value.workflows.length === 2);
  assert.deepEqual(value.workflows.map(row => row.path).sort(), [PERFORMANCE_CALLER, PERFORMANCE_CALLEE].sort());
  for (const workflow of value.workflows) {
    exact(workflow, ['path', 'bytes', 'sha256'], 'reviewed workflow descriptor');
    positive(workflow.bytes); assert(workflow.bytes <= 64 * 1024); sha(workflow.sha256);
  }
  assert(Array.isArray(value.jobs) && value.jobs.length === 4, 'S418 hosted: complete four-job selection');
  const jobIds = new Set(), checks = new Set(), artifacts = new Set(), roles = new Set();
  for (const job of value.jobs) {
    exact(job, ['host', 'round', 'jobId', 'checkRunId', 'primary', 'provider'], 'hosted job expectation');
    performanceJobName(job.host, job.round);
    positive(job.jobId); positive(job.checkRunId);
    const role = `${job.host}/${job.round}`;
    assert(!roles.has(role) && !jobIds.has(job.jobId) && !checks.has(job.checkRunId), 'S418 hosted: unique job/check/role');
    roles.add(role); jobIds.add(job.jobId); checks.add(job.checkRunId);
    artifact(job.primary, performanceArtifactName(job.host, job.round, value.run.id), 128 * 1024 * 1024);
    if (job.host === 'linux-x86_64') {
      artifact(job.provider, performanceArtifactName(job.host, job.round, value.run.id, true), 1024 * 1024);
    } else assert.equal(job.provider, null, 'S418 hosted: Windows cannot borrow a Linux provider');
    for (const record of [job.primary, job.provider].filter(Boolean)) {
      assert(!artifacts.has(record.id), 'S418 hosted: distinct primary/provider artifact identities'); artifacts.add(record.id);
    }
  }
  assert.equal(roles.size, 4); assert.equal(artifacts.size, 6);
  return value;
}
export function parseHostedPerformanceExpectations(bytes, reviewedSha256) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= MAX_DOCUMENT);
  sha(reviewedSha256);
  assert.equal(sha256(bytes), reviewedSha256, 'S418 hosted: independently supplied expectations digest');
  return validateHostedPerformanceExpectations(parse(bytes));
}
