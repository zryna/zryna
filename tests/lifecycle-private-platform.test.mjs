import assert from 'node:assert/strict';
import test from 'node:test';
import { verifyCaptureMetadata } from '../scripts/stability-gates/lifecycle-private-artifact.mjs';

test('capture platform identity rejects substituted runs, steps and artifact attachments', () => {
  const head = 'a'.repeat(40), repository = { id: 1350402316, full_name: 'zryna/zryna', fork: false };
  const expected = { run: 1, attempt: 2, workflowId: 3, jobId: 4, checkRunId: 5, id: 6, bytes: 7,
    sha256: 'b'.repeat(64), branch: 'private', expiresAt: '2099-01-01T00:00:00Z' };
  const run = { id: 1, run_attempt: 2, workflow_id: 3, repository, head_repository: repository,
    path: '.github/workflows/native-host-facts.yml', event: 'workflow_dispatch', head_sha: head,
    head_branch: 'private', status: 'completed', conclusion: 'success', check_suite_id: 8 };
  const job = { id: 4, name: 'lifecycle', run_id: 1, run_attempt: 2, head_sha: head,
    status: 'completed', conclusion: 'success', started_at: '2025-01-01T00:00:00Z', completed_at: '2025-01-01T00:02:00Z',
    check_run_url: 'https://api.github.com/repos/zryna/zryna/check-runs/5',
    steps: [{ name: 'Capture immutable lifecycle inputs', status: 'completed', conclusion: 'success' }] };
  const jobs = { total_count: 4, jobs: [job, ...[9, 10, 11].map(id => ({ id }))] };
  const artifact = { id: 6, name: `lifecycle-capture-${head}-1-2`, size_in_bytes: 7,
    digest: 'sha256:' + expected.sha256, expired: false, expires_at: expected.expiresAt, created_at: '2025-01-01T00:01:00Z',
    workflow_run: { id: 1, repository_id: repository.id, head_repository_id: repository.id, head_sha: head, head_branch: 'private' } };
  const check = { id: 5, app: { slug: 'github-actions' }, head_sha: head, check_suite: { id: 8 }, status: 'completed', conclusion: 'success' };
  const original = [expected, run, jobs, artifact, check, head];
  verifyCaptureMetadata(...original); // Synthetic metadata consistency, no hosted authentication claim.
  for (const mutate of [rows => { rows[1].event = 'pull_request'; }, rows => { rows[1].head_sha = 'c'.repeat(40); },
    rows => { rows[1].repository.id++; }, rows => { rows[2].jobs[0].steps[0].conclusion = 'skipped'; },
    rows => { rows[3].workflow_run.id++; }, rows => { rows[3].digest = 'sha256:' + '0'.repeat(64); },
    rows => { rows[3].expired = true; }, rows => { rows[4].app.slug = 'other'; }]) {
    const rows = structuredClone(original); mutate(rows); assert.throws(() => verifyCaptureMetadata(...rows));
  }
});

