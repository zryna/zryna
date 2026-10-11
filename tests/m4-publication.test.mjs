import assert from 'node:assert/strict';
import test from 'node:test';
import { createHash } from 'node:crypto';
import { loadPublicationWorkflowInventory } from '../scripts/m4/publication-workflows.mjs';
import { authenticatePublicationFoundation } from '../scripts/m4/verify-publication.mjs';
import { downloadPublicationArchive } from '../scripts/m4/retain-publication.mjs';
import { digest } from '../scripts/m4/registry-files.mjs';

const source = { commit: 'a'.repeat(40), tree: 'b'.repeat(40), ref: 'refs/heads/main' };
const workflowSource = {
  '.github/workflows/ci.yml': `name: CI
on:
  workflow_dispatch:
jobs:
  foundation:
    name: foundation
    runs-on: ubuntu-latest
  browser:
    name: browser parent
    uses: ./.github/workflows/browser-conformance.yml
`,
  '.github/workflows/browser-conformance.yml': `name: Browser
on:
  workflow_call:
jobs:
  browser:
    name: browser (\${{ matrix.os }})
    strategy:
      matrix:
        os: [linux, windows]
    runs-on: ubuntu-latest
`,
};
function workflowApi(sources) {
  return async endpoint => {
    const file = endpoint.split('/contents/')[1]?.split('?')[0];
    assert(file && Object.hasOwn(sources, file), 'exact requested workflow fixture');
    const bytes = Buffer.from(sources[file]);
    return { type: 'file', path: file, encoding: 'base64', size: bytes.length,
      content: bytes.toString('base64'), sha: createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex') };
  };
}
function fixture() {
  const expectations = { repository: 'zryna/zryna', source, jobs: [{ runId: 77, attempt: 2, event: 'workflow_dispatch' }] };
  const run = { id: 77, run_attempt: 2, repository: { full_name: 'zryna/zryna' }, path: '.github/workflows/ci.yml',
    head_branch: 'main', head_sha: source.commit, event: 'workflow_dispatch', status: 'completed', conclusion: 'success' };
  const names = ['foundation', 'browser parent / browser (linux)', 'browser parent / browser (windows)'];
  const jobs = { total_count: names.length, jobs: names.map(name => ({ name,
    run_id: 77, run_attempt: 2, head_sha: source.commit, status: 'completed', conclusion: 'success' })) };
  const responses = [ { object: { sha: source.commit } }, { sha: source.commit, tree: { sha: source.tree } }, run, jobs ];
  const sources = { ...workflowSource };
  const api = async endpoint => endpoint.includes('/contents/') ? workflowApi(sources)(endpoint)
    : structuredClone(responses[endpoint.endsWith('main') ? 0 : endpoint.includes('/git/commits/') ? 1 : endpoint.includes('/jobs?') ? 3 : 2]);
  const readWorkflow = file => Buffer.from(sources[file]);
  return { expectations, responses, sources, readWorkflow, api };
}

test('publication independently requires completed full current-main CI, every required job and exact attempt', async () => {
  const good = fixture();
  await authenticatePublicationFoundation(good.expectations, good.api, '/independent-fixture', good.readWorkflow);
  for (const mutate of [
    f => { f.responses[0].object.sha = 'c'.repeat(40); }, f => { f.responses[1].tree.sha = 'c'.repeat(40); },
    f => { f.responses[2].status = 'in_progress'; }, f => { f.responses[2].conclusion = 'failure'; },
    f => { f.responses[2].head_branch = 'topic'; }, f => { f.responses[2].head_sha = 'c'.repeat(40); },
    f => { f.responses[2].run_attempt = 1; }, f => { f.responses[2].event = 'pull_request'; },
    f => { f.responses[2].path = '.github/workflows/documentation.yml'; },
    f => { f.responses[3].jobs[0].conclusion = 'skipped'; }, f => { f.responses[3].jobs[0].run_attempt = 1; },
    f => { f.responses[3].jobs.pop(); }, f => { f.responses[3].jobs[0].name = f.responses[3].jobs[1].name; },
    f => { f.responses[3].total_count++; },
    f => { f.responses[3].jobs.push({ ...f.responses[3].jobs[0], name: 'unknown skipped source job', conclusion: 'skipped' }); f.responses[3].total_count++; },
    f => { f.sources['.github/workflows/ci.yml'] += '  future-required:\n    name: future required\n    runs-on: ubuntu-latest\n'; },
  ]) { const bad = fixture(); mutate(bad); await assert.rejects(() => authenticatePublicationFoundation(bad.expectations, bad.api, '/independent-fixture', bad.readWorkflow)); }
});

test('artifact transport rejects changed archive bytes and unsafe storage redirects without forwarding authentication', async () => {
  const bytes = Buffer.from('independent archive identity');
  const artifact = { id: 12, bytes: bytes.length, sha256: digest(bytes) };
  assert((await downloadPublicationArchive('zryna/zryna', artifact, {
    request: async () => new Response(bytes), token: 'test-only-credential',
  })).equals(bytes));
  await assert.rejects(() => downloadPublicationArchive('zryna/zryna', artifact, { request: async () => new Response('wrong') }));
  for (const location of ['http://example.com/a', 'https://example.com/a', 'https://user:pass@x.blob.core.windows.net/a', 'https://x.blob.core.windows.net:123/a']) {
    await assert.rejects(() => downloadPublicationArchive('zryna/zryna', artifact, {
      request: async () => new Response(null, { status: 302, headers: { location } }),
    }));
  }
  let count = 0;
  await downloadPublicationArchive('zryna/zryna', artifact, { token: 'test-only-credential', request: async (url, options) => {
    count++;
    if (count === 1) {
      assert.equal(options.headers.Authorization, 'Bearer test-only-credential');
      return new Response(null, { status: 302, headers: { location: 'https://test.blob.core.windows.net/archive?sig=fixture' } });
    }
    assert.equal(options.headers, undefined); assert.equal(options.redirect, 'error');
    return new Response(bytes);
  } });
  assert.equal(count, 2);
});

import { authenticateDocsArtifact } from '../scripts/m4/verify-docs-artifact.mjs';

test('published docs archive authentication rejects stale run, source, digest, expiry and previous-attempt bytes', async () => {
  const expected = { docsRunId: 88, source };
  const descriptor = { id: 14, bytes: 100, sha256: 'c'.repeat(64), expiresAt: '2030-01-01T00:00:00Z' };
  const job = { started_at: '2026-10-11T06:00:00Z', completed_at: '2026-10-11T06:10:00Z' };
  const name = `zryna-docs-next-${source.commit}-${'d'.repeat(64)}`;
  const artifact = { id: 14, name, size_in_bytes: 100, digest: `sha256:${descriptor.sha256}`,
    workflow_run: { id: 88, head_sha: source.commit }, expires_at: descriptor.expiresAt,
    expired: false, created_at: '2026-10-11T06:05:00Z' };
  const now = Date.parse('2026-10-11T07:00:00Z');
  await authenticateDocsArtifact(expected, descriptor, name, async () => artifact, job, now);
  for (const mutate of [
    a => { a.id++; }, a => { a.name = 'latest'; }, a => { a.digest = `sha256:${'e'.repeat(64)}`; },
    a => { a.size_in_bytes++; }, a => { a.workflow_run.id--; }, a => { a.workflow_run.head_sha = 'f'.repeat(40); },
    a => { a.expired = true; }, a => { a.expires_at = '2020-01-01T00:00:00Z'; },
    a => { a.created_at = '2026-10-11T05:00:00Z'; }, a => { a.created_at = 'invalid'; },
    a => { a.created_at = '2026-10-11T08:00:00Z'; },
  ]) { const altered = structuredClone(artifact); mutate(altered);
    await assert.rejects(() => authenticateDocsArtifact(expected, descriptor, name, async () => altered, job, now));
  }
});

import { validatePublicationContext } from '../scripts/m4/verify-publication.mjs';

function mainContextFixture() {
  const workflows = ['.github/workflows/browser-conformance.yml', '.github/workflows/ci.yml', '.github/workflows/installed-command-h1.yml'];
  const hosts = [
    ['core', 'ubuntu-latest', 'm4 (ubuntu-latest)', 'm4-platform', workflows[1]],
    ['core', 'windows-latest', 'm4 (windows-latest)', 'm4-platform', workflows[1]],
    ['browser', 'ubuntu-22.04', 'm4 browser / pinned browser (ubuntu-22.04)', 'conformance', workflows[0]],
    ['browser', 'windows-2022', 'm4 browser / pinned browser (windows-2022)', 'conformance', workflows[0]],
    ['installed', 'ubuntu-24.04', 'm4 installed / fresh installed H1 (ubuntu-24.04)', 'qualify', workflows[2]],
    ['installed', 'windows-2022', 'm4 installed / fresh installed H1 (windows-2022)', 'qualify', workflows[2]],
  ];
  const expectations = { format: 'zryna.m4-hosted-expectations.v1', repository: 'zryna/zryna', source,
    registrySha256: 'c'.repeat(64), aggregate: { jobId: 99, checkRunId: 199, checkUrl: 'https://github.com/zryna/zryna/actions/runs/77/job/99' }, workflowSources: workflows.map(path => ({ path, bytes: 100, sha256: 'd'.repeat(64) })),
    eventSource: { commit: source.commit, tree: source.tree, parents: [] },
    workflowExecution: { commit: source.commit, tree: source.tree, parents: [] },
    jobs: hosts.map(([lane, baseline, jobName, jobKey, workflowPath], index) => ({ lane, baseline, jobName, jobKey, workflowPath,
      runId: 77, attempt: 2, jobId: 100 + index, runWorkflowPath: workflows[1], workflowSha256: 'd'.repeat(64),
      event: 'workflow_dispatch', headCommit: source.commit, checkRunId: 200 + index,
      checkUrl: `https://github.com/zryna/zryna/actions/runs/77/job/${100 + index}`,
      artifact: { id: 300 + index, name: `m4-${lane}-${baseline}-${source.commit}-77-2`, bytes: 100,
        sha256: 'e'.repeat(64), expiresAt: '2030-01-01T00:00:00Z' },
      host: { os: baseline.startsWith('ubuntu') ? 'linux' : 'windows', architecture: 'x86_64',
        distribution: 'observed-fixture', version: 'observed-fixture', kernel: 'observed-fixture', runnerImage: baseline, runnerImageVersion: 'observed-fixture' },
      tools: [{ name: 'independent-fixture-tool', sha256: 'f'.repeat(64) }], materials: [], evidenceDirectory: '/independent-fixture/evidence',
    })) };
  const environment = { GITHUB_ACTIONS: 'true', GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_REF: source.ref, GITHUB_SHA: source.commit, GITHUB_RUN_ID: '88', GITHUB_RUN_ATTEMPT: '1', M4_CI_RUN_ID: '77', M4_CI_ATTEMPT: '2' };
  return { expectations, environment, checkout: structuredClone(source) };
}

test('official next publication rejects PR source relabeling, changed main context and mismatched workflow execution', () => {
  const f = mainContextFixture(); validatePublicationContext(f.expectations, f.environment, f.checkout);
  for (const mutate of [
    f => { f.environment.GITHUB_ACTIONS = 'false'; }, f => { f.environment.GITHUB_EVENT_NAME = 'pull_request'; },
    f => { f.environment.GITHUB_REF = 'refs/heads/topic'; }, f => { f.expectations.source = { ...source, ref: 'refs/heads/topic' }; },
    f => { f.environment.GITHUB_SHA = '0'.repeat(40); }, f => { f.checkout.tree = '0'.repeat(40); },
    f => { f.environment.M4_CI_ATTEMPT = '1'; }, f => { f.environment.M4_CI_RUN_ID = '9007199254740993'; },
    f => { f.environment.GITHUB_RUN_ID = '0'; }, f => { f.expectations.workflowExecution.commit = '0'.repeat(40); },
    f => { f.expectations.eventSource.commit = '0'.repeat(40); },
    f => { for (const job of f.expectations.jobs) job.event = 'pull_request'; },
  ]) { const bad = mainContextFixture(); mutate(bad);
    assert.throws(() => validatePublicationContext(bad.expectations, bad.environment, bad.checkout));
  }
});

test('source-derived inventory rejects dynamic matrices, unreviewed conditions, remote calls, cycles and source substitution', async () => {
  const inventory = sources => loadPublicationWorkflowInventory({ repository: 'zryna/zryna', source,
    root: '/independent-fixture', api: workflowApi(sources), readWorkflow: file => Buffer.from(sources[file]) });
  const expected = await inventory({ ...workflowSource });
  assert.deepEqual(expected.jobs.map(job => job.name), ['foundation', 'browser parent / browser (linux)', 'browser parent / browser (windows)']);
  for (const mutate of [
    s => { s['.github/workflows/ci.yml'] = s['.github/workflows/ci.yml'].replace('name: foundation', "name: foundation\n    if: github.event_name == 'push'"); },
    s => { s['.github/workflows/ci.yml'] = s['.github/workflows/ci.yml'].replace('./.github/workflows/browser-conformance.yml', 'owner/repo/.github/workflows/browser.yml@main'); },
    s => { s['.github/workflows/browser-conformance.yml'] += '  cycle:\n    uses: ./.github/workflows/ci.yml\n'; },
    s => { s['.github/workflows/browser-conformance.yml'] = s['.github/workflows/browser-conformance.yml'].replace('os: [linux, windows]', 'os: ${{ fromJSON(inputs.hosts) }}'); },
    s => { s['.github/workflows/browser-conformance.yml'] = s['.github/workflows/browser-conformance.yml'].replace('os: [linux, windows]', 'os: [linux, windows]\n        exclude: [{ os: windows }]'); },
    s => { s['.github/workflows/browser-conformance.yml'] = s['.github/workflows/browser-conformance.yml'].replace('matrix.os', 'github.actor'); },
    s => { s['.github/workflows/browser-conformance.yml'] = s['.github/workflows/browser-conformance.yml'].replace('os: [linux, windows]', 'os: [linux, linux]'); },
  ]) { const sources = { ...workflowSource }; mutate(sources); await assert.rejects(() => inventory(sources)); }
  await assert.rejects(() => loadPublicationWorkflowInventory({ repository: 'zryna/zryna', source,
    root: '/independent-fixture', api: workflowApi(workflowSource), readWorkflow: file => Buffer.from(workflowSource[file] + '# substituted bytes\n') }));
});
