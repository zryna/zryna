import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync } from 'node:fs';
import { validatePerformanceWorkflowPair } from '../scripts/stability-gates/cli-performance-workflow-pair.mjs';
import { platformArguments } from '../scripts/stability-gates/verify-cli-performance-platform.mjs';
import { linkReviewedPerformanceRun, inspectPerformancePlatformLink } from '../scripts/stability-gates/cli-performance-platform-link.mjs';
import { platformFixture, PREFIX, CALLER, CALLEE, NOW } from './cli-performance-platform-fixture.mjs';

test('live run, immutable Git identities and both reviewed workflow blobs link without qualifying measurements', async () => {
  for (const pr of [false, true]) {
    const f = platformFixture(pr), key = await linkReviewedPerformanceRun(...f.selection());
    const report = inspectPerformancePlatformLink(key);
    assert.equal(report.workflowSourcesVerified, pr ? 2 : 1);
    assert.equal(report.run.executionCommit, f.value.execution.commit);
    assert.equal(report.run.headCommit, f.value.source.commit);
    assert.deepEqual([report.artifactsVerified, report.sourceAuthenticated, report.performanceQualified, report.thresholds],
      [false, false, false, null]);
    assert.equal(report.platformArtifactsVerified, true);
    assert.equal(report.jobs.flatMap(job => job.artifacts).length, 6);
    assert.equal(f.calls.length, pr ? 14 : 11);
    assert(Object.isFrozen(key));
    report.expected.source.commit = '9'.repeat(40); report.sourceAuthenticated = true;
    assert.equal(inspectPerformancePlatformLink(key).expected.source.commit, f.value.source.commit);
    for (const forged of [{}, { ...key }, report, JSON.parse(JSON.stringify(report))])
      assert.throws(() => inspectPerformancePlatformLink(forged));
  }
});
test('separate same-head CI and missing, extra, substituted or wrong-event workflow references cannot attach a run', async () => {
  const mutations = [f => { f.run.referenced_workflows = []; },
    f => { f.run.referenced_workflows.push(structuredClone(f.run.referenced_workflows[0])); },
    f => { f.run.referenced_workflows[0].sha = '9'.repeat(40); },
    f => { f.run.referenced_workflows[0].path = `other/zryna/${CALLEE}@${f.value.execution.commit}`; },
    f => { f.run.referenced_workflows[0].ref = 'refs/heads/main'; }];
  for (const mutate of mutations) {
    const f = platformFixture(); await linkReviewedPerformanceRun(...f.selection());
    mutate(f); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
  }
});
test('failed retried forked stale or wrong-branch platform run rejects before source admission', async () => {
  const mutations = [f => { f.run.id++; }, f => { f.run.run_attempt++; }, f => { f.run.status = 'in_progress'; },
    f => { f.run.conclusion = 'failure'; }, f => { f.run.event = 'push'; }, f => { f.run.path = CALLEE; },
    f => { f.run.head_sha = '9'.repeat(40); }, f => { f.run.head_branch = 'main'; },
    f => { f.run.repository.id++; }, f => { f.run.head_repository.id++; },
    f => { f.run.repository.full_name = 'other/zryna'; }];
  for (const mutate of mutations) {
    const f = platformFixture(); await linkReviewedPerformanceRun(...f.selection());
    mutate(f); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
  }
});
test('PR head base number repository parents and synthetic execution remain independently bound', async () => {
  const mutations = [f => { f.run.pull_requests[0].number++; }, f => { f.run.pull_requests[0].head.sha = '9'.repeat(40); },
    f => { f.run.pull_requests[0].base.sha = '9'.repeat(40); }, f => { f.run.pull_requests[0].head.repo.id++; },
    f => { f.run.pull_requests[0].base.repo.id++; },
    f => { f.responses.get(`${PREFIX}/git/commits/${f.value.execution.commit}`).parents.reverse(); },
    f => { f.responses.get(`${PREFIX}/git/commits/${f.value.execution.commit}`).tree.sha = '9'.repeat(40); }];
  for (const mutate of mutations) {
    const f = platformFixture(true); await linkReviewedPerformanceRun(...f.selection());
    mutate(f); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
  }
});
test('candidate and execution workflow sources both require exact content size hash and Git blob identity', async () => {
  for (const identity of ['source', 'execution']) for (const path of [CALLER, CALLEE]) {
    for (const change of [r => { r.size++; }, r => { r.sha = '9'.repeat(40); }, r => { r.content += 'AAAA'; },
      r => { r.content = '!'; }, r => { r.path = 'other.yml'; }, r => { r.encoding = 'utf8'; }]) {
      const f = platformFixture(true); await linkReviewedPerformanceRun(...f.selection());
      change(f.responses.get(`${PREFIX}/contents/${path}?ref=${f.value[identity].commit}`));
      await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
    }
  }
});
test('reviewed bytes alone cannot introduce caller overrides, secret grants, retries or relaxed job bounds', async () => {
  const mutations = [f => { f.caller.jobs.observe.with = { round: 'heldout' }; },
    f => { f.caller.jobs.observe.secrets = 'inherit'; }, f => { f.callee.permissions.contents = 'write'; },
    f => { f.callee.jobs.collect['timeout-minutes'] = 61; },
    f => { f.callee.jobs.collect.strategy.matrix.round.push('retry'); },
    f => { f.callee.jobs.collect.steps[1]['continue-on-error'] = true; },
    f => { f.callee.concurrency.group = f.caller.concurrency.group; },
    f => { f.callee.jobs.collect.steps[0].with.ref = 'main'; }];
  for (const mutate of mutations) {
    const f = platformFixture(); await linkReviewedPerformanceRun(...f.selection());
    mutate(f); f.refreshWorkflows(); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
  }
});
test('asynchronous API callbacks cannot rewrite the reviewed expectations snapshot', async () => {
  const f = platformFixture(), [bytes, digest] = f.selection();
  const api = async endpoint => {
    bytes.fill(0); f.value.source.commit = '9'.repeat(40);
    return f.api(endpoint);
  };
  const report = inspectPerformancePlatformLink(await linkReviewedPerformanceRun(bytes, digest, api, NOW));
  assert.equal(report.expected.source.commit, '1'.repeat(40));
});
test('actual caller and callee satisfy the source-bound four-job contract without accepting role overrides', () => {
  const bytes = path => readFileSync(new URL(`../${path}`, import.meta.url));
  for (const event of ['pull_request', 'workflow_dispatch'])
    assert.equal(validatePerformanceWorkflowPair(bytes(CALLER), bytes(CALLEE), event).performanceQualified, false);
  assert.throws(() => validatePerformanceWorkflowPair(bytes(CALLER), bytes(CALLEE), 'push'));
});
test('metadata command requires external digest and a distinct create-only output without offline authority flags', () => {
  const valid = ['--expectations', 'expected.json', '--expectations-sha256', 'a'.repeat(64), '--output', 'report.json'];
  assert.equal(platformArguments(valid).digest, 'a'.repeat(64));
  for (const args of [[], valid.slice(0, 4), [...valid, '--source-authenticated'],
    ['--expectations', 'same.json', '--expectations-sha256', 'a'.repeat(64), '--output', 'same.json'],
    ['--expectations', 'a.json', '--expectations-sha256', 'bad', '--output', 'b.json']])
    assert.throws(() => platformArguments(args));
});

