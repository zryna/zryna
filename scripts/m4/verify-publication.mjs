import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { resolve, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { githubApi } from './hosted-api.mjs';
import { validateExpectations } from './hosted-expectations.mjs';
import { loadHostedRegistry, verifyM4Hosted } from './verify-hosted.mjs';
import { digest, parseCanonical } from './registry-files.mjs';
import { downloadPublicationArchive, retainPublicationArchive } from './retain-publication.mjs';
import { checkSupportFiles, canonicalSupportBytes } from './support.mjs';
import { loadPublicationWorkflowInventory, verifyCompletePublicationJobs } from './publication-workflows.mjs';

export function validatePublicationContext(expectations, environment, checkout) {
  validateExpectations(expectations);
  assert.equal(environment.GITHUB_ACTIONS, 'true', 'publication requires actual Actions context');
  assert.equal(environment.GITHUB_REPOSITORY, 'zryna/zryna');
  assert.equal(environment.GITHUB_EVENT_NAME, 'workflow_dispatch');
  assert.equal(environment.GITHUB_REF, 'refs/heads/main', 'official export requires actual main');
  assert.equal(expectations.repository, environment.GITHUB_REPOSITORY);
  assert.equal(expectations.source.ref, environment.GITHUB_REF);
  assert.equal(expectations.source.commit, environment.GITHUB_SHA);
  assert.deepEqual(checkout, expectations.source, 'exact current checkout commit/tree/ref');
  for (const identity of [expectations.workflowExecution, expectations.eventSource]) {
    assert.equal(identity.commit, expectations.source.commit, 'actual main event and workflow execution commit');
    assert.equal(identity.tree, expectations.source.tree, 'actual main event and workflow execution tree');
  }
  assert(/^[1-9][0-9]*$/u.test(environment.M4_CI_RUN_ID ?? '') && Number.isSafeInteger(Number(environment.M4_CI_RUN_ID)));
  assert(/^[1-9][0-9]*$/u.test(environment.M4_CI_ATTEMPT ?? '') && Number.isSafeInteger(Number(environment.M4_CI_ATTEMPT)));
  for (const key of ['GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']) {
    assert(/^[1-9][0-9]*$/u.test(environment[key] ?? '') && Number.isSafeInteger(Number(environment[key])));
  }
  for (const job of expectations.jobs) {
    assert.equal(job.runId, Number(environment.M4_CI_RUN_ID));
    assert.equal(job.attempt, Number(environment.M4_CI_ATTEMPT));
    assert.equal(job.headCommit, expectations.source.commit, 'PR synthetic/equal-tree revision cannot certify main');
    assert(['push', 'workflow_dispatch'].includes(job.event), 'main foundation CI only');
  }
}

export async function authenticatePublicationFoundation(expectations, api, root, readWorkflow) {
  const first = expectations.jobs[0];
  const prefix = `/repos/${expectations.repository}`;
  const [main, commit, run, jobs] = await Promise.all([
    api(`${prefix}/git/ref/heads/main`), api(`${prefix}/git/commits/${expectations.source.commit}`),
    api(`${prefix}/actions/runs/${first.runId}/attempts/${first.attempt}`),
    api(`${prefix}/actions/runs/${first.runId}/attempts/${first.attempt}/jobs?per_page=100`),
  ]);
  assert.equal(main.object.sha, expectations.source.commit, 'current main advanced; require fresh source evidence');
  assert.equal(commit.sha, expectations.source.commit);
  assert.equal(commit.tree.sha, expectations.source.tree);
  assert.equal(run.repository.full_name, expectations.repository);
  assert.equal(run.id, first.runId); assert.equal(run.run_attempt, first.attempt);
  assert.equal(run.path, '.github/workflows/ci.yml');
  assert.equal(run.head_branch, 'main'); assert.equal(run.head_sha, expectations.source.commit);
  assert.equal(run.event, first.event); assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  const inventory = await loadPublicationWorkflowInventory({ repository: expectations.repository, source: expectations.source, root, api, readWorkflow });
  verifyCompletePublicationJobs(jobs, inventory, { runId: first.runId, attempt: first.attempt, commit: expectations.source.commit });
  return { main, commit, run, jobs, inventory };
}

export async function verifyPublication({ expectations, root, retainedRoot, api = githubApi(),
  environment = process.env, download = downloadPublicationArchive }) {
  const git = args => execFileSync('git', args, { cwd: root, encoding: 'utf8', maxBuffer: 1024 * 1024 }).trim();
  validatePublicationContext(expectations, environment, {
    commit: git(['rev-parse', 'HEAD']), tree: git(['rev-parse', 'HEAD^{tree}']), ref: environment.GITHUB_REF,
  });
  assert.equal(git(['status', '--porcelain=v1', '--untracked-files=all']), '', 'clean exact-main source');
  const foundation = await authenticatePublicationFoundation(expectations, api, root);
  mkdirSync(retainedRoot, { recursive: false });
  writeFileSync(join(retainedRoot, 'expectations.json'), `${JSON.stringify(expectations, null, 2)}\n`, { flag: 'wx' });
  for (const job of expectations.jobs) {
    const archive = await download(expectations.repository, job.artifact);
    retainPublicationArchive(join(retainedRoot, `${job.lane}-${job.baseline}`), archive);
  }
  const registry = loadHostedRegistry(root);
  const hosted = await verifyM4Hosted({ expectations, root, retainedRoot, registry, api });
  assert.equal(hosted.executionComplete, true, 'complete authenticated execution matrix required before docs export');
  const support = checkSupportFiles(root);
  assert.equal(support.registrySha256, expectations.registrySha256);
  // Reauthenticate main and completed foundation after artifact retrieval and hostile-input checking.
  await authenticatePublicationFoundation(expectations, api, root);
  const result = { format: 'zryna.m4-docs-publication-gate.v1', source: expectations.source,
    registrySha256: expectations.registrySha256, matrixSha256: digest(canonicalSupportBytes(support)),
    ci: { runId: expectations.jobs[0].runId, attempt: expectations.jobs[0].attempt },
    documentationRun: { id: Number(environment.GITHUB_RUN_ID), attempt: Number(environment.GITHUB_RUN_ATTEMPT) },
    foundation, hosted, eligibilityScope: 'docs-export-only', eligible: hosted.executionComplete };
  writeFileSync(join(retainedRoot, 'publication-gate.json'), `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  return result;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const [flag, expectedSha256, outputFlag, output] = process.argv.slice(2);
    assert.equal(flag, '--expectations-sha256'); assert.equal(outputFlag, '--retained');
    assert.match(expectedSha256, /^[a-f0-9]{64}$/u); assert(output);
    const bytes = Buffer.from(process.env.M4_EXPECTATIONS_JSON ?? '', 'utf8');
    assert(bytes.length > 0 && bytes.length <= 60 * 1024, 'bounded externally reviewed canonical expectations input');
    assert.equal(digest(bytes), expectedSha256, 'externally supplied expectations SHA256');
    const root = resolve(import.meta.dirname, '../..');
    const result = await verifyPublication({ expectations: parseCanonical(bytes), root, retainedRoot: resolve(output) });
    process.stdout.write(`${JSON.stringify({ eligible: result.eligible, source: result.source,
      registrySha256: result.registrySha256, matrixSha256: result.matrixSha256 })}\n`);
  } catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
