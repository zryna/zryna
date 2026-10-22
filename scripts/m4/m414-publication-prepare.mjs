// Review-only operator coordinator for zryna/zryna#414. Original validators are imported unchanged.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { existsSync, lstatSync, mkdirSync, readFileSync, realpathSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

const PIN = Object.freeze({ repository: 'zryna/zryna',
  commit: '01c81f4087982b2f1c99c85f9a7c0360dbeb0e28',
  tree: 'd5544b2df1fd19fd23bb65887a5f3cbec1b033d7',
  parents: ['236d2a7126dd0a9722cd07974843b84fbc48f76c'],
  ref: 'refs/heads/main', runId: 39737726701, attempt: 1, jobs: 42, documents: 61, bundleFiles: 63 });
const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
const canonical = value => Buffer.from(`${JSON.stringify(value, null, 2)}\n`);
const [sourceFlag, sourceArg, outputFlag, outputArg] = process.argv.slice(2);
assert.equal(process.argv.length, 6);
assert.equal(sourceFlag, '--source-root'); assert.equal(outputFlag, '--output');
assert(isAbsolute(sourceArg) && isAbsolute(outputArg));
assert.equal(process.env.GITHUB_ACTIONS, 'true');
assert.equal(process.env.GITHUB_REPOSITORY, PIN.repository);
assert.equal(process.versions.node, '22.22.1');
assert(process.env.GITHUB_TOKEN, 'actual Actions read token required');
for (const key of ['GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']) assert.match(process.env[key] ?? '', /^[1-9][0-9]*$/u);
const root = realpathSync(sourceArg);
const output = resolve(outputArg);
const runnerTemp = realpathSync(process.env.RUNNER_TEMP);
assert(output.startsWith(`${runnerTemp}${sep}`), 'output must be an exclusive runner-temp child');
assert(!output.startsWith(`${root}${sep}`), 'coordinator output must remain outside clean source');
assert(!existsSync(output), 'exclusive output required');
assert(!resolve(process.argv[1]).startsWith(`${root}${sep}`), 'stage coordinator outside selected source');
const git = args => execFileSync('git', args, { cwd: root, timeout: 30_000,
  maxBuffer: 1024 * 1024, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] }).trim();
