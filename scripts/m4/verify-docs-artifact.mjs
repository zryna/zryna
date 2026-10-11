import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { githubApi } from './hosted-api.mjs';
import { zipEntries } from './hosted-archive.mjs';
import { validateExpectations, exactKeys } from './hosted-expectations.mjs';
import { digest, parseCanonical, readStableFile } from './registry-files.mjs';
import { loadHostedRegistry, verifyM4Hosted } from './verify-hosted.mjs';
import { authenticatePublicationFoundation } from './verify-publication.mjs';
import { downloadPublicationArchive, retainPublicationArchive } from './retain-publication.mjs';
import { checkSupportFiles, extractM4SupportFence, canonicalSupportBytes } from './support.mjs';
import { validateDocsBundle } from '../docs/bundle.mjs';

export async function authenticateDocsArtifact(expected, descriptor, name, api, job, now = Date.now()) {
  const artifact = await api(`/repos/zryna/zryna/actions/artifacts/${descriptor.id}`);
  assert.equal(artifact.id, descriptor.id); assert.equal(artifact.name, name);
  assert.equal(artifact.size_in_bytes, descriptor.bytes); assert.equal(artifact.digest, `sha256:${descriptor.sha256}`);
  assert.equal(artifact.workflow_run.id, expected.docsRunId); assert.equal(artifact.workflow_run.head_sha, expected.source.commit);
  assert.equal(artifact.expires_at, descriptor.expiresAt);
  assert.equal(artifact.expired, false); assert(Date.parse(descriptor.expiresAt) > now, 'fresh actual archive required');
  assert(Number.isFinite(Date.parse(artifact.created_at)) && Number.isFinite(Date.parse(job.started_at))
    && Number.isFinite(Date.parse(job.completed_at)) && Date.parse(artifact.created_at) >= Date.parse(job.started_at)
    && Date.parse(artifact.created_at) <= Date.parse(job.completed_at) && Date.parse(job.completed_at) <= now,
  'artifact must belong to the exact successful documentation attempt');
  return artifact;
}

