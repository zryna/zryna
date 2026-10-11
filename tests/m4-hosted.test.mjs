import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, writeFileSync, symlinkSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { authenticateJob, loadHostedRegistry } from '../scripts/m4/verify-hosted.mjs';
import { calculateM4Coverage, workspaceRoot } from '../scripts/check-m4-conformance.mjs';
import { zipEntries, verifyRetainedArchive } from '../scripts/m4/hosted-archive.mjs';
import { validateExpectations, HOSTED_LANES } from '../scripts/m4/hosted-expectations.mjs';
import { verifyM4CIResult } from '../scripts/m4/verify-ci-result.mjs';
import { digest } from '../scripts/m4/registry-files.mjs';
import { boundedResponse, githubApi } from '../scripts/m4/hosted-api.mjs';
import { authenticateWorkflow } from '../scripts/m4/hosted-workflow.mjs';
import { authenticateAggregate } from '../scripts/m4/hosted-aggregate.mjs';
import { createHash } from 'node:crypto';
import { deflateRawSync } from 'node:zlib';

function expectations() {
  const repository = 'fixture/compiler';
  const commit = 'a'.repeat(40);
  const workflowSources = ['browser-conformance', 'ci', 'installed-command-h1'].map(name => ({
    path: `.github/workflows/${name}.yml`, bytes: 1, sha256: 'b'.repeat(64),
  }));
  return { format: 'zryna.m4-hosted-expectations.v1', repository,
    source: { commit, tree: 'c'.repeat(40), ref: 'refs/heads/candidate' },
    workflowExecution: { commit, tree: 'c'.repeat(40), parents: ['f'.repeat(40)] },
    eventSource: { commit, tree: 'c'.repeat(40), parents: ['f'.repeat(40)] },
    aggregate: { jobId: 900, checkRunId: 901, checkUrl: `https://github.com/${repository}/runs/901` },
    registrySha256: 'd'.repeat(64), workflowSources,
    jobs: HOSTED_LANES.map(([lane, baseline, nativeName, jobKey], index) => ({
      lane, baseline, runId: 10, attempt: 2, jobId: 100 + index,
      jobName: lane === 'core' ? nativeName : `m4 ${lane} / ${nativeName}`, jobKey,
      runWorkflowPath: '.github/workflows/ci.yml', workflowPath: lane === 'core' ? '.github/workflows/ci.yml'
        : lane === 'browser' ? '.github/workflows/browser-conformance.yml' : '.github/workflows/installed-command-h1.yml',
      workflowSha256: 'b'.repeat(64), event: 'pull_request', headCommit: commit,
      checkRunId: 200 + index, checkUrl: `https://github.com/${repository}/runs/${200 + index}`,
      artifact: { id: 300 + index, name: `m4-${lane}-${baseline}-${commit}-10-2`,
        sha256: 'e'.repeat(64), bytes: 1, expiresAt: '2030-01-02T00:00:00Z' },
      host: { os: baseline.startsWith('ubuntu') ? 'linux' : 'windows', architecture: 'x86_64',
        distribution: 'observed', version: 'observed', kernel: 'observed', runnerImage: 'observed', runnerImageVersion: 'observed' },
      tools: [{ name: 'synthetic-validator-fixture' }], materials: [], evidenceDirectory: '/original/evidence',
    })),
  };
}