const sourceObservations = [];
function cleanSource() {
  const observed = { observedAt: new Date().toISOString(),
    commit: git(['rev-parse', 'HEAD']), tree: git(['rev-parse', 'HEAD^{tree}']),
    parents: git(['show', '-s', '--format=%P', 'HEAD']).split(' '),
    status: git(['status', '--porcelain=v1', '--untracked-files=all']) };
  assert.equal(observed.commit, PIN.commit); assert.equal(observed.tree, PIN.tree);
  assert.deepEqual(observed.parents, PIN.parents);
  assert.equal(observed.status, '', 'exact source checkout stays clean');
  sourceObservations.push(observed);
}
cleanSource();
// The independent export uses genuine local-main branch provenance below.
assert.equal(git(['symbolic-ref', '-q', 'HEAD']), PIN.ref, 'real local main branch required for standalone export');
mkdirSync(output, { recursive: false, mode: 0o700 });
mkdirSync(join(output, 'metadata'), { mode: 0o700 });
const write = (path, bytes) => writeFileSync(path, bytes, { flag: 'wx', mode: 0o600 });
const load = path => import(pathToFileURL(join(root, path)).href);
const { githubApi } = await load('scripts/m4/hosted-api.mjs');
const { HOSTED_LANES, validateExpectations } = await load('scripts/m4/hosted-expectations.mjs');
const { parseCanonical, readStableFile, digest } = await load('scripts/m4/registry-files.mjs');
const { sourceBinding } = await load('scripts/m4/evidence-context.mjs');
const { downloadPublicationArchive, retainPublicationArchive } = await load('scripts/m4/retain-publication.mjs');
const { loadPublicationWorkflowInventory, verifyCompletePublicationJobs } = await load('scripts/m4/publication-workflows.mjs');
const { authenticatePublicationFoundation } = await load('scripts/m4/verify-publication.mjs');
const { loadHostedRegistry, verifyM4Hosted } = await load('scripts/m4/verify-hosted.mjs');
const { checkSupportFiles, canonicalSupportBytes } = await load('scripts/m4/support.mjs');
const { validateDocsBundle } = await load('scripts/docs/bundle.mjs');
const observations = [];
const originalApi = githubApi();
async function api(endpoint) {
  assert(endpoint.startsWith(`/repos/${PIN.repository}/`), 'only exact-repository GETs');
  assert(observations.length < 180, 'finite API observation budget');
  const observedAt = new Date().toISOString();
  const value = await originalApi(endpoint); // Original API timeout, redirect policy and JSON bounds.
  const bytes = canonical(value);
  assert(bytes.length <= 8 * 1024 * 1024);
  const file = `${String(observations.length + 1).padStart(3, '0')}-${sha256(Buffer.from(endpoint)).slice(0, 16)}.json`;
  write(join(output, 'metadata', file), bytes);
  observations.push({ endpoint, method: 'GET', observedAt, file: `metadata/${file}`,
    bytes: bytes.length, sha256: sha256(bytes), representation: 'parsed API value serialized as JSON with final LF' });
  return value;
}
const prefix = `/repos/${PIN.repository}`;
const source = { commit: PIN.commit, tree: PIN.tree, ref: PIN.ref };
const run = await api(`${prefix}/actions/runs/${PIN.runId}/attempts/${PIN.attempt}`);
assert.equal(run.id, PIN.runId); assert.equal(run.run_attempt, PIN.attempt);
assert.equal(run.repository.full_name, PIN.repository); assert.equal(run.path, '.github/workflows/ci.yml');
assert.equal(run.head_sha, PIN.commit); assert.equal(run.head_branch, 'main');
assert.equal(run.event, 'workflow_dispatch');
assert.equal(run.status, 'completed', 'do not qualify queued/in-progress exact-main CI');
assert.equal(run.conclusion, 'success');
const commit = await api(`${prefix}/git/commits/${PIN.commit}`);
assert.equal(commit.sha, PIN.commit); assert.equal(commit.tree.sha, PIN.tree);
assert.deepEqual(commit.parents.map(item => item.sha), PIN.parents);
const main = await api(`${prefix}/git/ref/heads/main`);
assert.equal(main.object.sha, PIN.commit, 'main advancement requires a new review');
const observedJobs = await api(`${prefix}/actions/runs/${PIN.runId}/attempts/${PIN.attempt}/jobs?per_page=100`);
const inventory = await loadPublicationWorkflowInventory({ repository: PIN.repository, source, root, api });
assert.equal(inventory.jobs.length, PIN.jobs); assert.equal(observedJobs.total_count, PIN.jobs);
verifyCompletePublicationJobs(observedJobs, inventory, { runId: PIN.runId, attempt: PIN.attempt, commit: PIN.commit });
const artifacts = await api(`${prefix}/actions/runs/${PIN.runId}/artifacts?per_page=100`);
assert(Array.isArray(artifacts.artifacts) && artifacts.total_count === artifacts.artifacts.length
  && artifacts.total_count <= 100, 'one complete bounded artifact inventory required');
