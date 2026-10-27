// Platform identities only; archive contents and measurements require independent verification.
import assert from 'node:assert/strict';
import { performanceJobName, PERFORMANCE_REPOSITORY, PERFORMANCE_REPOSITORY_ID }
  from './cli-performance-hosted-expectations.mjs';

const prefix = `/repos/${PERFORMANCE_REPOSITORY}`;
function complete(value) {
  assert.equal(value.status, 'completed'); assert.equal(value.conclusion, 'success');
}
function timestamp(value) {
  assert(typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/u.test(value));
  const time = Date.parse(value); assert(Number.isFinite(time));
  assert.equal(new Date(time).toISOString(), value.replace('Z', '.000Z')); return time;
}
function successfulStep(job, name, conclusion = 'success') {
  const rows = job.steps.filter(step => step.name === name);
  assert.equal(rows.length, 1, `S418 hosted: exact required step ${name}`);
  assert.equal(rows[0].status, 'completed'); assert.equal(rows[0].conclusion, conclusion);
}
export async function verifyPerformancePlatformArtifacts(expected, run, api, now = Date.now()) {
  assert(Number.isFinite(now));
  assert(Number.isSafeInteger(run.check_suite_id) && run.check_suite_id > 0);
  const get = async endpoint => structuredClone(await api(endpoint));
  const [jobs, artifacts] = await Promise.all([
    get(`${prefix}/actions/runs/${expected.run.id}/attempts/1/jobs?per_page=100`),
    get(`${prefix}/actions/runs/${expected.run.id}/artifacts?per_page=100`),
  ]);
  assert.equal(jobs.total_count, 4); assert.equal(jobs.jobs?.length, 4);
  assert.equal(artifacts.total_count, 6); assert.equal(artifacts.artifacts?.length, 6);
  assert.equal(new Set(jobs.jobs.map(row => row.id)).size, 4);
  assert.equal(new Set(artifacts.artifacts.map(row => row.id)).size, 6);
  const verified = [];
  for (const wanted of expected.jobs) {
    const job = jobs.jobs.find(row => row.id === wanted.jobId);
    assert(job, 'S418 hosted: selected job missing from exact run attempt');
    complete(job); assert.equal(job.run_id, expected.run.id); assert.equal(job.run_attempt, 1);
    assert.equal(job.head_sha, expected.source.commit);
    const jobName = performanceJobName(wanted.host, wanted.round);
    assert.equal(job.name, jobName);
    assert.equal(job.check_run_url, `https://api.github.com${prefix}/check-runs/${wanted.checkRunId}`);
    assert(Array.isArray(job.steps) && job.steps.length <= 64);
    assert.equal(new Set(job.steps.map(step => step.number)).size, job.steps.length);
    for (const step of job.steps) {
      assert(Number.isSafeInteger(step.number) && step.number > 0);
      assert.equal(step.status, 'completed'); assert(['success', 'skipped'].includes(step.conclusion));
    }
    for (const name of ['Checkout exact observed head', 'Frozen workflow dependencies outside measurement',
      'Verify fixed collectors oracles and release capture controls', 'Locked Rust dependency preparation outside builder and measurement',
      'Assemble observed operator inputs and collect one fresh round', 'Retain all observations and failures']) successfulStep(job, name);
    successfulStep(job, 'Build Windows helper and run exactly eight controls once', wanted.host === 'windows-x86_64' ? 'success' : 'skipped');
    successfulStep(job, 'Retain Linux provider acquisition and loader observations', wanted.host === 'linux-x86_64' ? 'success' : 'skipped');
    const check = await get(`${prefix}/check-runs/${wanted.checkRunId}`);
    complete(check); assert.equal(check.id, wanted.checkRunId); assert.equal(check.name, jobName);
    assert.equal(check.head_sha, expected.source.commit); assert.equal(check.app?.slug, 'github-actions');
    assert.equal(check.check_suite?.id, run.check_suite_id);
    const start = timestamp(job.started_at), finish = timestamp(job.completed_at);
    assert(start <= finish && finish <= now, 'S418 hosted: completed job chronology');
    const retained = [];
    for (const descriptor of [wanted.primary, wanted.provider].filter(Boolean)) {
      const artifact = artifacts.artifacts.find(row => row.id === descriptor.id);
      assert(artifact, 'S418 hosted: selected artifact missing from exact run');
      assert.equal(artifact.name, descriptor.name); assert.equal(artifact.size_in_bytes, descriptor.bytes);
      assert.equal(artifact.digest, `sha256:${descriptor.sha256}`);
      assert.equal(artifact.expired, false); assert.equal(artifact.expires_at, descriptor.expiresAt);
      assert(timestamp(artifact.expires_at) > now, 'S418 hosted: artifact expired');
      assert.equal(artifact.workflow_run?.id, expected.run.id);
      assert.equal(artifact.workflow_run?.repository_id, PERFORMANCE_REPOSITORY_ID);
      assert.equal(artifact.workflow_run?.head_repository_id, PERFORMANCE_REPOSITORY_ID);
      assert.equal(artifact.workflow_run?.head_sha, expected.source.commit);
      assert.equal(artifact.workflow_run?.head_branch, run.head_branch);
      const created = timestamp(artifact.created_at);
      assert(start <= created && created <= finish, 'S418 hosted: artifact not created by selected job');
      retained.push({ id: artifact.id, sha256: descriptor.sha256, bytes: descriptor.bytes });
    }
    verified.push({ host: wanted.host, round: wanted.round, jobId: wanted.jobId, checkRunId: wanted.checkRunId, artifacts: retained });
  }
  return verified;
}