export async function verifyDocsArtifact({ expected, root, output, api = githubApi(), download = downloadPublicationArchive }) {
  exactKeys(expected, ['format', 'source', 'registrySha256', 'matrixSha256', 'manifestSha256',
    'expectationsSha256', 'docsWorkflowSha256', 'docsRunId', 'docsAttempt', 'gateArtifact', 'docsArtifact'], 'docs expectations');
  assert.equal(expected.format, 'zryna.m4-docs-artifact-expectations.v1');
  for (const key of ['registrySha256', 'matrixSha256', 'manifestSha256', 'expectationsSha256', 'docsWorkflowSha256']) {
    assert.match(expected[key], /^[a-f0-9]{64}$/u);
  }
  assert(Number.isSafeInteger(expected.docsRunId) && expected.docsRunId > 0);
  assert(Number.isSafeInteger(expected.docsAttempt) && expected.docsAttempt > 0);
  exactKeys(expected.source, ['commit', 'tree', 'ref'], 'docs source');
  assert.match(expected.source.commit, /^[a-f0-9]{40}$/u); assert.match(expected.source.tree, /^[a-f0-9]{40}$/u);
  assert.equal(expected.source.ref, 'refs/heads/main');
  for (const artifact of [expected.gateArtifact, expected.docsArtifact]) {
    exactKeys(artifact, ['id', 'bytes', 'sha256', 'expiresAt'], 'docs artifact');
    assert(Number.isSafeInteger(artifact.id) && artifact.id > 0);
    assert(Number.isSafeInteger(artifact.bytes) && artifact.bytes > 0 && artifact.bytes <= 768 * 1024 * 1024);
    assert.match(artifact.sha256, /^[a-f0-9]{64}$/u); assert(Number.isFinite(Date.parse(artifact.expiresAt)));
  }
  const workflow = readStableFile(join(root, '.github/workflows/documentation.yml'), 1024 * 1024);
  assert.equal(digest(workflow), expected.docsWorkflowSha256, 'independently reviewed exact docs workflow');
  const run = await api(`/repos/zryna/zryna/actions/runs/${expected.docsRunId}/attempts/${expected.docsAttempt}`);
  assert.equal(run.id, expected.docsRunId); assert.equal(run.run_attempt, expected.docsAttempt);
  assert.equal(run.repository.full_name, 'zryna/zryna'); assert.equal(run.path, '.github/workflows/documentation.yml');
  assert.equal(run.event, 'workflow_dispatch'); assert.equal(run.head_branch, 'main');
  assert.equal(run.head_sha, expected.source.commit); assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  const jobs = await api(`/repos/zryna/zryna/actions/runs/${expected.docsRunId}/attempts/${expected.docsAttempt}/jobs?per_page=100`);
  assert.equal(jobs.total_count, 1); assert.equal(jobs.jobs.length, 1);
  const [job] = jobs.jobs;
  assert.equal(job.name, 'documentation'); assert.equal(job.run_id, expected.docsRunId);
  assert.equal(job.run_attempt, expected.docsAttempt); assert.equal(job.head_sha, expected.source.commit);
  assert.equal(job.status, 'completed'); assert.equal(job.conclusion, 'success');
  assert.match(job.check_run_url, /^https:\/\/api\.github\.com\/repos\/zryna\/zryna\/check-runs\/[1-9][0-9]*$/u);
  const check = await api(new URL(job.check_run_url).pathname);
  assert.equal(check.app.slug, 'github-actions'); assert.equal(check.head_sha, expected.source.commit);
  assert.equal(check.status, 'completed'); assert.equal(check.conclusion, 'success');
  assert.notEqual(expected.gateArtifact.id, expected.docsArtifact.id);
  const gateName = `zryna-docs-gate-${expected.source.commit}-${expected.docsRunId}-${expected.docsAttempt}`;
  const docsName = `zryna-docs-next-${expected.source.commit}-${expected.manifestSha256}`;
  await authenticateDocsArtifact(expected, expected.gateArtifact, gateName, api, job);
  await authenticateDocsArtifact(expected, expected.docsArtifact, docsName, api, job);
  const gateZip = await download('zryna/zryna', expected.gateArtifact);
  const contents = new Map();
  const gateEntries = zipEntries(gateZip, (entry, bytes) => {
    assert(['expectations.json', 'publication-gate.json'].includes(entry.path), 'exact metadata-only gate artifact');
    assert(bytes.length <= 32 * 1024 * 1024, 'gate metadata bound'); contents.set(entry.path, bytes);
  });
  assert.equal(gateEntries.length, 2); assert.equal(contents.size, 2);
  assert.equal(digest(contents.get('expectations.json')), expected.expectationsSha256, 'independent hosted expectations pin');
  const hostedExpectations = validateExpectations(parseCanonical(contents.get('expectations.json')));
  assert.deepEqual(hostedExpectations.source, expected.source);
  for (const identity of [hostedExpectations.workflowExecution, hostedExpectations.eventSource]) {
    assert.equal(identity.commit, expected.source.commit); assert.equal(identity.tree, expected.source.tree);
  } assert.equal(hostedExpectations.registrySha256, expected.registrySha256);
  const gate = parseCanonical(contents.get('publication-gate.json'));
  assert.equal(gate.format, 'zryna.m4-docs-publication-gate.v1'); assert.equal(gate.eligibilityScope, 'docs-export-only'); assert.deepEqual(gate.source, expected.source);
  assert.equal(gate.registrySha256, expected.registrySha256); assert.equal(gate.matrixSha256, expected.matrixSha256);
  assert.deepEqual(gate.documentationRun, { id: expected.docsRunId, attempt: expected.docsAttempt });
  assert.deepEqual(gate.ci, { runId: hostedExpectations.jobs[0].runId, attempt: hostedExpectations.jobs[0].attempt });
  assert.equal(gate.eligible, true);
  await authenticatePublicationFoundation(hostedExpectations, api, root);
  mkdirSync(output, { recursive: false });
  writeFileSync(join(output, 'gate.zip'), gateZip, { flag: 'wx' });
  for (const entry of hostedExpectations.jobs) {
    retainPublicationArchive(join(output, `${entry.lane}-${entry.baseline}`), await download('zryna/zryna', entry.artifact));
  }
  const registry = loadHostedRegistry(root);
  const hosted = await verifyM4Hosted({ expectations: hostedExpectations, root, retainedRoot: output, registry, api });
  assert.equal(hosted.executionComplete, true, 'complete authenticated execution matrix');
  const docsZip = await download('zryna/zryna', expected.docsArtifact);
  retainPublicationArchive(join(output, 'docs'), docsZip);
  const bundleRoot = join(output, 'docs', 'evidence');
  const bundle = await validateDocsBundle(bundleRoot, { expectedManifestSha256: expected.manifestSha256,
    expectedChannel: 'next', expectedSourceCommit: expected.source.commit, expectedSourceRef: 'refs/heads/main' });
  const support = checkSupportFiles(root);
  assert.equal(support.registrySha256, expected.registrySha256);
  assert.equal(digest(canonicalSupportBytes(support)), expected.matrixSha256);
  const page = new TextDecoder('utf-8', { fatal: true }).decode(readStableFile(join(bundleRoot, 'documents/reference/m4-support.md'), 2 * 1024 * 1024));
  assert(extractM4SupportFence(page).equals(canonicalSupportBytes(support)), 'exact registry matrix in authenticated Markdown docs archive');
  await authenticatePublicationFoundation(hostedExpectations, api, root);
  const result = { format: 'zryna.m4-docs-artifact-assessment.v1', expected, hosted, manifest: bundle.manifest, eligibilityScope: 'docs-artifact-only', eligible: hosted.executionComplete };
  writeFileSync(join(output, 'assessment.json'), `${JSON.stringify(result, null, 2)}\n`, { flag: 'wx' });
  return result;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  try {
    const args = process.argv.slice(2);
    assert(args.length === 6 && args[0] === '--expectations' && args[2] === '--expectations-sha256' && args[4] === '--output');
    assert.match(args[3], /^[a-f0-9]{64}$/u);
    const bytes = readStableFile(resolve(args[1]), 64 * 1024); assert.equal(digest(bytes), args[3], 'external docs artifact expectation pin');
    const result = await verifyDocsArtifact({ expected: parseCanonical(bytes), root: resolve(import.meta.dirname, '../..'), output: resolve(args[5]) });
    process.stdout.write(`${JSON.stringify({ eligible: result.eligible, source: result.expected.source, manifestSha256: result.expected.manifestSha256 })}\n`);
  } catch (error) { process.stderr.write(`${error.message}\n`); process.exitCode = 1; }
}