const workflowSources = ['.github/workflows/ci.yml', '.github/workflows/browser-conformance.yml',
  '.github/workflows/installed-command-h1.yml'].map(path => {
  const bytes = readStableFile(join(root, path), 1024 * 1024);
  return { path, bytes: bytes.length, sha256: digest(bytes) };
});
const retainedRoot = join(output, 'retained'); mkdirSync(retainedRoot, { mode: 0o700 });
const jobs = [];
for (const [lane, baseline, name, jobKey] of HOSTED_LANES) {
  const jobName = lane === 'core' ? name : `m4 ${lane} / ${name}`;
  const matchingJobs = observedJobs.jobs.filter(item => item.name === jobName);
  assert.equal(matchingJobs.length, 1);
  const observed = matchingJobs[0];
  const match = /^https:\/\/api\.github\.com\/repos\/zryna\/zryna\/check-runs\/([1-9][0-9]*)$/u.exec(observed.check_run_url);
  assert(match); const checkRunId = Number(match[1]); assert(Number.isSafeInteger(checkRunId));
  const check = await api(`${prefix}/check-runs/${checkRunId}`);
  const artifactName = `m4-${lane}-${baseline}-${PIN.commit}-${PIN.runId}-${PIN.attempt}`;
  const matchingArtifacts = artifacts.artifacts.filter(item => item.name === artifactName);
  assert.equal(matchingArtifacts.length, 1, `one exact complete archive for ${lane}/${baseline}`);
  const listed = matchingArtifacts[0];
  const actual = await api(`${prefix}/actions/artifacts/${listed.id}`);
  assert.deepEqual(actual, listed, 'fresh single-artifact metadata matches run inventory');
  assert.equal(actual.workflow_run.id, PIN.runId); assert.equal(actual.workflow_run.head_sha, PIN.commit);
  assert.equal(actual.expired, false); assert(Date.parse(actual.expires_at) > Date.now());
  assert.match(actual.digest, /^sha256:[a-f0-9]{64}$/u);
  const artifact = { id: actual.id, name: actual.name, sha256: actual.digest.slice(7),
    bytes: actual.size_in_bytes, expiresAt: actual.expires_at };
  const archive = await downloadPublicationArchive(PIN.repository, artifact);
  assert.equal(archive.length, artifact.bytes); assert.equal(digest(archive), artifact.sha256);
  const directory = join(retainedRoot, `${lane}-${baseline}`);
  retainPublicationArchive(directory, archive); // Original complete hostile-ZIP parser and create-only retention.
  const receipt = parseCanonical(readStableFile(join(directory, 'evidence/receipt.json'), 8 * 1024 * 1024));
  sourceBinding(receipt.source, source);
  assert.equal(receipt.lane, lane);
  const workflowPath = lane === 'core' ? '.github/workflows/ci.yml'
    : lane === 'browser' ? '.github/workflows/browser-conformance.yml' : '.github/workflows/installed-command-h1.yml';
  const workflowSha256 = workflowSources.find(item => item.path === workflowPath).sha256;
  assert.deepEqual(receipt.workflow, { repository: PIN.repository, runId: PIN.runId, attempt: PIN.attempt,
    jobName, jobKey, workflowPath, workflowSha256, event: run.event, headCommit: PIN.commit,
    executionCommit: PIN.commit, checkoutCommit: PIN.commit });
  jobs.push({ lane, baseline, runId: PIN.runId, attempt: PIN.attempt, jobId: observed.id, jobName, jobKey,
    runWorkflowPath: '.github/workflows/ci.yml', workflowPath, workflowSha256, event: run.event,
    headCommit: PIN.commit, checkRunId, checkUrl: check.html_url, artifact,
    host: receipt.host, tools: receipt.tools, materials: receipt.materials, evidenceDirectory: receipt.evidenceDirectory });
}
const aggregates = observedJobs.jobs.filter(item => item.name === 'm4'); assert.equal(aggregates.length, 1);
const aggregateJob = aggregates[0];
const aggregateMatch = /^https:\/\/api\.github\.com\/repos\/zryna\/zryna\/check-runs\/([1-9][0-9]*)$/u.exec(aggregateJob.check_run_url);
assert(aggregateMatch);
const aggregateCheck = await api(`${prefix}/check-runs/${aggregateMatch[1]}`);
const registryBytes = readStableFile(join(root, 'tests/m4-conformance-v1.json'), 8 * 1024 * 1024);
const expectations = validateExpectations({ format: 'zryna.m4-hosted-expectations.v1', repository: PIN.repository,
  source, registrySha256: digest(registryBytes), workflowSources,
  workflowExecution: { commit: PIN.commit, tree: PIN.tree, parents: PIN.parents },
  eventSource: { commit: PIN.commit, tree: PIN.tree, parents: PIN.parents }, jobs,
  aggregate: { jobId: aggregateJob.id, checkRunId: aggregateCheck.id, checkUrl: aggregateCheck.html_url } });
const expectationsBytes = canonical(expectations); assert(expectationsBytes.length <= 60 * 1024);
assert.deepEqual(parseCanonical(expectationsBytes), expectations);
write(join(output, 'expectations.json'), expectationsBytes);
write(join(output, 'expectations.sha256'), Buffer.from(`${digest(expectationsBytes)}  expectations.json\n`));
const foundation = await authenticatePublicationFoundation(expectations, api, root);
const registry = loadHostedRegistry(root);
const hosted = await verifyM4Hosted({ expectations, retainedRoot, root, registry, api });
assert.equal(hosted.executionComplete, true, 'unchanged original six-lane validation required');
const support = checkSupportFiles(root);
assert.equal(support.registrySha256, expectations.registrySha256);
cleanSource();

