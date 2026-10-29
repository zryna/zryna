// Ongoing fixed job observations; completed phase authority remains in campaign-link-v1.
import assert from 'node:assert/strict';
import { basename, dirname } from 'node:path';
import { canonical, MAX_DOCUMENT, readSafe, sha256 } from './input.mjs';
import { snapshot } from './source.mjs';
import { authenticateWorkflowReference } from '../m4/hosted-workflow.mjs';
import { PERFORMANCE_REPOSITORY, PERFORMANCE_REPOSITORY_ID } from './cli-performance-hosted-expectations.mjs';
import { PROSPECTIVE_POLICY_SHA256 } from './cli-performance-prospective-policy-v1.mjs';
import { NUMERIC_WORKFLOWS, NUMERIC_WORKFLOW_PINS, NUMERIC_LABELS, numericSlots,
  numericTimestamp, parseNumericCampaignPlan } from './cli-performance-campaign-plan-v1.mjs';
export const NUMERIC_API_PREFIX = `/repos/${PERFORMANCE_REPOSITORY}`;
export function numericRunId(value) {
  assert(typeof value === 'string' && /^[1-9][0-9]{0,15}$/u.test(value));
  const id = Number(value); assert(Number.isSafeInteger(id) && id > 0); return id;
}
export function originalNumericPlan(root, source, phase, runId) {
  const workflows = NUMERIC_WORKFLOWS.map((path, index) => {
    const bytes = readSafe(root, path, 65536), expected = NUMERIC_WORKFLOW_PINS[index];
    assert.equal(bytes.length, expected.bytes); assert.equal(sha256(bytes), expected.sha256);
    return { path, bytes: bytes.length, sha256: sha256(bytes) };
  });
  const bytes = Buffer.from(canonical({ format: 'zryna.cli-performance-campaign-plan.v1',
    source, phase, runId, policySha256: PROSPECTIVE_POLICY_SHA256, workflows }));
  parseNumericCampaignPlan(bytes, sha256(bytes)); return bytes;
}
function repository(value) {
  assert.equal(value?.id, PERFORMANCE_REPOSITORY_ID); assert.equal(value?.full_name, PERFORMANCE_REPOSITORY);
}
export function numericRuntimeContext(root, phase, host, selectedId) {
  assert(['anchor', 'heldout'].includes(phase));
  assert(phase === 'anchor' ? host === 'linux-x86_64' : Object.hasOwn(NUMERIC_LABELS, host));
  const env = process.env;
  assert.equal(env.GITHUB_ACTIONS, 'true'); assert.equal(env.GITHUB_EVENT_NAME, 'workflow_dispatch');
  assert.equal(env.GITHUB_REPOSITORY, PERFORMANCE_REPOSITORY); assert.equal(env.GITHUB_RUN_ATTEMPT, '1');
  const runId = numericRunId(env.GITHUB_RUN_ID), actual = snapshot(root);
  assert.equal(env.GITHUB_SHA, actual.commit); assert.equal(env.EXPECTED_SOURCE_REVISION, actual.commit);
  const source = { commit: actual.commit, tree: actual.tree, ref: env.GITHUB_REF };
  originalNumericPlan(root, source, phase, runId);
  assert.equal(env.RUNNER_OS, host === 'linux-x86_64' ? 'Linux' : 'Windows'); assert.equal(env.RUNNER_ARCH, 'X64');
  if (phase === 'heldout') {
    assert.equal(env.M418_HOST, host); assert.equal(env.M418_ROUND, 'held-out-candidate');
    assert.equal(numericRunId(env.M418_ANCHOR_RUN_ID), selectedId);
  } else assert.equal(numericRunId(env.M418_BASELINE_RUN_ID), selectedId);
  const eventPath = env.GITHUB_EVENT_PATH;
  assert(typeof eventPath === 'string');
  const eventBytes = readSafe(dirname(eventPath), basename(eventPath), MAX_DOCUMENT);
  const event = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(eventBytes));
  canonical(event); // Existing document depth/node/string bounds; GitHub owns original event spelling.
  repository(event.repository);
  assert(!event.ref || event.ref === source.ref || `refs/heads/${event.ref}` === source.ref);
  assert.equal(numericRunId(event.inputs?.[phase === 'anchor' ? 'baseline_run_id' : 'anchor_run_id']), selectedId);
  return { root, phase, host, runId, source, actual };
}
export async function observeCurrentNumericJob(context, api) {
  const { source, phase, runId, host } = context, prefix = NUMERIC_API_PREFIX;
  const run = await api(`${prefix}/actions/runs/${runId}`);
  repository(run.repository); repository(run.head_repository);
  assert.equal(run.id, runId); assert.equal(run.run_attempt, 1); assert.equal(run.event, 'workflow_dispatch');
  assert.equal(run.head_sha, source.commit); assert.equal(run.head_branch, source.ref.slice(11));
  assert.equal(run.path, `.github/workflows/cli-performance-${phase}-v1.yml`);
  assert.equal(run.status, 'in_progress'); assert.equal(run.conclusion, null);
  assert(Number.isSafeInteger(run.check_suite_id) && run.check_suite_id > 0);
  const references = run.referenced_workflows ?? [];
  assert.equal(references.length, phase === 'anchor' ? 0 : 1);
  for (const reference of references)
    authenticateWorkflowReference(reference, PERFORMANCE_REPOSITORY, NUMERIC_WORKFLOWS[3], source.commit, run);
  const created = numericTimestamp(run.created_at), started = numericTimestamp(run.run_started_at);
  assert(created <= started);
  const history = await api(`${prefix}/actions/workflows/cli-performance-${phase}-v1.yml/runs?event=workflow_dispatch&head_sha=${source.commit}&per_page=100`);
  assert.equal(history.total_count, 1); assert.equal(history.workflow_runs?.length, 1);
  assert.equal(history.workflow_runs[0].id, runId); assert.equal(history.workflow_runs[0].run_attempt, 1);
  const commit = await api(`${prefix}/git/commits/${source.commit}`);
  assert.equal(commit.sha, source.commit); assert.equal(commit.tree?.sha, source.tree);
  const response = await api(`${prefix}/actions/runs/${runId}/attempts/1/jobs?per_page=100`);
  const slots = numericSlots(phase); assert.equal(response.total_count, slots.length); assert.equal(response.jobs?.length, slots.length);
  assert.deepEqual(response.jobs.map(job => job.name).sort(), slots.map(slot => slot.name).sort());
  const name = slots.find(slot => slot.host === host).name;
  const matching = response.jobs.filter(job => job.name === name); assert.equal(matching.length, 1); const job = matching[0];
  assert.equal(job.run_id, runId); assert.equal(job.run_attempt, 1); assert.equal(job.head_sha, source.commit);
  assert.equal(job.status, 'in_progress'); assert.equal(job.conclusion, null);
  assert(Array.isArray(job.steps) && job.steps.length <= 64);
  for (const name of ['Guard fixed ' + (phase === 'anchor' ? 'independent anchor phase' : 'prospective numeric collection'),
    'Checkout exact observed head', 'Frozen workflow dependencies outside measurement']) {
    const matches = job.steps.filter(step => step.name === name); assert.equal(matches.length, 1);
    assert.equal(matches[0].status, 'completed'); assert.equal(matches[0].conclusion, 'success');
  }
  const active = job.steps.filter(step => step.status === 'in_progress'); assert.equal(active.length, 1);
  assert.equal(active[0].conclusion, null);
  assert((phase === 'anchor' ? ['Reconcile completed baselines and derive fixed immutable table'] :
    ['Verify completed numeric anchor before any release build or measurement',
      'Assemble observed operator inputs and collect one fresh round']).includes(active[0].name));
  assert(Number.isSafeInteger(job.id) && job.id > 0);
  assert.deepEqual(job.labels, [NUMERIC_LABELS[host]]); assert.equal(job.runner_group_id, 0);
  assert.equal(job.runner_group_name, 'GitHub Actions');
  assert(Number.isSafeInteger(job.runner_id) && job.runner_id > 0); assert.equal(job.runner_name, `GitHub Actions ${job.runner_id}`);
  assert.equal(job.check_run_url, `https://api.github.com${prefix}/check-runs/${job.id}`);
  const check = await api(`${prefix}/check-runs/${job.id}`);
  assert.equal(check.id, job.id); assert.equal(check.name, name); assert.equal(check.head_sha, source.commit);
  assert.equal(check.app?.slug, 'github-actions'); assert.equal(check.check_suite?.id, run.check_suite_id);
  assert.equal(check.status, 'in_progress'); assert.equal(check.conclusion, null);
  assert(started <= numericTimestamp(job.started_at));
  return { phase, host, source, runId, jobId: job.id, createdAt: run.created_at,
    runStartedAt: run.run_started_at, startedAt: job.started_at };
}
