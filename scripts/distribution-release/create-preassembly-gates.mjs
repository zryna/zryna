import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded } from './canonical.mjs';
import { validatePreassemblyGatesShape } from './validate-preassembly-gates.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const API = 'https://api.github.com';
const REPOSITORY = 'zryna/zryna';
const WORKFLOW = '.github/workflows/ci.yml';
const REQUIRED = [
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
];
const MAX_RESPONSE = 1024 * 1024;
const REQUEST_TIMEOUT = 15_000;

function reject(message) {
  throw new Error(`R406-GATES-PRODUCER: ${message}`);
}

async function cancelReader(reader) {
  try {
    await reader.cancel();
  } catch {
    // The request still fails closed below even when transport cleanup fails.
  }
}

async function request(path, token, fetchImpl, timeoutMs) {
  const controller = new AbortController();
  const { promise: timeout, reject: rejectTimeout } = Promise.withResolvers();
  const timer = setTimeout(() => {
    controller.abort();
    rejectTimeout(new Error(`R406-GATES-PRODUCER: GitHub API ${path} timed out`));
  }, timeoutMs);
  const operation = (async () => {
    const response = await fetchImpl(`${API}${path}`, {
      headers: {
        Accept: 'application/vnd.github+json',
        'Accept-Encoding': 'identity',
        Authorization: `Bearer ${token}`,
        'X-GitHub-Api-Version': '2026-03-10',
      },
      redirect: 'error',
      signal: controller.signal,
    });
    if (!response.ok) reject(`GitHub API ${path} returned ${response.status}`);
    const contentEncoding = response.headers?.get?.('content-encoding');
    if (contentEncoding !== null && contentEncoding !== undefined
      && contentEncoding.toLowerCase() !== 'identity') {
      reject(`GitHub API ${path} returned encoded content`);
    }
    const declaredText = response.headers?.get?.('content-length');
    let declared;
    if (declaredText !== null && declaredText !== undefined) {
      if (!/^(0|[1-9][0-9]*)$/.test(declaredText)) {
        reject(`GitHub API ${path} returned an invalid content length`);
      }
      declared = Number(declaredText);
      if (!Number.isSafeInteger(declared) || declared > MAX_RESPONSE) {
        reject(`GitHub API ${path} exceeded ${MAX_RESPONSE} bytes`);
      }
    }
    if (!response.body || typeof response.body.getReader !== 'function') {
      reject(`GitHub API ${path} did not return a readable stream`);
    }
    const reader = response.body.getReader();
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
    if (declared !== undefined && declared !== size) {
      reject(`GitHub API ${path} content length differs from the streamed bytes`);
    }
    try {
      return JSON.parse(Buffer.concat(chunks, size).toString('utf8'));
    } catch {
      reject(`GitHub API ${path} did not return JSON`);
    }
  })();
  try {
    return await Promise.race([operation, timeout]);
  } catch (error) {
    if (error?.name === 'AbortError') {
      reject(`GitHub API ${path} timed out`);
    }
    throw error;
  } finally {
    clearTimeout(timer);
  }
}

function numericId(value, label) {
  const text = String(value);
  if (!/^[1-9][0-9]{0,19}$/.test(text)) reject(`${label} is not a bounded numeric ID`);
  return text;
}

export async function createPreassemblyGates({
  environment = process.env, fetchImpl = fetch, requestTimeoutMs = REQUEST_TIMEOUT,
} = {}) {
  const token = environment.GITHUB_TOKEN;
  const sourceCommit = environment.GITHUB_SHA;
  if (!token || environment.GITHUB_REPOSITORY !== REPOSITORY
    || environment.GITHUB_SERVER_URL !== 'https://github.com'
    || environment.GITHUB_REF !== 'refs/tags/v0.2.0'
    || !/^[0-9a-f]{40}$/.test(sourceCommit ?? '')) {
    reject('exact protected workflow repository, tag, SHA, and token are required');
  }
  if (!Number.isInteger(requestTimeoutMs) || requestTimeoutMs < 1
    || requestTimeoutMs > REQUEST_TIMEOUT) reject('request timeout is invalid');
  const query = new URLSearchParams({
    event: 'workflow_dispatch', status: 'success', head_sha: sourceCommit, per_page: '100',
  });
  const runs = await request(
    `/repos/${REPOSITORY}/actions/workflows/ci.yml/runs?${query}`,
    token,
    fetchImpl,
    requestTimeoutMs,
  );
  const matches = Array.isArray(runs.workflow_runs) ? runs.workflow_runs : [];
  if (matches.length !== 1 || runs.total_count !== 1) {
    reject('exactly one successful manual CI run must match the tagged commit');
  }
  const run = matches[0];
  const runId = numericId(run.id, 'run ID');
  const runAttempt = run.run_attempt;
  const runUrl = `https://github.com/${REPOSITORY}/actions/runs/${runId}`;
  const workflowPaths = [WORKFLOW, `${WORKFLOW}@main`, `${WORKFLOW}@refs/heads/main`];
  if (run.repository?.full_name !== REPOSITORY || !workflowPaths.includes(run.path)
    || run.event !== 'workflow_dispatch' || run.head_branch !== 'main'
    || run.head_sha !== sourceCommit || run.status !== 'completed' || run.conclusion !== 'success'
    || run.html_url !== runUrl || !Number.isInteger(runAttempt) || runAttempt < 1) {
    reject('selected CI run identity or conclusion differs');
  }
  const protection = await request(
    `/repos/${REPOSITORY}/branches/main/protection/required_status_checks`,
    token,
    fetchImpl,
    requestTimeoutMs,
  );
  const contexts = [...new Set([
    ...(Array.isArray(protection.contexts) ? protection.contexts : []),
    ...(Array.isArray(protection.checks) ? protection.checks.map(({ context }) => context) : []),
  ])].sort();
  if (JSON.stringify(contexts) !== JSON.stringify(REQUIRED)) {
    reject('current main required status contexts differ from the release contract');
  }
  const jobsResponse = await request(
    `/repos/${REPOSITORY}/actions/runs/${runId}/attempts/${runAttempt}/jobs?per_page=100`,
    token,
    fetchImpl,
    requestTimeoutMs,
  );
  if (!Array.isArray(jobsResponse.jobs) || jobsResponse.total_count !== jobsResponse.jobs.length
    || jobsResponse.jobs.length > 100) reject('selected attempt jobs are incomplete or unbounded');
  const requiredJobs = REQUIRED.map((name) => {
    const found = jobsResponse.jobs.filter((job) => job.name === name);
    if (found.length !== 1) reject(`required job ${name} is missing or duplicated`);
    const job = found[0];
    const jobId = numericId(job.id, `${name} job ID`);
    const checkRunId = numericId(job.check_run_url?.split('/').at(-1), `${name} check-run ID`);
    if (numericId(job.run_id, `${name} run ID`) !== runId
      || job.head_sha !== sourceCommit || job.status !== 'completed' || job.conclusion !== 'success'
      || job.workflow_name !== 'CI') reject(`required job ${name} differs from the selected attempt`);
    return {
      name, conclusion: 'success', sourceCommit, runId, runAttempt, jobId, checkRunId,
    };
  });
  return validatePreassemblyGatesShape({
    format: 'zryna.preassembly-gates.v1',
    repository: `https://github.com/${REPOSITORY}`,
    workflow: WORKFLOW,
    runId,
    runAttempt,
    runUrl,
    sourceCommit,
    requiredContexts: contexts,
    requiredJobs,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    process.stdout.write(`${canonicalBounded(await createPreassemblyGates())}\n`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
