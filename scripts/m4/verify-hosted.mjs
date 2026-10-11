import { spawnSync } from 'node:child_process';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { githubApi } from './hosted-api.mjs';
import { verifyRetainedArchive } from './hosted-archive.mjs';
import { requireEqual, validateExpectations } from './hosted-expectations.mjs';
import { authenticateWorkflow } from './hosted-workflow.mjs';
import { authenticateAggregate } from './hosted-aggregate.mjs';
import { digest, parseCanonical, readStableFile } from './registry-files.mjs';
import { loadM4Evidence, composeM4Evidence } from './evidence.mjs';

function git(root, args) {
  const result = spawnSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 1024 * 1024,
    timeout: 30_000, windowsHide: true, shell: false });
  if (result.error || result.signal || result.status !== 0) throw new Error('M4 hosted: source observation failed');
  return result.stdout.trim();
}
function source(root, expected) {
  requireEqual(git(root, ['rev-parse', 'HEAD']), expected.commit, 'actual checkout commit');
  requireEqual(git(root, ['rev-parse', 'HEAD^{tree}']), expected.tree, 'actual checkout tree');
  requireEqual(git(root, ['status', '--porcelain=v1', '--untracked-files=all']), '', 'checkout cleanliness');
}
export async function authenticateJob({ expectations, job, api, now }) {
  const prefix = `/repos/${expectations.repository}`;
  const [run, observed, check, artifact] = await Promise.all([
    api(`${prefix}/actions/runs/${job.runId}/attempts/${job.attempt}`),
    api(`${prefix}/actions/jobs/${job.jobId}`),
    api(`${prefix}/check-runs/${job.checkRunId}`),
    api(`${prefix}/actions/artifacts/${job.artifact.id}`),
  ]);
  requireEqual(run.id, job.runId, 'run id');
  requireEqual(run.run_attempt, job.attempt, 'run attempt');
  requireEqual(run.repository?.full_name, expectations.repository, 'run repository');
  requireEqual(run.path, job.runWorkflowPath, 'run workflow');
  requireEqual(run.head_sha, job.headCommit, 'run head');
  requireEqual(run.head_sha, expectations.source.commit, 'exact API candidate');
  if (typeof run.head_branch !== 'string' || !run.head_branch.length) throw new Error('M4 hosted: run branch absent');
  requireEqual(`refs/heads/${run.head_branch}`, expectations.source.ref, 'run source ref');
  requireEqual(run.event, job.event, 'run event');
  if (!(run.status === 'completed' && run.conclusion === 'success')) {
    throw new Error('M4 hosted: required run has not succeeded');
  }
  requireEqual(observed.id, job.jobId, 'job id');
  requireEqual(observed.run_id, job.runId, 'job run');
  requireEqual(observed.run_attempt, job.attempt, 'job attempt');
  requireEqual(observed.name, job.jobName, 'job name');
  requireEqual(observed.head_sha, job.headCommit, 'job head');
  requireEqual(observed.status, 'completed', 'job completion');
  requireEqual(observed.conclusion, 'success', 'job outcome');
  requireEqual(observed.check_run_url, `https://api.github.com${prefix}/check-runs/${job.checkRunId}`, 'job check link');
  requireEqual(check.id, job.checkRunId, 'check id');
  requireEqual(check.app?.slug, 'github-actions', 'check app');
  requireEqual(check.head_sha, job.headCommit, 'check head');
  requireEqual(check.status, 'completed', 'check completion');
  requireEqual(check.conclusion, 'success', 'check outcome');
  requireEqual(check.html_url, job.checkUrl, 'check URL');
  requireEqual(artifact.id, job.artifact.id, 'artifact id');
  requireEqual(artifact.name, job.artifact.name, 'artifact name');
  requireEqual(artifact.size_in_bytes, job.artifact.bytes, 'artifact bytes');
  requireEqual(artifact.digest, `sha256:${job.artifact.sha256}`, 'artifact digest');
  requireEqual(artifact.workflow_run?.id, job.runId, 'artifact run');
  requireEqual(artifact.workflow_run?.head_sha, job.headCommit, 'artifact head');
  requireEqual(artifact.expires_at, job.artifact.expiresAt, 'artifact expiry');
  if (artifact.expired !== false || Date.parse(artifact.expires_at) <= now) throw new Error('M4 hosted: artifact expired');
  const start = Date.parse(observed.started_at);
  const completed = Date.parse(observed.completed_at);
  const created = Date.parse(artifact.created_at);
  if (![start, completed, created].every(Number.isFinite) || completed < start
      || created < start || created > completed || completed > now) throw new Error('M4 hosted: artifact/job chronology differs');
  return { run, job: observed, check, artifact };
}

