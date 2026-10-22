import { requireEqual } from './hosted-expectations.mjs';

export async function authenticateAggregate(expectations, api) {
  const prefix = `/repos/${expectations.repository}`;
  const first = expectations.jobs[0];
  const wanted = expectations.aggregate;
  const [job, check] = await Promise.all([
    api(`${prefix}/actions/jobs/${wanted.jobId}`), api(`${prefix}/check-runs/${wanted.checkRunId}`),
  ]);
  requireEqual(job.id, wanted.jobId, 'aggregate job id');
  requireEqual(job.run_id, first.runId, 'aggregate run');
  requireEqual(job.run_attempt, first.attempt, 'aggregate attempt');
  requireEqual(job.head_sha, first.headCommit, 'aggregate head');
  requireEqual(job.name, 'm4', 'aggregate name');
  requireEqual(job.status, 'completed', 'aggregate completion');
  requireEqual(job.conclusion, 'success', 'aggregate outcome');
  requireEqual(job.check_run_url, `https://api.github.com${prefix}/check-runs/${wanted.checkRunId}`, 'aggregate check link');
  requireEqual(check.id, wanted.checkRunId, 'aggregate check id');
  requireEqual(check.name, 'm4', 'aggregate check name');
  requireEqual(check.app?.slug, 'github-actions', 'aggregate check app');
  requireEqual(check.head_sha, first.headCommit, 'aggregate check head');
  requireEqual(check.status, 'completed', 'aggregate check completion');
  requireEqual(check.conclusion, 'success', 'aggregate check outcome');
  requireEqual(check.html_url, wanted.checkUrl, 'aggregate check URL');
}
