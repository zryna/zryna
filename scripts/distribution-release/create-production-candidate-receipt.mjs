import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { ACCEPTED_RECIPE_SHA256 } from './check-release-readiness.mjs';
import { canonicalBounded, parseCanonical } from './canonical.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const API = 'https://api.github.com';
const REPOSITORY = 'zryna/zryna';
const WORKFLOW = '.github/workflows/release-production-candidate.yml';
const MAX_RESPONSE = 1024 * 1024;
const REQUEST_TIMEOUT = 15_000;
const OBJECT_ID = /^[0-9a-f]{40}$/;
const DIGEST = /^[0-9a-f]{64}$/;
const REQUIRED_JOBS = Object.freeze([
  'admit protected production candidate',
  'accept installed candidate x86_64-pc-windows-msvc',
  'accept installed candidate x86_64-unknown-linux-gnu',
  'build candidate x86_64-pc-windows-msvc replica 1',
  'build candidate x86_64-pc-windows-msvc replica 2',
  'build candidate x86_64-unknown-linux-gnu replica 1',
  'build candidate x86_64-unknown-linux-gnu replica 2',
  'reproduce candidate x86_64-pc-windows-msvc',
  'reproduce candidate x86_64-unknown-linux-gnu',
]);

function reject(message) {
  throw new Error(`R406-PRODUCTION-CANDIDATE-RECEIPT: ${message}`);
}

function exactKeys(value, keys, label) {
  if (value === null || typeof value !== 'object' || Array.isArray(value)
    || Object.keys(value).sort().join('\0') !== [...keys].sort().join('\0')) {
    reject(`${label} fields differ`);
  }
}

function numericId(value, label) {
  const text = String(value);
  if (!/^[1-9][0-9]{0,19}$/.test(text)) reject(`${label} differs`);
  return text;
}

async function cancelReader(reader) {
  try { await reader.cancel(); } catch { /* The request still fails closed. */ }
}

async function request(path, token, fetchImpl, timeoutMs) {
  const controller = new AbortController();
  const { promise: timeout, reject: rejectTimeout } = Promise.withResolvers();
  const timer = setTimeout(() => {
    controller.abort();
    rejectTimeout(new Error(`R406-PRODUCTION-CANDIDATE-RECEIPT: GitHub API ${path} timed out`));
  }, timeoutMs);
  const operation = (async () => {
    const response = await fetchImpl(`${API}${path}`, {
      headers: {
        Accept: 'application/vnd.github+json', 'Accept-Encoding': 'identity',
        Authorization: `Bearer ${token}`, 'X-GitHub-Api-Version': '2026-03-10',
      },
      redirect: 'error', signal: controller.signal,
    });
    if (!response.ok) reject(`GitHub API ${path} returned ${response.status}`);
    const encoding = response.headers?.get?.('content-encoding');
    if (encoding !== null && encoding !== undefined && encoding.toLowerCase() !== 'identity') {
      reject(`GitHub API ${path} returned encoded content`);
    }
    const reader = response.body?.getReader?.();
    if (!reader) reject(`GitHub API ${path} did not return a readable stream`);
    const chunks = [];
    let size = 0;
    while (true) {
      const { done, value } = await reader.read();
      if (done) break;
      if (!(value instanceof Uint8Array)) {
        await cancelReader(reader);
        reject(`GitHub API ${path} returned invalid bytes`);
      }
      size += value.byteLength;
      if (size > MAX_RESPONSE) {
        await cancelReader(reader);
        reject(`GitHub API ${path} exceeded ${MAX_RESPONSE} bytes`);
      }
      chunks.push(Buffer.from(value));
    }
    try { return JSON.parse(Buffer.concat(chunks, size).toString('utf8')); }
    catch { reject(`GitHub API ${path} did not return JSON`); }
  })();
  try { return await Promise.race([operation, timeout]); }
  catch (error) {
    if (error?.name === 'AbortError') reject(`GitHub API ${path} timed out`);
    throw error;
  } finally { clearTimeout(timer); }
}

export function validateProductionCandidateReceipt(value, sourceCommit,
  acceptedRecipeSha256 = ACCEPTED_RECIPE_SHA256) {
  canonicalBounded(value);
  exactKeys(value, [
    'format', 'status', 'productionAdmission', 'workflow', 'sourceCommit', 'recipeSha256',
    'runId', 'runAttempt', 'runUrl', 'requiredJobs',
  ], 'receipt');
  if (value.format !== 'zryna.production-candidate-receipt.v1'
    || value.status !== 'production-candidate-passed'
    || value.productionAdmission !== 'candidate-prerequisite-only'
    || value.workflow !== WORKFLOW || value.sourceCommit !== sourceCommit
    || !OBJECT_ID.test(value.sourceCommit) || value.recipeSha256 !== acceptedRecipeSha256
    || !DIGEST.test(value.recipeSha256 ?? '')
    || value.runUrl !== `https://github.com/${REPOSITORY}/actions/runs/${value.runId}`
    || !Number.isInteger(value.runAttempt) || value.runAttempt < 1 || value.runAttempt > 100
    || !Array.isArray(value.requiredJobs) || value.requiredJobs.length !== REQUIRED_JOBS.length) {
    reject('receipt identity differs');
  }
  numericId(value.runId, 'run ID');
  for (const [index, job] of value.requiredJobs.entries()) {
    exactKeys(job, ['name', 'conclusion', 'jobId', 'checkRunId'], `job ${index + 1}`);
    if (job.name !== REQUIRED_JOBS[index] || job.conclusion !== 'success') {
      reject(`job ${index + 1} identity differs`);
    }
    numericId(job.jobId, `job ${index + 1} ID`);
    numericId(job.checkRunId, `job ${index + 1} check-run ID`);
  }
  return value;
}

