// Structural limits complement independently reviewed complete workflow-byte descriptors.
import assert from 'node:assert/strict';
import { parseDocument } from 'yaml';
import { exact } from './input.mjs';

function document(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= 64 * 1024);
  const parsed = parseDocument(new TextDecoder('utf-8', { fatal: true }).decode(bytes), { uniqueKeys: true });
  assert.deepEqual(parsed.errors, []); assert.deepEqual(parsed.warnings, []);
  return parsed.toJS({ maxAliasCount: 0 });
}
export function validatePerformanceWorkflowPair(callerBytes, calleeBytes, event) {
  const caller = document(callerBytes), callee = document(calleeBytes);
  for (const value of [caller, callee]) {
    exact(value, ['name', 'on', 'permissions', 'concurrency', 'jobs'], 'performance workflow');
    assert.deepEqual(value.permissions, { contents: 'read' });
    exact(value.concurrency, ['group', 'cancel-in-progress'], 'workflow concurrency');
    assert.equal(value.concurrency['cancel-in-progress'], true);
    assert(typeof value.concurrency.group === 'string' && value.concurrency.group.length <= 256);
  }
  assert.notEqual(caller.concurrency.group, callee.concurrency.group, 'S418 hosted: caller/callee must not cancel each other');
  assert(caller.on && Object.hasOwn(caller.on, event), 'S418 hosted: declared caller event');
  assert(Object.keys(caller.on).every(key => ['pull_request', 'workflow_dispatch', 'push'].includes(key)));
  exact(caller.jobs, ['observe'], 'fixed caller job');
  assert.deepEqual(caller.jobs.observe, { name: 'observe', uses: './.github/workflows/cli-performance-collector.yml' },
    'S418 hosted: no caller inputs, secrets, environment or success bypass');
  assert(callee.on && Object.keys(callee.on).length === 1 && Object.hasOwn(callee.on, 'workflow_call'));
  assert(callee.on.workflow_call === null || Object.keys(callee.on.workflow_call).length === 0,
    'S418 hosted: baseline callee accepts no caller-controlled options');
  exact(callee.jobs, ['collect'], 'fixed collector job');
  const job = callee.jobs.collect;
  exact(job, ['name', 'runs-on', 'timeout-minutes', 'strategy', 'env', 'steps'], 'fixed collector job fields');
  assert.equal(job.name, '${{ matrix.os }} / ${{ matrix.round }} (unqualified)');
  assert.equal(job['runs-on'], '${{ matrix.os }}'); assert.equal(job['timeout-minutes'], 60);
  assert.deepEqual(job.strategy, { 'fail-fast': false, matrix: {
    os: ['ubuntu-24.04', 'windows-2022'], round: ['baseline-round-1', 'baseline-round-2'],
  } });
  assert.equal(job.env.EXPECTED_SOURCE_REVISION, '${{ github.event.pull_request.head.sha || github.sha }}');
  assert.equal(job.env.DECLARED_PR_HEAD, '${{ github.event.pull_request.head.sha || github.sha }}');
  assert.equal(job.env.M418_ROUND, '${{ matrix.round }}');
  assert.equal(job.env.CARGO_BUILD_JOBS, 2); assert.equal(job.env.CARGO_INCREMENTAL, 0);
  assert(Array.isArray(job.steps) && job.steps.length > 0 && job.steps.length <= 32);
  const checkouts = job.steps.filter(step => step.uses?.startsWith('actions/checkout@'));
  assert.equal(checkouts.length, 1); assert.equal(job.steps[0], checkouts[0]);
  assert.equal(checkouts[0].uses, 'actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1');
  assert.deepEqual(checkouts[0].with, { 'fetch-depth': 0, 'persist-credentials': false,
    ref: '${{ github.event.pull_request.head.sha || github.sha }}' });
  for (const step of job.steps) {
    assert(!Object.hasOwn(step, 'continue-on-error'), 'S418 hosted: failed proof cannot be waived');
    if (step.uses) assert.match(step.uses, /^[A-Za-z0-9_./-]+@[a-f0-9]{40}$/u);
  }
  const operator = job.steps.filter(step => step.name === 'Assemble observed operator inputs and collect one fresh round');
  assert.equal(operator.length, 1); assert.equal(operator[0].shell, 'pwsh');
  assert(operator[0].run.includes('scripts/stability-gates/run-cli-performance-hosted.mjs'));
  assert(operator[0].run.includes("$head -ne $env:EXPECTED_SOURCE_REVISION"));
  return { workflowStructureVerified: true, performanceQualified: false, thresholds: null };
}
