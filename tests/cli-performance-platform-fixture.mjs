// Independent synthetic platform/workflow responses. No live execution or source approval.
import { createHash } from 'node:crypto';
import { stringify } from 'yaml';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
export const CALLER = '.github/workflows/cli-performance.yml';
export const CALLEE = '.github/workflows/cli-performance-collector.yml';
export const PREFIX = '/repos/zryna/zryna';
export const NOW = Date.parse('2026-10-27T00:00:00Z');
export const repository = () => ({ id: 1350402316, full_name: 'zryna/zryna' });
export function platformFixture(pr = false) {
  const caller = { name: 'Synthetic caller', on: { pull_request: null, workflow_dispatch: null },
    permissions: { contents: 'read' }, concurrency: { group: 'caller-${{ github.ref }}', 'cancel-in-progress': true },
    jobs: { observe: { name: 'observe', uses: './.github/workflows/cli-performance-collector.yml' } } };
  const callee = { name: 'Synthetic collector', on: { workflow_call: null }, permissions: { contents: 'read' },
    concurrency: { group: 'collector-${{ github.ref }}', 'cancel-in-progress': true }, jobs: { collect: {
      name: '${{ matrix.os }} / ${{ matrix.round }} (unqualified)', 'runs-on': '${{ matrix.os }}', 'timeout-minutes': 60,
      strategy: { 'fail-fast': false, matrix: { os: ['ubuntu-24.04', 'windows-2022'],
        round: ['baseline-round-1', 'baseline-round-2'] } }, env: {
        EXPECTED_SOURCE_REVISION: '${{ github.event.pull_request.head.sha || github.sha }}',
        DECLARED_PR_HEAD: '${{ github.event.pull_request.head.sha || github.sha }}',
        M418_ROUND: '${{ matrix.round }}', CARGO_BUILD_JOBS: 2, CARGO_INCREMENTAL: 0 },
      steps: [{ uses: 'actions/checkout@3d3c42e5aac5ba805825da76410c181273ba90b1', with: {
        'fetch-depth': 0, 'persist-credentials': false, ref: '${{ github.event.pull_request.head.sha || github.sha }}' } },
      { name: 'Assemble observed operator inputs and collect one fresh round', shell: 'pwsh',
        run: "if ($head -ne $env:EXPECTED_SOURCE_REVISION) { throw 'head' }\nnode scripts/stability-gates/run-cli-performance-hosted.mjs\n" }] } } };
  const value = { format: 'zryna.cli-performance-hosted-expectations.v1', repository: 'zryna/zryna', repositoryId: 1350402316,
    source: { commit: '1'.repeat(40), tree: '2'.repeat(40), ref: 'refs/heads/fix/performance-proof' },
    execution: { commit: (pr ? '4' : '1').repeat(40), tree: (pr ? '5' : '2').repeat(40),
      parents: pr ? ['3'.repeat(40), '1'.repeat(40)] : ['3'.repeat(40)] },
    run: { id: 100, attempt: 1, event: pr ? 'pull_request' : 'workflow_dispatch', pullRequest: pr ? 575 : null },
    workflows: [], jobs: [] };
  let jobId = 101, artifactId = 301;
  const archive = name => ({ id: artifactId++, name, bytes: 1000, sha256: 'a'.repeat(64), expiresAt: '2026-11-01T00:00:00Z' });
  for (const [host, runner] of [['linux-x86_64', 'ubuntu-24.04'], ['windows-x86_64', 'windows-2022']])
    for (const round of ['baseline-round-1', 'baseline-round-2']) value.jobs.push({ host, round, jobId: jobId++, checkRunId: jobId + 99,
      primary: archive(`cli-performance-${runner}-${round}-100-1`),
      provider: host === 'linux-x86_64' ? archive(`cli-performance-linux-provider-${round}-100-1`) : null });
  const run = { id: 100, run_attempt: 1, check_suite_id: 500, event: value.run.event, path: CALLER, status: 'completed', conclusion: 'success',
    head_sha: value.source.commit, head_branch: 'fix/performance-proof', repository: repository(), head_repository: repository(),
    referenced_workflows: [{ path: `zryna/zryna/${CALLEE}@${value.execution.commit}`, sha: value.execution.commit,
      ref: pr ? 'refs/pull/575/merge' : value.source.ref }], pull_requests: pr ? [{ number: 575,
      head: { sha: value.source.commit, ref: 'fix/performance-proof', repo: { id: 1350402316 } },
      base: { sha: value.execution.parents[0], ref: 'main', repo: { id: 1350402316 } } }] : [] };
  const responses = new Map([[`${PREFIX}/actions/runs/100`, run],
    [`${PREFIX}/git/commits/${value.source.commit}`, { sha: value.source.commit, tree: { sha: value.source.tree },
      parents: [{ sha: '3'.repeat(40) }] }]]);
  if (pr) responses.set(`${PREFIX}/git/commits/${value.execution.commit}`, { sha: value.execution.commit,
    tree: { sha: value.execution.tree }, parents: value.execution.parents.map(sha => ({ sha })) });
  function refreshWorkflows() {
    value.workflows = [];
    for (const [path, document] of [[CALLER, caller], [CALLEE, callee]]) {
      const bytes = Buffer.from(stringify(document));
      value.workflows.push({ path, bytes: bytes.length, sha256: sha256(bytes) });
      for (const identity of new Set([value.source.commit, value.execution.commit]))
        responses.set(`${PREFIX}/contents/${path}?ref=${identity}`, { type: 'file', path, encoding: 'base64',
          size: bytes.length, content: bytes.toString('base64'),
          sha: createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex') });
    }
  }
  refreshWorkflows();
  const jobs = [], artifacts = [];
  for (const wanted of value.jobs) {
    const name = `observe / ${wanted.host === 'linux-x86_64' ? 'ubuntu-24.04' : 'windows-2022'} / ${wanted.round} (unqualified)`;
    const steps = ['Checkout exact observed head', 'Frozen workflow dependencies outside measurement',
      'Verify fixed collectors oracles and release capture controls', 'Locked Rust dependency preparation outside builder and measurement',
      'Assemble observed operator inputs and collect one fresh round', 'Retain all observations and failures',
      'Build Windows helper and run exactly eight controls once', 'Retain Linux provider acquisition and loader observations']
      .map((name, i) => ({ name, number: i + 1, status: 'completed', conclusion: 'success' }));
    steps[wanted.host === 'linux-x86_64' ? 6 : 7].conclusion = 'skipped';
    jobs.push({ id: wanted.jobId, name, run_id: 100, run_attempt: 1, head_sha: value.source.commit,
      status: 'completed', conclusion: 'success', check_run_url: `https://api.github.com${PREFIX}/check-runs/${wanted.checkRunId}`,
      started_at: '2026-10-26T00:00:00Z', completed_at: '2026-10-26T01:00:00Z', steps });
    responses.set(`${PREFIX}/check-runs/${wanted.checkRunId}`, { id: wanted.checkRunId, name, head_sha: value.source.commit,
      status: 'completed', conclusion: 'success', app: { slug: 'github-actions' }, check_suite: { id: 500 } });
    for (const descriptor of [wanted.primary, wanted.provider].filter(Boolean)) artifacts.push({ id: descriptor.id,
      name: descriptor.name, size_in_bytes: descriptor.bytes, digest: `sha256:${descriptor.sha256}`, expired: false,
      created_at: '2026-10-26T00:59:00Z', expires_at: descriptor.expiresAt, workflow_run: { id: 100,
        repository_id: 1350402316, head_repository_id: 1350402316, head_sha: value.source.commit, head_branch: run.head_branch } });
  }
  responses.set(`${PREFIX}/actions/runs/100/attempts/1/jobs?per_page=100`, { total_count: 4, jobs });
  responses.set(`${PREFIX}/actions/runs/100/artifacts?per_page=100`, { total_count: 6, artifacts });
  const calls = [];
  const api = async endpoint => { calls.push(endpoint); if (!responses.has(endpoint)) throw Error(`Unexpected endpoint ${endpoint}`);
    return structuredClone(responses.get(endpoint)); };
  const selection = () => { const bytes = Buffer.from(canonical(value)); return [bytes, sha256(bytes), api, NOW]; };
  return { value, run, caller, callee, jobs, artifacts, responses, calls, api, selection, refreshWorkflows };
}
