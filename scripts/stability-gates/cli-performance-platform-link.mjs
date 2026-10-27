// Online platform metadata linkage only. Raw artifacts, host class and performance remain separate.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { githubApi } from '../m4/hosted-api.mjs';
import { authenticateWorkflowReference } from '../m4/hosted-workflow.mjs';
import { sha256 } from './input.mjs';
import { parseHostedPerformanceExpectations, PERFORMANCE_CALLER, PERFORMANCE_CALLEE,
  PERFORMANCE_REPOSITORY, PERFORMANCE_REPOSITORY_ID } from './cli-performance-hosted-expectations.mjs';
import { validatePerformanceWorkflowPair } from './cli-performance-workflow-pair.mjs';
import { verifyPerformancePlatformArtifacts } from './cli-performance-platform-artifacts.mjs';

const linked = new WeakMap();
const prefix = `/repos/${PERFORMANCE_REPOSITORY}`;
function repository(value) {
  assert.equal(value?.id, PERFORMANCE_REPOSITORY_ID, 'S418 hosted: platform repository ID');
  assert.equal(value?.full_name, PERFORMANCE_REPOSITORY, 'S418 hosted: platform repository name');
}
function commit(value, expected, parents = undefined) {
  assert.equal(value?.sha, expected.commit, 'S418 hosted: immutable commit');
  assert.equal(value.tree?.sha, expected.tree, 'S418 hosted: immutable tree');
  if (parents) assert.deepEqual(value.parents?.map(row => row.sha), parents, 'S418 hosted: immutable execution parents');
}
function workflowBytes(value, descriptor) {
  assert.equal(value?.type, 'file'); assert.equal(value.path, descriptor.path);
  assert.equal(value.encoding, 'base64'); assert.equal(value.size, descriptor.bytes);
  assert(typeof value.content === 'string' && value.content.length <= 128 * 1024);
  const encoded = value.content.replaceAll('\n', '');
  assert(/^[A-Za-z0-9+/]*={0,2}$/u.test(encoded));
  const bytes = Buffer.from(encoded, 'base64'); assert.equal(bytes.toString('base64'), encoded);
  assert.equal(bytes.length, descriptor.bytes); assert.equal(sha256(bytes), descriptor.sha256);
  const blob = createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
  assert.equal(value.sha, blob, 'S418 hosted: reviewed workflow Git blob');
  return bytes;
}
function validateRun(run, expected) {
  assert.equal(run.id, expected.run.id); assert.equal(run.run_attempt, expected.run.attempt);
  repository(run.repository); repository(run.head_repository);
  assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  assert.equal(run.event, expected.run.event); assert.equal(run.path, PERFORMANCE_CALLER);
  assert.equal(run.head_sha, expected.source.commit);
  assert.equal(run.head_branch, expected.source.ref.slice('refs/heads/'.length));
  assert(Array.isArray(run.referenced_workflows) && run.referenced_workflows.length === 1,
    'S418 hosted: exact same-run reusable execution attachment required');
  const reference = run.referenced_workflows[0];
  authenticateWorkflowReference(reference, PERFORMANCE_REPOSITORY, PERFORMANCE_CALLEE, expected.execution.commit, run);
  if (run.event === 'pull_request') {
    assert(Array.isArray(run.pull_requests) && run.pull_requests.length <= 32);
    const attached = run.pull_requests.filter(row => row.number === expected.run.pullRequest);
    assert.equal(attached.length, 1, 'S418 hosted: explicitly selected attached PR');
    const pr = attached[0];
    assert.equal(pr.head?.sha, expected.source.commit); assert.equal(pr.head?.ref, run.head_branch);
    assert.equal(pr.base?.sha, expected.execution.parents[0]);
    assert.equal(pr.head?.repo?.id, PERFORMANCE_REPOSITORY_ID);
    assert.equal(pr.base?.repo?.id, PERFORMANCE_REPOSITORY_ID);
  }
}
export async function linkReviewedPerformanceRun(expectationsBytes, reviewedSha256, api = githubApi(), now = Date.now()) {
  // Parsing creates an isolated snapshot before any asynchronous API call or callback.
  const expected = parseHostedPerformanceExpectations(Buffer.from(expectationsBytes), reviewedSha256);
  const get = async endpoint => structuredClone(await api(endpoint));
  const run = await get(`${prefix}/actions/runs/${expected.run.id}`);
  validateRun(run, expected);
  const candidate = await get(`${prefix}/git/commits/${expected.source.commit}`);
  commit(candidate, expected.source);
  const execution = expected.execution.commit === expected.source.commit ? candidate
    : await get(`${prefix}/git/commits/${expected.execution.commit}`);
  commit(execution, expected.execution, expected.execution.parents);
  const executionSources = new Map();
  for (const identity of new Set([expected.source.commit, expected.execution.commit])) {
    const sources = new Map();
    for (const descriptor of expected.workflows) {
      const response = await get(`${prefix}/contents/${descriptor.path}?ref=${identity}`);
      sources.set(descriptor.path, workflowBytes(response, descriptor));
    }
    validatePerformanceWorkflowPair(sources.get(PERFORMANCE_CALLER), sources.get(PERFORMANCE_CALLEE), run.event);
    executionSources.set(identity, sources);
  }
  const jobs = await verifyPerformancePlatformArtifacts(expected, run, get, now);
  const finalRun = await get(`${prefix}/actions/runs/${expected.run.id}`);
  validateRun(finalRun, expected);
  assert.equal(finalRun.check_suite_id, run.check_suite_id, 'S418 hosted: stable run suite');
  const key = Object.freeze({});
  linked.set(key, { expected, reviewedSha256, run: {
    id: run.id, attempt: run.run_attempt, event: run.event, headCommit: run.head_sha,
    executionCommit: run.referenced_workflows[0].sha, executionRef: run.referenced_workflows[0].ref,
  }, workflowSourcesVerified: executionSources.size, platformArtifactsVerified: true, jobs });
  return key;
}
export function inspectPerformancePlatformLink(selection) {
  const value = linked.get(selection);
  assert(value, 'S418 hosted: live internally linked selection required, not a caller-authored report');
  return structuredClone({ format: 'zryna.cli-performance-platform-link.v1', ...value,
    artifactsVerified: false, sourceAuthenticated: false, performanceQualified: false, thresholds: null });
}