test('run restarted or replaced during metadata reads rejects at the final live observation', async () => {
  for (const mutate of [run => { run.run_attempt = 2; run.status = 'in_progress'; run.conclusion = null; },
    run => { run.referenced_workflows = []; }, run => { run.check_suite_id++; }]) {
    const f = platformFixture(); await linkReviewedPerformanceRun(...f.selection());
    let reads = 0;
    const api = async endpoint => {
      if (endpoint === `${PREFIX}/actions/runs/100` && ++reads === 2) mutate(f.run);
      return f.api(endpoint);
    };
    const [bytes, digest] = f.selection();
    await assert.rejects(linkReviewedPerformanceRun(bytes, digest, api, NOW));
    assert.equal(reads, 2);
  }
});

test('closed PRs retain immutable same-run execution proof without a mutable PR association', async () => {
  const f = platformFixture(true); f.run.pull_requests = [];
  const key = await linkReviewedPerformanceRun(...f.selection());
  assert.equal(inspectPerformancePlatformLink(key).run.executionCommit, f.value.execution.commit);
  assert.deepEqual(f.run.pull_requests, [], 'verification must not invent platform PR records');
});
test('absent PR associations cannot waive the selected immutable PR ref or Git merge identities', async () => {
  for (const mutate of [f => { f.run.referenced_workflows = []; },
    f => { f.run.referenced_workflows[0].ref = 'refs/pull/999/merge'; },
    f => { f.run.referenced_workflows[0].ref = 'refs/heads/main'; },
    f => { f.run.referenced_workflows[0].sha = '9'.repeat(40); },
    f => { f.run.referenced_workflows[0].path = `other/zryna/${CALLEE}@${f.value.execution.commit}`; },
    f => { const row = platformFixture(true).run.pull_requests[0]; f.run.pull_requests = [row, { ...row, number: 999 }]; },
    f => { f.responses.get(`${PREFIX}/git/commits/${f.value.execution.commit}`).parents.reverse(); },
    f => { f.responses.get(`${PREFIX}/git/commits/${f.value.execution.commit}`).tree.sha = '9'.repeat(40); }]) {
    const f = platformFixture(true); f.run.pull_requests = [];
    await linkReviewedPerformanceRun(...f.selection());
    mutate(f); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
  }
});
test('PR association removal during verification preserves the same immutable execution proof', async () => {
  const f = platformFixture(true); let reads = 0;
  const api = async endpoint => {
    if (endpoint === `${PREFIX}/actions/runs/100` && ++reads === 2) f.run.pull_requests = [];
    return f.api(endpoint);
  };
  const [bytes, digest] = f.selection();
  await linkReviewedPerformanceRun(bytes, digest, api, NOW); assert.equal(reads, 2);
});
