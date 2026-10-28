import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, MAX_DOCUMENT, sha256 } from '../scripts/stability-gates/input.mjs';
import { NUMERIC_LABELS, NUMERIC_WORKFLOWS, NUMERIC_WORKFLOW_PINS,
  parseNumericCampaignPlan } from '../scripts/stability-gates/cli-performance-campaign-plan-v1.mjs';
import { inspectNumericCampaignPhase, linkNumericCampaignPhase } from '../scripts/stability-gates/cli-performance-campaign-link-v1.mjs';
import { campaignFixture, NOW, PREFIX } from './cli-performance-campaign-fixture.mjs';
const run = f => linkNumericCampaignPhase(...f.arguments());
async function rejects(mutate, phase = 'baselines') {
  const f = campaignFixture(phase); mutate(f); await assert.rejects(run(f));
}
test('empty, first-extra and non-Buffer plans reject before copying, hashing or API access', async () => {
  for (const bytes of [Buffer.alloc(0), Buffer.alloc(MAX_DOCUMENT + 1), new Uint8Array(1), {}]) {
    let calls = 0;
    await assert.rejects(linkNumericCampaignPhase(bytes, 'a'.repeat(64), async () => { calls++; }, NOW));
    assert.equal(calls, 0);
    assert.throws(() => parseNumericCampaignPlan(bytes, 'a'.repeat(64)));
  }
});
test('three separate terminal phases preserve exact 4/1/2 jobs and 6/1/3 artifacts without admission', async () => {
  for (const [phase, jobs, artifacts] of [['baselines', 4, 6], ['anchor', 1, 1], ['heldout', 2, 3]]) {
    const f = campaignFixture(phase), key = await run(f), report = inspectNumericCampaignPhase(key);
    assert.equal(report.jobs.length, jobs); assert.equal(report.jobs.flatMap(job => job.artifacts).length, artifacts);
    assert(report.jobs.every(job => job.virtualization.value === 'github-actions-hosted-vm'));
    for (const name of ['sourceAuthenticated', 'proofQualified', 'performanceQualified', 'hostAdmitted', 'baselineAdmitted'])
      assert.equal(report[name], false);
    assert.equal(report.thresholdAuthority, null); assert.equal(report.closureStatus, 'blocked');
    for (const forged of [{}, { ...key }, report, structuredClone(report)]) assert.throws(() => inspectNumericCampaignPhase(forged));
    report.jobs[0].artifacts[0].sha256 = 'f'.repeat(64);
    assert.notEqual(inspectNumericCampaignPhase(key).jobs[0].artifacts[0].sha256, 'f'.repeat(64));
  }
});
test('caller-reviewed digest refresh cannot change frozen policy, workflow commands or topology', () => {
  const f = campaignFixture(), [bytes, digest] = f.arguments();
  assert.deepEqual(parseNumericCampaignPlan(bytes, digest), f.plan);
  for (const mutate of [p => { p.phase = 'retry'; }, p => { p.runId = '12345'; },
    p => { p.policySha256 = 'f'.repeat(64); }, p => { p.workflows[3].sha256 = 'f'.repeat(64); },
    p => { p.workflows[0].bytes++; }, p => { p.workflows.reverse(); }, p => { p.callback = true; }]) {
    const p = structuredClone(f.plan); mutate(p); const changed = Buffer.from(canonical(p));
    assert.throws(() => parseNumericCampaignPlan(changed, sha256(changed)));
  }
  assert.throws(() => { NUMERIC_WORKFLOWS.push('override'); });
  assert.throws(() => { NUMERIC_WORKFLOW_PINS[0].sha256 = 'f'.repeat(64); });
  assert.throws(() => { NUMERIC_LABELS['linux-x86_64'] = 'self-hosted'; });
});
test('failed, incomplete, retried, wrong-source, fork and mismatched reusable attachment runs reject', async () => {
  for (const mutate of [f => { f.run.run_attempt = 2; }, f => { f.run.status = 'in_progress'; },
    f => { f.run.conclusion = 'failure'; }, f => { f.run.event = 'pull_request'; },
    f => { f.run.head_sha = 'f'.repeat(40); }, f => { f.run.head_repository.id++; },
    f => { f.run.referenced_workflows = []; }, f => { f.run.referenced_workflows[0].ref = 'refs/heads/main'; }]) await rejects(mutate);
  await rejects(f => { f.run.referenced_workflows.push({}); }, 'anchor');
});
test('all four original workflow contents require actual Git blob, byte count and SHA256', async () => {
  for (const path of NUMERIC_WORKFLOWS) for (const mutate of [r => { r.size++; }, r => { r.sha = 'f'.repeat(40); },
    r => { r.content += 'AAAA'; }, r => { r.path = 'other.yml'; }])
    await rejects(f => mutate(f.responses.get(`${PREFIX}/contents/${path}?ref=${f.plan.source.commit}`)));
});
test('fresh dispatches cannot become replacement slots or be selected from a larger campaign history', async () => {
  await rejects(f => { f.history.total_count = 2; });
  await rejects(f => { f.history.workflow_runs.push({ ...f.run, id: 999 }); });
  await rejects(f => { f.history.workflow_runs = [{ ...f.run, id: 999 }]; });
  const f = campaignFixture();
  f.history.total_count = 2;
  f.history.workflow_runs.push({ ...f.run, id: 999, head_branch: 'other-ref-same-source', conclusion: 'failure' });
  await assert.rejects(run(f));
  assert(f.calls.some(path => path.includes('/actions/workflows/') && !path.includes('branch=')),
    'same-source history must cover branch aliases before binding the selected exact ref');
});
test('missing, duplicate, extra or failed jobs and skipped mandatory measurement steps cannot pass', async () => {
  for (const mutate of [f => { f.jobResponse.total_count++; }, f => { f.jobs.pop(); },
    f => { f.jobs[1].id = f.jobs[0].id; }, f => { f.jobs[1].name = f.jobs[0].name; },
    f => { f.jobs[0].conclusion = 'failure'; }, f => { f.jobs[0].run_attempt = 2; },
    f => { f.jobs[0].steps.find(s => s.name === 'Assemble observed operator inputs and collect one fresh round').conclusion = 'skipped'; },
    f => { f.jobs[0].steps[0].number = f.jobs[0].steps[1].number; },
    f => { f.responses.get(`${PREFIX}/check-runs/100`).app.slug = 'other'; }]) await rejects(mutate);
  await rejects(f => { f.jobs[0].steps.find(s => s.name.startsWith('Verify completed numeric anchor')).conclusion = 'skipped'; }, 'heldout');
});
test('hosted service provenance, supported image and serial baseline job chronology are required', async () => {
  for (const mutate of [f => { f.jobs[0].runner_group_id = 1; }, f => { f.jobs[0].runner_group_name = 'private'; },
    f => { f.jobs[0].runner_name = 'self-hosted'; }, f => { f.jobs[0].labels = ['ubuntu-latest']; },
    f => { f.jobs[1].started_at = f.jobs[0].completed_at; },
    f => { f.jobs[0].completed_at = '2030-01-01T99:00:00Z'; }]) await rejects(mutate);
});
test('extra, expired, wrong-origin and unbounded artifacts reject before any original-byte replay', async () => {
  for (const mutate of [f => { f.artifactResponse.total_count++; }, f => { f.artifacts.pop(); },
    f => { f.artifacts[0].digest = 'sha256:invalid'; }, f => { f.artifacts[0].workflow_run.head_sha = 'f'.repeat(40); },
    f => { f.artifacts[0].expired = true; }, f => { f.artifacts[0].expires_at = '2029-01-01T00:00:00Z'; },
    f => { f.artifacts[0].created_at = '2029-01-01T00:00:00Z'; }, f => { f.artifacts[1].size_in_bytes = 1048577; },
    f => { f.artifacts[1].name = f.artifacts[0].name; }]) await rejects(mutate);
  await rejects(f => { f.artifacts[0].size_in_bytes = 1048577; }, 'anchor');
});
test('reviewed input is snapshotted before asynchronous API access and terminal run is rechecked', async () => {
  const f = campaignFixture(), [bytes, digest] = f.arguments();
  const api = async path => { bytes.fill(0); return f.api(path); };
  assert.equal(inspectNumericCampaignPhase(await linkNumericCampaignPhase(bytes, digest, api, NOW)).jobs.length, 4);
  const g = campaignFixture(); let calls = 0;
  const changed = async path => { const value = await g.api(path);
    if (path === `${PREFIX}/actions/runs/${g.run.id}` && ++calls === 2) value.run_attempt = 2;
    return value; };
  const [raw, hash] = g.arguments(); await assert.rejects(linkNumericCampaignPhase(raw, hash, changed, NOW));
});
