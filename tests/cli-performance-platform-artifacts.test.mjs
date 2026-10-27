import assert from 'node:assert/strict';
import test from 'node:test';
import { linkReviewedPerformanceRun, inspectPerformancePlatformLink } from '../scripts/stability-gates/cli-performance-platform-link.mjs';
import { platformFixture, PREFIX } from './cli-performance-platform-fixture.mjs';

async function rejectMutation(mutate) {
  const f = platformFixture(); await linkReviewedPerformanceRun(...f.selection());
  mutate(f); await assert.rejects(linkReviewedPerformanceRun(...f.selection()));
}
test('missing duplicate extra stale or unsuccessful jobs cannot complete the selected matrix', async () => {
  for (const mutate of [f => { f.jobs.pop(); }, f => { f.jobs[1] = structuredClone(f.jobs[0]); },
    f => { f.responses.get(`${PREFIX}/actions/runs/100/attempts/1/jobs?per_page=100`).total_count = 5; },
    f => { f.jobs[0].name = f.jobs[1].name; }, f => { f.jobs[0].run_id++; }, f => { f.jobs[0].run_attempt++; },
    f => { f.jobs[0].head_sha = '9'.repeat(40); }, f => { f.jobs[0].status = 'queued'; },
    f => { f.jobs[0].conclusion = 'failure'; }, f => { f.jobs[0].check_run_url += '/wrong'; }]) await rejectMutation(mutate);
});
test('required collection calibration provider retention and source steps must run successfully on their owning hosts', async () => {
  for (const index of [0, 2]) for (const step of [0, 1, 2, 3, 4, 5, index === 0 ? 7 : 6])
    await rejectMutation(f => { f.jobs[index].steps[step].conclusion = 'skipped'; });
  await rejectMutation(f => { f.jobs[0].steps[0].conclusion = 'failure'; });
  await rejectMutation(f => { f.jobs[0].steps.push(structuredClone(f.jobs[0].steps[0])); });
  await rejectMutation(f => { f.jobs[0].steps.shift(); });
});
test('every selected check must be the same successful GitHub Actions check in the run suite', async () => {
  for (const mutate of [r => { r.id++; }, r => { r.name = 'other'; }, r => { r.head_sha = '9'.repeat(40); },
    r => { r.status = 'queued'; }, r => { r.conclusion = 'neutral'; }, r => { r.app.slug = 'other'; },
    r => { r.check_suite.id++; }]) await rejectMutation(f => mutate(f.responses.get(`${PREFIX}/check-runs/${f.value.jobs[0].checkRunId}`)));
});
test('primary and Linux provider archives retain distinct exact run repository source size digest and lifetime bindings', async () => {
  for (const i of [0, 1, 4, 5]) for (const mutate of [r => { r.id++; }, r => { r.name = 'other'; },
    r => { r.size_in_bytes++; }, r => { r.digest = 'sha256:' + '9'.repeat(64); }, r => { r.expired = true; },
    r => { r.workflow_run.id++; }, r => { r.workflow_run.repository_id++; }, r => { r.workflow_run.head_repository_id++; },
    r => { r.workflow_run.head_sha = '9'.repeat(40); }, r => { r.workflow_run.head_branch = 'main'; },
    r => { r.expires_at = '2026-10-01T00:00:00Z'; }]) await rejectMutation(f => mutate(f.artifacts[i]));
  await rejectMutation(f => { f.artifacts.pop(); });
  await rejectMutation(f => { f.artifacts[1] = structuredClone(f.artifacts[0]); });
  await rejectMutation(f => { f.responses.get(`${PREFIX}/actions/runs/100/artifacts?per_page=100`).total_count = 7; });
});
test('artifact creation must fall inside its selected job and all times must describe completed real dates', async () => {
  for (const mutate of [f => { f.artifacts[0].created_at = '2026-10-25T23:59:59Z'; },
    f => { f.artifacts[0].created_at = '2026-10-26T01:00:01Z'; },
    f => { f.jobs[0].started_at = '2026-10-26T01:00:01Z'; },
    f => { f.jobs[0].completed_at = '2027-01-01T00:00:00Z'; },
    f => { f.jobs[0].started_at = '2026-02-30T00:00:00Z'; }]) await rejectMutation(mutate);
  await rejectMutation(f => { f.artifacts[0].expires_at = f.value.jobs[0].primary.expiresAt = '2026-10-27T00:00:00Z'; });
});
test('API enumeration order does not select a preferred round and inclusive creation boundaries remain valid', async () => {
  const f = platformFixture();
  f.jobs.reverse(); f.artifacts.reverse();
  for (const row of f.artifacts) row.created_at = '2026-10-26T00:00:00Z';
  const report = inspectPerformancePlatformLink(await linkReviewedPerformanceRun(...f.selection()));
  assert.equal(report.jobs.length, 4);
  for (const row of f.artifacts) row.created_at = '2026-10-26T01:00:00Z';
  await linkReviewedPerformanceRun(...f.selection());
});