// Honest standalone comparison: local branch/HEAD provenance is checked by the unchanged original CLI.
// The operator's actual Actions identity is retained in the receipt, never replaced with main identities.
const independentRelative = `.zryna/out/docs/m414-independent-${process.env.GITHUB_RUN_ID}-${process.env.GITHUB_RUN_ATTEMPT}`;
const standaloneEnvironment = { ...process.env };
for (const key of ['GITHUB_ACTIONS', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_REF_NAME', 'GITHUB_REF_TYPE']) delete standaloneEnvironment[key];
const exportOutput = execFileSync(process.execPath, ['scripts/docs/export.mjs', '--channel', 'next',
  '--source-commit', PIN.commit, '--source-ref', PIN.ref, '--output', independentRelative], {
  cwd: root, env: standaloneEnvironment, timeout: 120_000, maxBuffer: 1024 * 1024,
  encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
assert.match(exportOutput, /^[a-f0-9]{64}\n$/u);
const manifestSha256 = exportOutput.trim();
const originalBundle = join(root, independentRelative);
const bundle = await validateDocsBundle(originalBundle, { expectedManifestSha256: manifestSha256,
  expectedChannel: 'next', expectedSourceCommit: PIN.commit, expectedSourceRef: PIN.ref }, root);
assert.equal(bundle.manifest.documents.length, PIN.documents);
const independentBundle = join(output, 'independent-docs'); mkdirSync(independentBundle, { mode: 0o700 });
for (const portable of ['manifest.json', 'manifest.sha256', ...bundle.manifest.documents.map(item => item.path)]) {
  assert(!isAbsolute(portable) && !portable.split('/').includes('..'));
  const from = join(originalBundle, portable); const metadata = lstatSync(from);
  assert(metadata.isFile() && !metadata.isSymbolicLink());
  const bytes = readStableFile(from, 2 * 1024 * 1024);
  const target = join(independentBundle, portable); mkdirSync(dirname(target), { recursive: true }); write(target, bytes);
}
await validateDocsBundle(independentBundle, { expectedManifestSha256: manifestSha256,
  expectedChannel: 'next', expectedSourceCommit: PIN.commit, expectedSourceRef: PIN.ref }, root);
cleanSource();
const finalFoundation = await authenticatePublicationFoundation(expectations, api, root);
const assessment = { format: 'zryna.m414-preparation-assessment.v1', pins: PIN, source,
  operator: { repository: process.env.GITHUB_REPOSITORY, commit: process.env.GITHUB_SHA,
    ref: process.env.GITHUB_REF, event: process.env.GITHUB_EVENT_NAME,
    runId: process.env.GITHUB_RUN_ID, attempt: process.env.GITHUB_RUN_ATTEMPT },
  coordinatorSha256: digest(readFileSync(process.argv[1])), expectationsSha256: digest(expectationsBytes),
  docsWorkflowSha256: digest(readStableFile(join(root, '.github/workflows/documentation.yml'), 1024 * 1024)),
  registrySha256: expectations.registrySha256, matrixSha256: digest(canonicalSupportBytes(support)),
  foundation, hosted, finalFoundation, independentManifestSha256: manifestSha256,
  independentExport: { provenance: 'standalone real clean local main Git branch; original CLI Git verification enabled',
    documents: bundle.manifest.documents.length, rawBundle: 'independent-docs',
    workflowContextVariablesRemoved: ['GITHUB_ACTIONS', 'GITHUB_SHA', 'GITHUB_REF', 'GITHUB_REF_NAME', 'GITHUB_REF_TYPE'] },
  scope: 'preparation-and-independent-comparison-only',
  archiveRetention: { completeArchivesValidatedOn: 'actual operator runner by unchanged original validators',
    runnerDirectory: 'retained', includedInReviewArtifact: false,
    originalArchivesRemainAt: 'six pinned original CI artifact IDs in archive-receipts.json',
    laterVerification: 'final verifier must independently redownload and authenticate all six complete original ZIPs' },
  blockers: ['Independently review canonical expectations before original documentation workflow dispatch.',
    'Original documentation workflow and final authenticated artifact verification remain required.'] };
write(join(output, 'api-observations.json'), canonical(observations));
write(join(output, 'source-checkout.json'), canonical({ root, localBranch: git(['symbolic-ref', '-q', 'HEAD']),
  source, observations: sourceObservations, trackedFilesAuthenticatedBy: 'real Git HEAD/tree and original unchanged validators' }));
write(join(output, 'archive-receipts.json'), canonical(jobs.map(job => ({ lane: job.lane, baseline: job.baseline,
  artifact: job.artifact, operatorRunnerCompleteArchive: `retained/${job.lane}-${job.baseline}/artifact.zip`,
  operatorRunnerRetainedEvidence: `retained/${job.lane}-${job.baseline}/evidence`, originalHostReceipt: 'receipt.json',
  completeArchiveIncludedInReviewArtifact: false }))));
write(join(output, 'assessment.json'), canonical(assessment));
write(join(output, 'completion.json'), canonical({ source, ci: { runId: PIN.runId, attempt: PIN.attempt },
  expectationsSha256: digest(expectationsBytes), manifestSha256, executionComplete: hosted.executionComplete,
  scope: assessment.scope, archiveCount: jobs.length, documents: PIN.documents, bundleFiles: PIN.bundleFiles }));
console.log(JSON.stringify({ expectationsSha256: digest(expectationsBytes), manifestSha256,
  executionComplete: hosted.executionComplete, scope: assessment.scope }));
