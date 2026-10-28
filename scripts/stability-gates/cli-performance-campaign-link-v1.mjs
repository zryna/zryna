// Actual platform identities only. Original-byte replay and final qualification are separate owners.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { githubApi } from '../m4/hosted-api.mjs';
import { authenticateWorkflowReference } from '../m4/hosted-workflow.mjs';
import { MAX_DOCUMENT, sha256 } from './input.mjs';
import { PERFORMANCE_REPOSITORY, PERFORMANCE_REPOSITORY_ID } from './cli-performance-hosted-expectations.mjs';
import { NUMERIC_LABELS, NUMERIC_WORKFLOWS, numericArtifactNames, numericSlots, numericTimestamp,
  parseNumericCampaignPlan, requiredNumericSteps } from './cli-performance-campaign-plan-v1.mjs';
const selections = new WeakMap(), prefix = `/repos/${PERFORMANCE_REPOSITORY}`;
function repository(value) {
  assert.equal(value?.id, PERFORMANCE_REPOSITORY_ID); assert.equal(value?.full_name, PERFORMANCE_REPOSITORY);
}
function complete(value) { assert.equal(value.status, 'completed'); assert.equal(value.conclusion, 'success'); }
async function uniqueDispatch(get, plan) {
  const path = `cli-performance-${plan.phase}-v1.yml`;
  const history = await get(`${prefix}/actions/workflows/${path}/runs?event=workflow_dispatch&head_sha=${plan.source.commit}&per_page=100`);
  assert.equal(history.total_count, 1, 'one prospective phase dispatch; no alternate fresh-run selection');
  assert.equal(history.workflow_runs?.length, 1);
  const selected = history.workflow_runs[0];
  assert.equal(selected.id, plan.runId); assert.equal(selected.run_attempt, 1);
  assert.equal(selected.head_sha, plan.source.commit); assert.equal(selected.event, 'workflow_dispatch');
  assert.equal(selected.head_branch, plan.source.ref.slice(11));
  assert.equal(selected.path, `.github/workflows/${path}`);
}
function validateRun(run, plan) {
  complete(run); repository(run.repository); repository(run.head_repository);
  assert.equal(run.id, plan.runId); assert.equal(run.run_attempt, 1, 'no replacement measurement attempt');
  assert.equal(run.event, 'workflow_dispatch'); assert.equal(run.head_sha, plan.source.commit);
  assert.equal(run.head_branch, plan.source.ref.slice('refs/heads/'.length));
  assert.equal(run.path, `.github/workflows/cli-performance-${plan.phase}-v1.yml`);
  assert(Number.isSafeInteger(run.check_suite_id) && run.check_suite_id > 0);
  const references = run.referenced_workflows ?? [];
  assert(Array.isArray(references));
  assert.equal(references.length, plan.phase === 'anchor' ? 0 : 1, 'exact numeric reusable workflow attachment');
  for (const reference of references)
    authenticateWorkflowReference(reference, PERFORMANCE_REPOSITORY, NUMERIC_WORKFLOWS[3], plan.source.commit, run);
}
function workflow(value, descriptor) {
  assert.equal(value.type, 'file'); assert.equal(value.path, descriptor.path);
  assert.equal(value.encoding, 'base64'); assert.equal(value.size, descriptor.bytes);
  assert(typeof value.content === 'string' && value.content.length <= 128 * 1024);
  const encoded = value.content.replaceAll('\n', ''); assert(/^[A-Za-z0-9+/]*={0,2}$/u.test(encoded));
  const bytes = Buffer.from(encoded, 'base64'); assert.equal(bytes.toString('base64'), encoded);
  assert.equal(bytes.length, descriptor.bytes); assert.equal(sha256(bytes), descriptor.sha256);
  assert.equal(value.sha, createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex'));
}
function validateJob(job, slot, run, plan, now) {
  complete(job); assert.equal(job.run_id, run.id); assert.equal(job.run_attempt, 1);
  assert.equal(job.head_sha, plan.source.commit); assert.equal(job.name, slot.name);
  assert(Number.isSafeInteger(job.id) && job.id > 0);
  assert.deepEqual(job.labels, [NUMERIC_LABELS[slot.host]], 'exact supported hosted image');
  assert.equal(job.runner_group_id, 0); assert.equal(job.runner_group_name, 'GitHub Actions');
  assert(Number.isSafeInteger(job.runner_id) && job.runner_id > 0);
  assert.equal(job.runner_name, `GitHub Actions ${job.runner_id}`);
  assert(Array.isArray(job.steps) && job.steps.length > 0 && job.steps.length <= 64);
  assert.equal(new Set(job.steps.map(step => step.number)).size, job.steps.length);
  for (const step of job.steps) {
    assert(Number.isSafeInteger(step.number) && step.number > 0);
    assert.equal(step.status, 'completed'); assert(['success', 'skipped'].includes(step.conclusion));
  }
  for (const [name, conclusion] of requiredNumericSteps(slot)) {
    const matches = job.steps.filter(step => step.name === name);
    assert.equal(matches.length, 1, `mandatory numeric step ${name}`); assert.equal(matches[0].conclusion, conclusion);
  }
  const start = numericTimestamp(job.started_at), finish = numericTimestamp(job.completed_at);
  assert(start < finish && finish <= now);
  return { start, finish };
}
function validateArtifact(value, name, run, plan, interval, now, provider) {
  assert.equal(value.name, name); assert(Number.isSafeInteger(value.id) && value.id > 0);
  assert(Number.isSafeInteger(value.size_in_bytes) && value.size_in_bytes > 0);
  assert(value.size_in_bytes <= (provider || plan.phase === 'anchor' ? 1024 * 1024 : 128 * 1024 * 1024));
  assert(typeof value.digest === 'string' && /^sha256:[a-f0-9]{64}$/u.test(value.digest));
  assert.equal(value.expired, false); assert(numericTimestamp(value.expires_at) > now);
  const origin = value.workflow_run;
  assert.equal(origin?.id, run.id); assert.equal(origin.repository_id, PERFORMANCE_REPOSITORY_ID);
  assert.equal(origin.head_repository_id, PERFORMANCE_REPOSITORY_ID);
  assert.equal(origin.head_sha, plan.source.commit); assert.equal(origin.head_branch, run.head_branch);
  const created = numericTimestamp(value.created_at);
  assert(interval.start <= created && created <= interval.finish);
  return { id: value.id, name, bytes: value.size_in_bytes, sha256: value.digest.slice(7),
    createdAt: value.created_at, expiresAt: value.expires_at };
}
export async function linkNumericCampaignPhase(bytes, reviewedSha256, api = githubApi(), now = Date.now()) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= MAX_DOCUMENT, 'bounded original campaign plan before copying');
  assert(Number.isSafeInteger(now));
  const plan = parseNumericCampaignPlan(Buffer.from(bytes), reviewedSha256);
  const get = async endpoint => structuredClone(await api(endpoint));
  const run = await get(`${prefix}/actions/runs/${plan.runId}`); validateRun(run, plan);
  await uniqueDispatch(get, plan);
  const commit = await get(`${prefix}/git/commits/${plan.source.commit}`);
  assert.equal(commit.sha, plan.source.commit); assert.equal(commit.tree?.sha, plan.source.tree);
  for (const descriptor of plan.workflows)
    workflow(await get(`${prefix}/contents/${descriptor.path}?ref=${plan.source.commit}`), descriptor);
  const [jobs, artifacts] = await Promise.all([
    get(`${prefix}/actions/runs/${plan.runId}/attempts/1/jobs?per_page=100`),
    get(`${prefix}/actions/runs/${plan.runId}/artifacts?per_page=100`),
  ]);
  const slots = numericSlots(plan.phase), count = slots.reduce((n, slot) => n + numericArtifactNames(slot, run.id).length, 0);
  assert.equal(jobs.total_count, slots.length); assert.equal(jobs.jobs?.length, slots.length);
  assert.equal(artifacts.total_count, count); assert.equal(artifacts.artifacts?.length, count);
  assert.equal(new Set(jobs.jobs.map(job => job.id)).size, slots.length);
  assert.equal(new Set(artifacts.artifacts.map(artifact => artifact.id)).size, count);
  assert.equal(new Set(artifacts.artifacts.map(artifact => artifact.name)).size, count);
  const selected = [];
  for (const slot of slots) {
    const matches = jobs.jobs.filter(job => job.name === slot.name); assert.equal(matches.length, 1);
    const job = matches[0], interval = validateJob(job, slot, run, plan, now);
    assert.equal(job.check_run_url, `https://api.github.com${prefix}/check-runs/${job.id}`);
    const check = await get(`${prefix}/check-runs/${job.id}`); complete(check);
    assert.equal(check.id, job.id); assert.equal(check.name, slot.name); assert.equal(check.head_sha, plan.source.commit);
    assert.equal(check.app?.slug, 'github-actions'); assert.equal(check.check_suite?.id, run.check_suite_id);
    const retained = numericArtifactNames(slot, run.id).map((name, index) => {
      const values = artifacts.artifacts.filter(value => value.name === name); assert.equal(values.length, 1);
      return validateArtifact(values[0], name, run, plan, interval, now, index === 1);
    });
    selected.push({ ...slot, jobId: job.id, checkRunId: check.id, startedAt: job.started_at,
      completedAt: job.completed_at, artifacts: retained,
      virtualization: { status: 'observed', value: 'github-actions-hosted-vm' } });
  }
  if (plan.phase === 'baselines') for (const index of [0, 2])
    assert(numericTimestamp(selected[index].completedAt) < numericTimestamp(selected[index + 1].startedAt), 'serial baseline jobs per host');
  const final = await get(`${prefix}/actions/runs/${plan.runId}`); validateRun(final, plan);
  assert.equal(final.check_suite_id, run.check_suite_id);
  await uniqueDispatch(get, plan);
  const key = Object.freeze({}); selections.set(key, { plan, reviewedSha256, jobs: selected }); return key;
}
export function inspectNumericCampaignPhase(key) {
  const value = selections.get(key); assert(value, 'live numeric campaign selection required, not a copied report');
  return structuredClone({ format: 'zryna.cli-performance-campaign-platform-link.v1', ...value,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false, hostAdmitted: false,
    baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked' });
}