export function validateProductionCandidateReceiptText(text, sourceCommit,
  acceptedRecipeSha256 = ACCEPTED_RECIPE_SHA256) {
  return validateProductionCandidateReceipt(parseCanonical(text), sourceCommit,
    acceptedRecipeSha256);
}

export async function createProductionCandidateReceipt({
  environment = process.env, fetchImpl = fetch, requestTimeoutMs = REQUEST_TIMEOUT,
  acceptedRecipeSha256 = ACCEPTED_RECIPE_SHA256,
} = {}) {
  const sourceCommit = environment.GITHUB_SHA;
  if (!environment.GITHUB_TOKEN || environment.GITHUB_EVENT_NAME !== 'push'
    || environment.GITHUB_REPOSITORY !== REPOSITORY
    || environment.GITHUB_SERVER_URL !== 'https://github.com'
    || environment.GITHUB_REF !== 'refs/tags/v0.2.1'
    || environment.GITHUB_REF_PROTECTED !== 'true' || !OBJECT_ID.test(sourceCommit ?? '')
    || environment.GITHUB_WORKFLOW_SHA !== sourceCommit
    || !Number.isInteger(requestTimeoutMs) || requestTimeoutMs < 1
    || requestTimeoutMs > REQUEST_TIMEOUT || !DIGEST.test(acceptedRecipeSha256 ?? '')) {
    reject('exact protected tag context, accepted recipe, and token are required');
  }
  const query = new URLSearchParams({
    branch: 'main', event: 'workflow_dispatch', status: 'success', head_sha: sourceCommit,
    per_page: '100',
  });
  const runs = await request(
    `/repos/${REPOSITORY}/actions/workflows/release-production-candidate.yml/runs?${query}`,
    environment.GITHUB_TOKEN, fetchImpl, requestTimeoutMs,
  );
  const matches = Array.isArray(runs.workflow_runs) ? runs.workflow_runs : [];
  if (runs.total_count !== 1 || matches.length !== 1) {
    reject('exactly one successful production-candidate run must match the tagged commit');
  }
  const run = matches[0];
  const runId = numericId(run.id, 'run ID');
  const runAttempt = run.run_attempt;
  const runUrl = `https://github.com/${REPOSITORY}/actions/runs/${runId}`;
  if (run.repository?.full_name !== REPOSITORY
    || ![WORKFLOW, `${WORKFLOW}@main`, `${WORKFLOW}@refs/heads/main`].includes(run.path)
    || run.event !== 'workflow_dispatch' || run.head_branch !== 'main'
    || run.head_sha !== sourceCommit || run.status !== 'completed' || run.conclusion !== 'success'
    || run.html_url !== runUrl || !Number.isInteger(runAttempt) || runAttempt < 1
    || runAttempt > 100) reject('selected production-candidate run identity differs');
  const jobsResponse = await request(
    `/repos/${REPOSITORY}/actions/runs/${runId}/attempts/${runAttempt}/jobs?per_page=100`,
    environment.GITHUB_TOKEN, fetchImpl, requestTimeoutMs,
  );
  if (!Array.isArray(jobsResponse.jobs) || jobsResponse.total_count !== jobsResponse.jobs.length
    || jobsResponse.jobs.length > 100) reject('selected candidate jobs are incomplete or unbounded');
  const requiredJobs = REQUIRED_JOBS.map((name) => {
    const found = jobsResponse.jobs.filter((job) => job.name === name);
    if (found.length !== 1) reject(`required job ${name} is missing or duplicated`);
    const job = found[0];
    if (numericId(job.run_id, `${name} run ID`) !== runId || job.head_sha !== sourceCommit
      || job.status !== 'completed' || job.conclusion !== 'success'
      || job.workflow_name !== 'Production release candidate') {
      reject(`required job ${name} differs from the selected attempt`);
    }
    return {
      name, conclusion: 'success', jobId: numericId(job.id, `${name} job ID`),
      checkRunId: numericId(job.check_run_url?.split('/').at(-1), `${name} check-run ID`),
    };
  });
  return validateProductionCandidateReceipt({
    format: 'zryna.production-candidate-receipt.v1',
    status: 'production-candidate-passed', productionAdmission: 'candidate-prerequisite-only',
    workflow: WORKFLOW, sourceCommit, recipeSha256: acceptedRecipeSha256,
    runId, runAttempt, runUrl, requiredJobs,
  }, sourceCommit, acceptedRecipeSha256);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try { process.stdout.write(`${canonicalBounded(await createProductionCandidateReceipt())}\n`); }
  catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