export async function verifyM4Hosted({ expectations, retainedRoot, root, registry,
  api = githubApi(), now = Date.now() }) {
  if (!Number.isFinite(now)) throw new Error('M4 hosted: invalid verification time');
  validateExpectations(expectations);
  source(root, expectations.source);
  const prefix = `/repos/${expectations.repository}`;
  const first = expectations.jobs[0];
  const run = await api(`${prefix}/actions/runs/${first.runId}/attempts/${first.attempt}`);
  await authenticateWorkflow(expectations, api, run);
  await authenticateAggregate(expectations, api);
  for (const descriptor of expectations.workflowSources) {
    const bytes = readStableFile(join(root, descriptor.path), 1024 * 1024);
    requireEqual(bytes.length, descriptor.bytes, 'workflow bytes');
    requireEqual(digest(bytes), descriptor.sha256, 'workflow source digest');
  }
  const registryBytes = readStableFile(join(root, 'tests/m4-conformance-v1.json'), 8 * 1024 * 1024);
  requireEqual(digest(registryBytes), expectations.registrySha256, 'registry digest');
  requireEqual(parseCanonical(registryBytes), registry, 'registry value');
  const jobs = [];
  for (const job of expectations.jobs) {
    await authenticateJob({ expectations, job, api, now });
    const directory = join(retainedRoot, `${job.lane}-${job.baseline}`);
    verifyRetainedArchive(join(directory, 'artifact.zip'), directory, job.artifact);
    const workflow = { repository: expectations.repository, runId: job.runId, attempt: job.attempt,
      jobName: job.jobName, jobKey: job.jobKey, workflowPath: job.workflowPath,
      workflowSha256: job.workflowSha256, event: job.event, headCommit: expectations.eventSource.commit,
      executionCommit: expectations.workflowExecution.commit,
      checkoutCommit: expectations.source.commit };
    const assessment = await loadM4Evidence(join(directory, 'evidence', 'receipt.json'), {
      root: join(directory, 'evidence'), sourceRoot: root, registry,
      expected: { source: expectations.source,
        registrySha256: expectations.registrySha256, lane: job.lane, host: job.host,
        workflow, tools: job.tools, materials: job.materials, evidenceDirectory: job.evidenceDirectory },
    });
    if (assessment.executionAuthenticated !== true) throw new Error(`M4 hosted: ${job.lane}/${job.baseline} execution provenance differs`);
    jobs.push({ lane: job.lane, baseline: job.baseline, jobId: job.jobId, assessment });
  }
  source(root, expectations.source);
  const composition = composeM4Evidence(registry, jobs.map(job => job.assessment));
  if (composition.executionComplete !== true) throw new Error('M4 hosted: six-lane execution matrix is incomplete');
  return { format: 'zryna.m4-hosted-assessment.v1', source: expectations.source,
    registrySha256: expectations.registrySha256, jobs, composition,
    executionComplete: composition.executionComplete, eligible: composition.eligible, blockers: composition.blockers };
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    if (args.length !== 6 || args[0] !== '--expectations' || args[2] !== '--expectations-sha256'
        || args[4] !== '--retained' || !/^[a-f0-9]{64}$/.test(args[3])) {
      throw new Error('usage: verify-hosted.mjs --expectations PATH --expectations-sha256 SHA256 --retained DIRECTORY');
    }
    const bytes = readStableFile(resolve(args[1]), 8 * 1024 * 1024);
    requireEqual(digest(bytes), args[3], 'externally supplied expectations digest');
    const root = resolve(import.meta.dirname, '../..');
    const registry = parseCanonical(readStableFile(join(root, 'tests/m4-conformance-v1.json'), 8 * 1024 * 1024));
    const result = await verifyM4Hosted({ expectations: parseCanonical(bytes), retainedRoot: resolve(args[5]), root, registry });
    process.stdout.write(`${JSON.stringify(result, null, 2)}\n`);
  } catch (error) { console.error(error.message); process.exitCode = 1; }
}