test('publication registry loader preserves source authority and rejects changed registry bytes', () => {
  const registry = loadHostedRegistry(workspaceRoot);
  assert.equal(calculateM4Coverage(registry).contractComplete, true);
  assert.throws(() => calculateM4Coverage(structuredClone(registry)), /independently validated contract authority/);
  const root = mkdtempSync(join(tmpdir(), 'zryna-m4-hosted-registry-'));
  try {
    mkdirSync(join(root, 'tests'));
    const changed = structuredClone(registry);
    changed.sources[0].sha256 = '0'.repeat(64);
    writeFileSync(join(root, 'tests/m4-conformance-v1.json'), `${JSON.stringify(changed, null, 2)}\n`);
    assert.throws(() => loadHostedRegistry(root), /registry digest mismatch/);
  } finally { rmSync(root, { recursive: true, force: true }); }
});
function metadata(value, job) {
  const prefix = `/repos/${value.repository}`;
  const created = '2030-01-01T00:01:00Z';
  return {
    [`${prefix}/actions/runs/${job.runId}/attempts/${job.attempt}`]: {
      id: job.runId, run_attempt: job.attempt, repository: { full_name: value.repository },
      path: job.runWorkflowPath, head_sha: job.headCommit, head_branch: 'candidate', event: job.event, status: 'completed', conclusion: 'success',
    },
    [`${prefix}/actions/jobs/${job.jobId}`]: {
      id: job.jobId, run_id: job.runId, run_attempt: job.attempt, name: job.jobName, head_sha: job.headCommit,
      status: 'completed', conclusion: 'success', check_run_url: `https://api.github.com${prefix}/check-runs/${job.checkRunId}`,
      started_at: '2030-01-01T00:00:00Z', completed_at: '2030-01-01T00:02:00Z',
    },
    [`${prefix}/check-runs/${job.checkRunId}`]: { id: job.checkRunId, app: { slug: 'github-actions' },
      head_sha: job.headCommit, status: 'completed', conclusion: 'success', html_url: job.checkUrl },
    [`${prefix}/actions/artifacts/${job.artifact.id}`]: {
      id: job.artifact.id, name: job.artifact.name, size_in_bytes: job.artifact.bytes,
      digest: `sha256:${job.artifact.sha256}`, workflow_run: { id: job.runId, head_sha: job.headCommit },
      created_at: created, expires_at: job.artifact.expiresAt, expired: false,
    },
  };
}
test('hosted expectations require exactly six separately identified pinned host lanes', () => {
  const valid = expectations();
  assert.equal(validateExpectations(valid), valid);
  for (const mutate of [
    x => x.jobs.pop(), x => x.jobs.push(x.jobs[0]), x => { x.jobs[1] = structuredClone(x.jobs[0]); },
    x => { x.jobs[0].baseline = 'linux'; }, x => { x.jobs[0].headCommit = 'latest'; },
    x => { x.jobs[0].attempt++; }, x => { x.jobs[0].workflowSha256 = 'f'.repeat(64); },
    x => { x.jobs[0].host.runnerImageVersion = ''; }, x => { x.jobs[0].host.architecture = 'arm64'; },
    x => { x.jobs[0].artifact.name = 'green'; }, x => { x.jobs[0].artifact.id = x.jobs[1].artifact.id; },
    x => { x.workflowSources.pop(); }, x => { x.source.commit = x.source.tree; x.extra = true; },
  ]) { const changed = structuredClone(valid); mutate(changed); assert.throws(() => validateExpectations(changed)); }
});
test('authenticated metadata rejects mismatched run, attempt, checkout-independent job and artifact identities', async () => {
  const value = expectations();
  const job = value.jobs[0];
  const original = metadata(value, job);
  const now = Date.parse('2030-01-01T00:03:00Z');
  const verify = entries => authenticateJob({ expectations: value, job, api: async key => entries[key], now, runState: 'completed' });
  await verify(original);
  for (const [endpoint, fields] of Object.entries(original)) {
    for (const [key, datum] of Object.entries(fields)) {
      const changed = structuredClone(original);
      changed[endpoint][key] = typeof datum === 'number' ? datum + 1
        : typeof datum === 'boolean' ? !datum : typeof datum === 'object' ? {} : 'wrong';
      await assert.rejects(verify(changed), undefined, key);
    }
  }
  await assert.rejects(authenticateJob({ expectations: value, job, api: async key => original[key],
    now: Date.parse(job.artifact.expiresAt), runState: 'completed' }));
});
test('optional aggregate distinguishes required success from precisely routed skips', () => {
  verifyM4CIResult('true', 'success', 'success', ['success', 'success', 'success']);
  verifyM4CIResult('false', 'success', 'success', ['skipped', 'skipped', 'skipped']);
  for (const required of ['true', 'false', '', 'undefined']) {
    for (const outcome of ['failure', 'cancelled', 'skipped', '', 'success']) {
      if ((required === 'true' && outcome === 'success') || (required === 'false' && outcome === 'skipped')) continue;
      assert.throws(() => verifyM4CIResult(required, 'success', 'success', [outcome, outcome, outcome]));
    }
  }
  for (const foundation of ['failure', 'skipped', 'cancelled']) {
    assert.throws(() => verifyM4CIResult('true', 'success', foundation, ['success', 'success', 'success']));
  }
});
function crc32(bytes) {
  let crc = 0xffffffff;
  for (const value of bytes) {
    crc ^= value;
    for (let i = 0; i < 8; i++) crc = (crc >>> 1) ^ ((crc & 1) ? 0xedb88320 : 0);
  }
  return (crc ^ 0xffffffff) >>> 0;
}
function storedZip(entries, options = {}) {
  const method = options.method ?? 0;
  const flags = options.descriptor ? 8 : 0;
  const locals = [];
  const centrals = [];
  let offset = 0;
  for (const [path, content] of entries) {
    const name = Buffer.from(path);
    const bytes = Buffer.from(content);
    const encoded = method === 8 ? deflateRawSync(bytes) : bytes;
    const compressed = options.trailing ? Buffer.concat([encoded, Buffer.from('extra')]) : encoded;
    const local = Buffer.alloc(30);
    local.writeUInt32LE(0x04034b50); local.writeUInt16LE(20, 4);
    local.writeUInt16LE(flags, 6); local.writeUInt16LE(method, 8);
    local.writeUInt32LE(crc32(bytes), 14); local.writeUInt32LE(compressed.length, 18);
    local.writeUInt32LE(bytes.length, 22); local.writeUInt16LE(name.length, 26);
    const central = Buffer.alloc(46);
    central.writeUInt32LE(0x02014b50); central.writeUInt16LE(20, 4); central.writeUInt16LE(20, 6);
    central.writeUInt16LE(flags, 8); central.writeUInt16LE(method, 10);
    central.writeUInt32LE(crc32(bytes), 16); central.writeUInt32LE(compressed.length, 20);
    central.writeUInt32LE(bytes.length, 24); central.writeUInt16LE(name.length, 28); central.writeUInt32LE(offset, 42);
    const descriptor = Buffer.alloc(options.descriptor ? 16 : 0);
    if (options.descriptor) {
      descriptor.writeUInt32LE(0x08074b50); descriptor.writeUInt32LE(crc32(bytes), 4);
      descriptor.writeUInt32LE(compressed.length, 8); descriptor.writeUInt32LE(bytes.length, 12);
    }
    locals.push(local, name, compressed, descriptor); centrals.push(central, name);
    offset += local.length + name.length + compressed.length + descriptor.length;
  }
  const local = Buffer.concat(locals); const central = Buffer.concat(centrals); const end = Buffer.alloc(22);
  end.writeUInt32LE(0x06054b50); end.writeUInt16LE(entries.length, 8); end.writeUInt16LE(entries.length, 10);
  end.writeUInt32LE(central.length, 12); end.writeUInt32LE(local.length, 16);
  return Buffer.concat([local, central, end]);
}
test('retained artifact validation binds every untruncated file to the externally pinned archive', () => {
  const directory = mkdtempSync(join(tmpdir(), 'm4-hosted-'));
  try {
    mkdirSync(join(directory, 'evidence'));
    const zip = storedZip([['receipt.json', '{}\n'], ['proof.log', 'actual fixture log\n']]);
    const file = join(directory, 'artifact.zip'); writeFileSync(file, zip);
    writeFileSync(join(directory, 'evidence', 'receipt.json'), '{}\n');
    writeFileSync(join(directory, 'evidence', 'proof.log'), 'actual fixture log\n');
    const expected = { bytes: zip.length, sha256: digest(zip) };
    assert.deepEqual(verifyRetainedArchive(file, directory, expected), ['receipt.json', 'proof.log']);
    writeFileSync(join(directory, 'evidence', 'proof.log'), 'actual fixture');
    assert.throws(() => verifyRetainedArchive(file, directory, expected));
    assert.throws(() => verifyRetainedArchive(file, directory, { ...expected, sha256: '0'.repeat(64) }));
    writeFileSync(join(directory, 'evidence', 'extra.log'), 'extra');
    assert.throws(() => verifyRetainedArchive(file, directory, expected));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
test('ZIP parser independently rejects traversal, collisions, corruption and unaccounted bytes', () => {
  const original = storedZip([['receipt.json', '{}\n']]);
  assert.equal(zipEntries(original)[0].sha256, digest(Buffer.from('{}\n')));
  for (const invalid of [storedZip([['../outside', 'x']]), storedZip([['a', 'x'], ['A', 'y']]),
    original.subarray(0, -1), Buffer.concat([original, Buffer.from('extra')]), Buffer.from('not zip')]) {
    assert.throws(() => zipEntries(invalid));
  }
  const corrupt = Buffer.from(original); corrupt[30 + Buffer.byteLength('receipt.json')] ^= 1;
  assert.throws(() => zipEntries(corrupt));
});
test('retained file and linked ancestor cannot be substituted for authentic archive contents', {
  skip: process.platform === 'win32' ? 'POSIX symlink fixture; Windows reparse checks belong to registry suite' : false,
}, () => {
  const directory = mkdtempSync(join(tmpdir(), 'm4-links-'));
  try {
    mkdirSync(join(directory, 'evidence')); writeFileSync(join(directory, 'outside'), '{}\n');
    symlinkSync(join(directory, 'outside'), join(directory, 'evidence', 'receipt.json'));
    const zip = storedZip([['receipt.json', '{}\n']]); const file = join(directory, 'artifact.zip'); writeFileSync(file, zip);
    assert.throws(() => verifyRetainedArchive(file, directory, { bytes: zip.length, sha256: digest(zip) }));
  } finally { rmSync(directory, { recursive: true, force: true }); }
});
test('API requests reject mutable selectors and bound actual streamed bytes before parsing', async () => {
  for (const path of ['https://attacker.invalid', '/repos/fixture/compiler/actions/runs/../latest', '/user']) {
    await assert.rejects(githubApi(undefined)(path));
  }
  await assert.rejects(boundedResponse(new Response('too large'), 2));
  await assert.rejects(boundedResponse(new Response('{}', { status: 403 }), 2));
  assert.equal((await boundedResponse(new Response('{}'), 2)).toString(), '{}');
});
test('immutable workflow/event source authentication keeps a synthetic merge distinct from the candidate', async () => {
  const value = expectations();
  const bytes = Buffer.from('reviewed workflow\n');
  value.workflowSources.forEach(item => { item.bytes = bytes.length; item.sha256 = digest(bytes); });
  value.workflowExecution = { commit: '1'.repeat(40), tree: '2'.repeat(40), parents: ['f'.repeat(40), value.source.commit] };
  value.eventSource = structuredClone(value.workflowExecution);
  const prefix = `/repos/${value.repository}`;
  const responses = {
    [`${prefix}/git/commits/${value.source.commit}`]: { sha: value.source.commit,
      tree: { sha: value.source.tree } },
    [`${prefix}/git/commits/${value.workflowExecution.commit}`]: { sha: value.workflowExecution.commit,
      tree: { sha: value.workflowExecution.tree }, parents: value.workflowExecution.parents.map(sha => ({ sha })) },
  };
  for (const item of value.workflowSources) responses[`${prefix}/contents/${item.path}?ref=${value.workflowExecution.commit}`] = {
    type: 'file', path: item.path, encoding: 'base64', content: bytes.toString('base64'), size: bytes.length,
    sha: createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex'),
  };
  const run = { id: value.jobs[0].runId, run_attempt: value.jobs[0].attempt, head_sha: value.source.commit,
    referenced_workflows: ['browser-conformance', 'installed-command-h1', 'portable-setup'].map(name => ({
      path: `${value.repository}/.github/workflows/${name}.yml@refs/pull/42/merge`, ref: 'refs/pull/42/merge',
      sha: value.workflowExecution.commit,
    })) };
  await authenticateWorkflow(value, async path => responses[path], run);
  assert.notEqual(value.source.commit, value.workflowExecution.commit);
  for (const [endpoint, object] of Object.entries(responses)) for (const key of Object.keys(object)) {
    const changed = structuredClone(responses); changed[endpoint][key] = null;
    await assert.rejects(authenticateWorkflow(value, async path => changed[path], run));
  }
  const unrelated = structuredClone(value);
  unrelated.jobs[0].headCommit = '9'.repeat(40);
  await assert.rejects(authenticateWorkflow(unrelated, async path => responses[path], run));
  for (const mutate of [
    x => { delete x.referenced_workflows; }, x => { x.referenced_workflows.shift(); },
    x => { x.referenced_workflows.push(structuredClone(x.referenced_workflows[0])); },
    x => { x.referenced_workflows[0].sha = '9'.repeat(40); },
    x => { x.referenced_workflows[0].ref = 'refs/heads/unrelated'; },
    x => { x.referenced_workflows[0].path = 'attacker/compiler/.github/workflows/browser-conformance.yml@refs/pull/42/merge'; },
  ]) { const changed = structuredClone(run); mutate(changed); await assert.rejects(authenticateWorkflow(value, async path => responses[path], changed)); }
});
test('M4 aggregate must be the successful completed exact-run/attempt GitHub Actions check', async () => {
  const value = expectations();
  const prefix = `/repos/${value.repository}`;
  const responses = {
    [`${prefix}/actions/jobs/900`]: { id: 900, run_id: 10, run_attempt: 2, head_sha: value.source.commit,
      name: 'm4', status: 'completed', conclusion: 'success', check_run_url: `https://api.github.com${prefix}/check-runs/901` },
    [`${prefix}/check-runs/901`]: { id: 901, name: 'm4', app: { slug: 'github-actions' }, head_sha: value.source.commit,
      status: 'completed', conclusion: 'success', html_url: value.aggregate.checkUrl },
  };
  await authenticateAggregate(value, async path => responses[path]);
  for (const [endpoint, object] of Object.entries(responses)) for (const key of Object.keys(object)) {
    const changed = structuredClone(responses); changed[endpoint][key] = null;
    await assert.rejects(authenticateAggregate(value, async path => changed[path]));
  }
});
test('ZIP independently rejects local/central substitution, linked entries, bound overflow and descriptor corruption', () => {
  const original = storedZip([['proof.log', 'proof\n']]);
  const central = original.readUInt32LE(original.length - 6);
  const invalid = [];
  let bytes = Buffer.from(original); bytes[30] ^= 32; invalid.push(bytes);
  bytes = Buffer.from(original); bytes.writeUInt32LE(0xa0000000, central + 38); invalid.push(bytes);
  bytes = Buffer.from(original); bytes.writeUInt32LE(256 * 1024 * 1024 + 1, central + 24); invalid.push(bytes);
  const descriptor = storedZip([['proof.log', 'proof\n']], { descriptor: true });
  assert.equal(zipEntries(descriptor)[0].size, 6);
  bytes = Buffer.from(descriptor); bytes[30 + Buffer.byteLength('proof.log') + 6 + 4] ^= 1; invalid.push(bytes);
  const deflated = storedZip([['proof.log', 'proof\n']], { method: 8 });
  assert.equal(zipEntries(deflated)[0].sha256, digest(Buffer.from('proof\n')));
  invalid.push(storedZip([['proof.log', 'proof\n']], { method: 8, trailing: true }));
  invalid.push(storedZip(Array.from({ length: 257 }, (_, i) => [`entry-${i}.log`, ''])));
  for (const archive of invalid) assert.throws(() => zipEntries(archive));
});
