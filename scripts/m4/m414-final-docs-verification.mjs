// Review-only final verifier template for zryna/zryna#414. No defaults for external documentation pins.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { dirname, isAbsolute, join, resolve, sep } from 'node:path';
import { existsSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

const PIN = Object.freeze({ repository: 'zryna/zryna',
  commit: '01c81f4087982b2f1c99c85f9a7c0360dbeb0e28',
  tree: 'd5544b2df1fd19fd23bb65887a5f3cbec1b033d7',
  parents: ['236d2a7126dd0a9722cd07974843b84fbc48f76c'], ref: 'refs/heads/main',
  ciRunId: 39737726701, ciAttempt: 1, jobs: 42, documents: 61, bundleFiles: 63 });
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
const source = { commit: PIN.commit, tree: PIN.tree, ref: PIN.ref };
const hashInput = name => {
  const value = process.env[name]; assert.match(value ?? '', /^[a-f0-9]{64}$/u, `required external ${name}`); return value;
};
const integerInput = name => {
  const value = process.env[name]; assert.match(value ?? '', /^[1-9][0-9]*$/u, `required external ${name}`);
  const integer = Number(value); assert(Number.isSafeInteger(integer)); return integer;
};
// These must be root-reviewed constants in the later operator workflow, never inferred from downloaded output.
const trusted = { expectationsSha256: hashInput('M414_HOSTED_EXPECTATIONS_SHA256'),
  manifestSha256: hashInput('M414_DOCS_MANIFEST_SHA256'), registrySha256: hashInput('M414_REGISTRY_SHA256'),
  matrixSha256: hashInput('M414_MATRIX_SHA256'), docsWorkflowSha256: hashInput('M414_DOCS_WORKFLOW_SHA256'),
  docsRunId: integerInput('M414_DOCS_RUN_ID'), docsAttempt: integerInput('M414_DOCS_ATTEMPT') };
const encoded = process.env.M414_HOSTED_EXPECTATIONS_BASE64;
assert(typeof encoded === 'string' && encoded.length > 0 && encoded.length <= 80 * 1024);
assert(/^[A-Za-z0-9+/]+={0,2}$/u.test(encoded), 'canonical externally reviewed base64 required');
const trustedHostedBytes = Buffer.from(encoded, 'base64');
assert.equal(trustedHostedBytes.toString('base64'), encoded);
assert(trustedHostedBytes.length > 0 && trustedHostedBytes.length <= 60 * 1024);
assert.equal(sha256(trustedHostedBytes), trusted.expectationsSha256, 'external reviewed hosted expectations hash');
assert.notEqual(trusted.docsRunId, PIN.ciRunId);
const [sourceFlag, sourceArg, outputFlag, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 6); assert.equal(sourceFlag, '--source-root'); assert.equal(outputFlag, '--output');
assert(isAbsolute(sourceArg) && isAbsolute(outputArg));
assert.equal(process.env.GITHUB_ACTIONS, 'true'); assert.equal(process.env.GITHUB_REPOSITORY, PIN.repository);
assert.equal(process.versions.node, '22.22.1'); assert.equal(process.versions.zlib, '1.3.1-e00f703');
assert.equal(process.arch, 'x64'); assert.equal(process.platform, 'linux');
assert(process.env.GITHUB_TOKEN, 'actual read-scoped Actions token required');
for (const name of ['GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']) integerInput(name);
assert.notEqual(Number(process.env.GITHUB_RUN_ID), trusted.docsRunId);
const root = realpathSync(sourceArg); const output = resolve(outputArg);
const runnerTemp = realpathSync(process.env.RUNNER_TEMP);
assert(output.startsWith(`${runnerTemp}${sep}`) && !output.startsWith(`${root}${sep}`));
assert(!resolve(process.argv[1]).startsWith(`${root}${sep}`), 'coordinator must be staged outside exact source');
assert(!existsSync(output), 'exclusive output required');
const git = args => execFileSync('git', args, { cwd: root, timeout: 30_000, maxBuffer: 1024 * 1024,
  encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const sourceObservations = [];
function cleanSource(phase) {
  const observed = { phase, observedAt: new Date().toISOString(), commit: git(['rev-parse', 'HEAD']),
    tree: git(['rev-parse', 'HEAD^{tree}']), parents: git(['show', '-s', '--format=%P', 'HEAD']).split(' '),
    localBranch: git(['symbolic-ref', '-q', 'HEAD']),
    status: git(['status', '--porcelain=v1', '--untracked-files=all']) };
  assert.equal(observed.commit, PIN.commit); assert.equal(observed.tree, PIN.tree);
  assert.deepEqual(observed.parents, PIN.parents); assert.equal(observed.localBranch, PIN.ref);
  assert.equal(observed.status, '', 'actual source remains completely clean'); sourceObservations.push(observed);
}
cleanSource('before-imports');
mkdirSync(output, { mode: 0o700 });
const review = join(output, 'review'); mkdirSync(review, { mode: 0o700 });
mkdirSync(join(review, 'metadata'), { mode: 0o700 });
const write = (path, bytes) => writeFileSync(path, bytes, { flag: 'wx', mode: 0o600 });
const load = path => import(pathToFileURL(join(root, path)).href);
const { githubApi } = await load('scripts/m4/hosted-api.mjs');
const { validateExpectations } = await load('scripts/m4/hosted-expectations.mjs');
const { digest, parseCanonical, readStableFile } = await load('scripts/m4/registry-files.mjs');
const { downloadPublicationArchive } = await load('scripts/m4/retain-publication.mjs');
const { authenticatePublicationFoundation } = await load('scripts/m4/verify-publication.mjs');
const { authenticateDocsArtifact, verifyDocsArtifact } = await load('scripts/m4/verify-docs-artifact.mjs');
const { checkSupportFiles, canonicalSupportBytes } = await load('scripts/m4/support.mjs');
const { validateDocsBundle } = await load('scripts/docs/bundle.mjs');
const { scanFiles } = await load('scripts/docs/bundle-filesystem.mjs');
const hostedExpectations = validateExpectations(parseCanonical(trustedHostedBytes));
assert.deepEqual(canonical(hostedExpectations), trustedHostedBytes);
assert.equal(hostedExpectations.repository, PIN.repository); assert.deepEqual(hostedExpectations.source, source);
assert.equal(hostedExpectations.registrySha256, trusted.registrySha256);
for (const identity of [hostedExpectations.workflowExecution, hostedExpectations.eventSource]) {
  assert.deepEqual(identity, { commit: PIN.commit, tree: PIN.tree, parents: PIN.parents });
}
for (const job of hostedExpectations.jobs) {
  assert.equal(job.runId, PIN.ciRunId); assert.equal(job.attempt, PIN.ciAttempt);
  assert.equal(job.headCommit, PIN.commit); assert.equal(job.event, 'workflow_dispatch');
}
assert.equal(digest(readStableFile(join(root, '.github/workflows/documentation.yml'), 1024 * 1024)), trusted.docsWorkflowSha256);
const support = checkSupportFiles(root); assert.equal(support.registrySha256, trusted.registrySha256);
assert.equal(digest(canonicalSupportBytes(support)), trusted.matrixSha256);
write(join(review, 'root-trusted-hosted-expectations.json'), trustedHostedBytes);
write(join(review, 'root-trusted-inputs.json'), canonical({ format: 'zryna.m414-final-verifier-trusted-inputs.v1',
  source, ci: { runId: PIN.ciRunId, attempt: PIN.ciAttempt }, ...trusted,
  origin: 'externally reviewed constants supplied by root in actual verifier operator workflow',
  manifestTrust: 'external reviewed independent export manifest hash; never derived from official bundle' }));
const apiMetadataLimits = Object.freeze({ maximumObservations: 200,
  maximumObservationBytes: 8 * 1024 * 1024, maximumAggregateBytes: 64 * 1024 * 1024 });
const observations = [];
let metadataBytes = 0;
const originalApi = githubApi();
async function api(endpoint) {
  assert(endpoint.startsWith(`/repos/${PIN.repository}/`));
  assert(observations.length < apiMetadataLimits.maximumObservations);
  const observedAt = new Date().toISOString(); const value = await originalApi(endpoint);
  const bytes = canonical(value); assert(bytes.length <= apiMetadataLimits.maximumObservationBytes);
  metadataBytes += bytes.length;
  assert(metadataBytes <= apiMetadataLimits.maximumAggregateBytes, 'aggregate API metadata budget before exclusive write');
  const file = `${String(observations.length + 1).padStart(3, '0')}-${sha256(Buffer.from(endpoint)).slice(0, 16)}.json`;
  write(join(review, 'metadata', file), bytes);
  observations.push({ endpoint, method: 'GET', observedAt, file: `metadata/${file}`,
    bytes: bytes.length, sha256: digest(bytes), representation: 'parsed API value serialized as JSON with final LF' });
  return value;
}
const prefix = `/repos/${PIN.repository}`;
const initialFoundation = await authenticatePublicationFoundation(hostedExpectations, api, root);
assert.equal(initialFoundation.jobs.total_count, PIN.jobs); assert.equal(initialFoundation.inventory.jobs.length, PIN.jobs);
assert.deepEqual(initialFoundation.commit.parents.map(parent => parent.sha), PIN.parents);
const gateName = `zryna-docs-gate-${PIN.commit}-${trusted.docsRunId}-${trusted.docsAttempt}`;
const docsName = `zryna-docs-next-${PIN.commit}-${trusted.manifestSha256}`;
const listed = await api(`${prefix}/actions/runs/${trusted.docsRunId}/artifacts?per_page=100`);
assert(Array.isArray(listed.artifacts) && listed.total_count === listed.artifacts.length && listed.total_count <= 100);
const selectArtifact = async name => {
  const matches = listed.artifacts.filter(item => item.name === name); assert.equal(matches.length, 1);
  const actual = await api(`${prefix}/actions/artifacts/${matches[0].id}`);
  assert.deepEqual(actual, matches[0]); assert.equal(actual.name, name);
  assert.equal(actual.workflow_run.id, trusted.docsRunId); assert.equal(actual.workflow_run.head_sha, PIN.commit);
  assert.equal(actual.expired, false); assert(Date.parse(actual.expires_at) > Date.now());
  assert(Number.isSafeInteger(actual.id) && actual.id > 0);
  assert(Number.isSafeInteger(actual.size_in_bytes) && actual.size_in_bytes > 0
    && actual.size_in_bytes <= 64 * 1024 * 1024, 'bounded official metadata/docs archive');
  assert.match(actual.digest, /^sha256:[a-f0-9]{64}$/u);
  return { id: actual.id, bytes: actual.size_in_bytes, sha256: actual.digest.slice(7), expiresAt: actual.expires_at };
};
const gateArtifact = await selectArtifact(gateName); const docsArtifact = await selectArtifact(docsName);
const expected = { format: 'zryna.m4-docs-artifact-expectations.v1', source,
  registrySha256: trusted.registrySha256, matrixSha256: trusted.matrixSha256,
  manifestSha256: trusted.manifestSha256, expectationsSha256: trusted.expectationsSha256,
  docsWorkflowSha256: trusted.docsWorkflowSha256, docsRunId: trusted.docsRunId,
  docsAttempt: trusted.docsAttempt, gateArtifact, docsArtifact };
write(join(review, 'docs-artifact-expectations.json'), canonical(expected));
write(join(review, 'docs-artifact-expectations-provenance.json'), canonical({
  format: 'zryna.m414-docs-expectations-provenance.v1', externallyTrustedFields: Object.keys(trusted),
  apiAcquiredFields: ['gateArtifact', 'docsArtifact'],
  constructedDocumentSha256: digest(canonical(expected)),
  note: 'Constructed descriptor hash is a receipt, not an independent root trust anchor. Manifest, source, CI, hosted expectations, registry, matrix and docs workflow are external pins.' }));
const allowedDownloads = [
  { kind: 'official-gate', descriptor: gateArtifact }, { kind: 'official-docs', descriptor: docsArtifact },
  ...hostedExpectations.jobs.map(job => ({ kind: `m4-${job.lane}-${job.baseline}`, descriptor: job.artifact })),
];
assert.equal(new Set(allowedDownloads.map(item => item.descriptor.id)).size, 8);
const acquisitions = [];
async function download(repository, artifact) {
  assert.equal(repository, PIN.repository);
  const allowed = allowedDownloads.find(item => item.descriptor.id === artifact.id); assert(allowed);
  assert.deepEqual(artifact, allowed.descriptor); assert(!acquisitions.some(item => item.artifact.id === artifact.id));
  const startedAt = new Date().toISOString();
  const bytes = await downloadPublicationArchive(repository, artifact); // Fresh original GET/redirect/whole hash path.
  assert.equal(bytes.length, artifact.bytes); assert.equal(digest(bytes), artifact.sha256);
  const record = { kind: allowed.kind, repository, artifact, startedAt, completedAt: new Date().toISOString(),
    actualBytes: bytes.length, actualSha256: digest(bytes), freshlyDownloaded: true,
    completeArchiveIncludedInReviewArtifact: !allowed.kind.startsWith('m4-') };
  if (allowed.kind === 'official-gate') write(join(review, 'gate-artifact.zip'), bytes);
  if (allowed.kind === 'official-docs') write(join(review, 'docs-artifact.zip'), bytes);
  acquisitions.push(record); return bytes;
}
const retainedRoot = join(output, 'official-retained');
// Unchanged original verifier authenticates docs run/job/check and artifact chronology, original gate metadata,
// all 42 CI jobs and six complete original archives, real Git source, complete evidence, and exact raw docs.
const originalAssessment = await verifyDocsArtifact({ expected, root, output: retainedRoot, api, download });
assert.equal(originalAssessment.eligible, true); assert.equal(originalAssessment.eligibilityScope, 'docs-artifact-only');
assert.equal(originalAssessment.hosted.executionComplete, true);
assert.equal(acquisitions.length, 8); assert.equal(acquisitions.filter(item => item.kind.startsWith('m4-')).length, 6);
cleanSource('after-original-docs-artifact-verification');

const independentRelative = `.zryna/out/docs/m414-final-independent-${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT}`;
const standaloneEnvironment = { ...process.env };
const clearedContextVariables = ['GITHUB_ACTIONS', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_REF_NAME', 'GITHUB_REF_TYPE'];
for (const key of clearedContextVariables) delete standaloneEnvironment[key];
const exportStdout = execFileSync(process.execPath, ['scripts/docs/export.mjs', '--channel', 'next',
  '--source-commit', PIN.commit, '--source-ref', PIN.ref, '--output', independentRelative], {
  cwd: root, env: standaloneEnvironment, timeout: 120_000, maxBuffer: 1024 * 1024,
  encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
assert.equal(exportStdout, `${trusted.manifestSha256}\n`, 'new independent export equals externally reviewed manifest pin');
const officialBundle = join(retainedRoot, 'docs/evidence'); const independentBundle = join(root, independentRelative);
const bundleExpectations = { expectedManifestSha256: trusted.manifestSha256, expectedChannel: 'next',
  expectedSourceCommit: PIN.commit, expectedSourceRef: PIN.ref };
const official = await validateDocsBundle(officialBundle, bundleExpectations, root);
const independent = await validateDocsBundle(independentBundle, bundleExpectations, root);
assert.deepEqual(official.manifest, originalAssessment.manifest); assert.deepEqual(official.manifest, independent.manifest);
assert.equal(official.manifest.documents.length, PIN.documents);
const expectedPaths = ['manifest.json', 'manifest.sha256', ...official.manifest.documents.map(item => item.path)].sort();
assert.equal(expectedPaths.length, PIN.bundleFiles);
assert.deepEqual((await scanFiles(officialBundle)).sort(), expectedPaths);
assert.deepEqual((await scanFiles(independentBundle)).sort(), expectedPaths);
const copiedOfficial = join(review, 'official-docs'); mkdirSync(copiedOfficial, { mode: 0o700 });
const comparedFiles = [];
for (const portable of expectedPaths) {
  assert(!isAbsolute(portable) && !portable.split('/').includes('..'));
  const officialBytes = readStableFile(join(officialBundle, portable), 2 * 1024 * 1024);
  const independentBytes = readStableFile(join(independentBundle, portable), 2 * 1024 * 1024);
  assert(officialBytes.equals(independentBytes), `all raw bytes must match: ${portable}`);
  const target = join(copiedOfficial, portable); mkdirSync(dirname(target), { recursive: true }); write(target, officialBytes);
  comparedFiles.push({ path: portable, bytes: officialBytes.length, officialSha256: digest(officialBytes),
    independentSha256: digest(independentBytes), byteEqual: true });
}
await validateDocsBundle(copiedOfficial, bundleExpectations, root);
cleanSource('after-independent-export-and-all-raw-file-comparison');
const finalFoundation = await authenticatePublicationFoundation(hostedExpectations, api, root);
assert.equal(finalFoundation.jobs.total_count, PIN.jobs); assert.equal(finalFoundation.inventory.jobs.length, PIN.jobs);
// Repeat fresh documentation attempt/job/check and original artifact-chronology authentication after comparison.
const finalRun = await api(`${prefix}/actions/runs/${trusted.docsRunId}/attempts/${trusted.docsAttempt}`);
assert.equal(finalRun.id, trusted.docsRunId); assert.equal(finalRun.run_attempt, trusted.docsAttempt);
assert.equal(finalRun.repository.full_name, PIN.repository); assert.equal(finalRun.path, '.github/workflows/documentation.yml');
assert.equal(finalRun.event, 'workflow_dispatch'); assert.equal(finalRun.head_sha, PIN.commit); assert.equal(finalRun.head_branch, 'main');
assert.equal(finalRun.status, 'completed'); assert.equal(finalRun.conclusion, 'success');
const finalJobs = await api(`${prefix}/actions/runs/${trusted.docsRunId}/attempts/${trusted.docsAttempt}/jobs?per_page=100`);
assert.equal(finalJobs.total_count, 1); assert.equal(finalJobs.jobs.length, 1);
const [finalJob] = finalJobs.jobs; assert.equal(finalJob.name, 'documentation');
assert.equal(finalJob.run_id, trusted.docsRunId); assert.equal(finalJob.run_attempt, trusted.docsAttempt);
assert.equal(finalJob.head_sha, PIN.commit); assert.equal(finalJob.status, 'completed'); assert.equal(finalJob.conclusion, 'success');
assert.match(finalJob.check_run_url, /^https:\/\/api\.github\.com\/repos\/zryna\/zryna\/check-runs\/[1-9][0-9]*$/u);
const finalCheck = await api(new URL(finalJob.check_run_url).pathname);
assert.equal(finalCheck.app.slug, 'github-actions'); assert.equal(finalCheck.head_sha, PIN.commit);
assert.equal(finalCheck.status, 'completed'); assert.equal(finalCheck.conclusion, 'success');
await authenticateDocsArtifact(expected, gateArtifact, gateName, api, finalJob);
await authenticateDocsArtifact(expected, docsArtifact, docsName, api, finalJob);
cleanSource('final');
const result = { format: 'zryna.m414-final-docs-verification.v1', source, pins: PIN, trusted,
  operator: { repository: process.env.GITHUB_REPOSITORY, commit: process.env.GITHUB_SHA,
    ref: process.env.GITHUB_REF, event: process.env.GITHUB_EVENT_NAME,
    runId: process.env.GITHUB_RUN_ID, attempt: process.env.GITHUB_RUN_ATTEMPT },
  coordinatorSha256: digest(readFileSync(process.argv[1])), originalAssessment,
  apiMetadataRetention: { ...apiMetadataLimits, actualObservations: observations.length, actualAggregateBytes: metadataBytes },
  initialFoundation, finalFoundation,
  independentExport: { provenance: 'unchanged original CLI; Git verification enabled on genuine clean local-main branch',
    childOnlyClearedContextVariables: clearedContextVariables, manifestSha256: trusted.manifestSha256,
    documents: PIN.documents, rawFiles: PIN.bundleFiles, allRawFilesByteEqual: true },
  archiveRetention: { originalCompleteArchivesFreshlyVerified: 8, completeM4Archives: 6,
    completeM4ArchivesIncludedInReviewArtifact: false, originals: 'pinned original CI artifact IDs',
    originalGateAndDocsArchivesIncludedInReviewArtifact: true,
    rootLocalScope: 'whole review ZIP and included original gate/docs ZIPs can be validated locally; six M4 whole ZIPs were verified on actual verifier runner' },
  eligibilityScope: 'docs-artifact-only', eligible: originalAssessment.eligible,
  broaderPublicationClaimed: false };
write(join(review, 'api-observations.json'), canonical(observations));
write(join(review, 'source-checkout.json'), canonical({ root, source, observations: sourceObservations }));
write(join(review, 'archive-acquisitions.json'), canonical(acquisitions));
write(join(review, 'original-assessment.json'), canonical(originalAssessment));
write(join(review, 'raw-file-comparison.json'), canonical({ manifestSha256: trusted.manifestSha256,
  documents: PIN.documents, rawFiles: PIN.bundleFiles, comparedFiles }));
write(join(review, 'result.json'), canonical(result));
write(join(review, 'completion.json'), canonical({ source, ci: { runId: PIN.ciRunId, attempt: PIN.ciAttempt },
  docs: { runId: trusted.docsRunId, attempt: trusted.docsAttempt },
  expectationsSha256: trusted.expectationsSha256, manifestSha256: trusted.manifestSha256,
  completeOriginalArchivesFreshlyVerified: acquisitions.length, completeM4Archives: 6,
  comparedRawFiles: PIN.bundleFiles, eligibilityScope: result.eligibilityScope, eligible: result.eligible }));
console.log(JSON.stringify({ source, docsRunId: trusted.docsRunId, docsAttempt: trusted.docsAttempt,
  manifestSha256: trusted.manifestSha256, comparedRawFiles: PIN.bundleFiles,
  eligibilityScope: result.eligibilityScope, eligible: result.eligible }));
