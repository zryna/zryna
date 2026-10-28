// Platform custody and complete bounded transport validation; no extraction or approval.
import assert from 'node:assert/strict';
import { github } from '../m6/compiler-release/github.mjs';
import { downloadPublicationArchive } from '../m4/retain-publication.mjs';
import { zipEntries } from '../m4/hosted-archive.mjs';
import { decodeTar } from '../distribution/archive-tar.mjs';
import { PERFORMANCE_REPOSITORY_ID } from './cli-performance-hosted-expectations.mjs';
import { sha256 } from './input.mjs';
import { join } from 'node:path';
import { ORIGINALS, save } from './lifecycle-private-io.mjs';

export function verifyCaptureMetadata(e, run, jobs, artifact, check, head) {
  for (const key of ['run', 'attempt', 'workflowId', 'jobId', 'checkRunId', 'id', 'bytes']) {
    assert(Number.isSafeInteger(e[key]) && e[key] > 0);
  }
  assert(e.bytes <= 768 * 1024 ** 2); assert.match(e.sha256, /^[a-f0-9]{64}$/u);
  assert.equal(run.id, e.run); assert.equal(run.run_attempt, e.attempt); assert.equal(run.workflow_id, e.workflowId);
  for (const repo of [run.repository, run.head_repository]) {
    assert.equal(repo.id, PERFORMANCE_REPOSITORY_ID); assert.equal(repo.full_name, 'zryna/zryna'); assert.equal(repo.fork, false);
  }
  assert.equal(run.path, '.github/workflows/native-host-facts.yml'); assert.equal(run.event, 'workflow_dispatch');
  assert.equal(run.head_sha, head); assert.equal(run.head_branch, e.branch);
  assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  assert.equal(jobs.total_count, 4); assert.equal(jobs.jobs.length, 4);
  assert.equal(new Set(jobs.jobs.map(job => job.id)).size, 4);
  const job = jobs.jobs.find(value => value.id === e.jobId); assert(job);
  for (const [key, value] of Object.entries({ name: 'lifecycle', run_id: e.run, run_attempt: e.attempt,
    head_sha: head, status: 'completed', conclusion: 'success' })) assert.equal(job[key], value);
  const step = job.steps.filter(value => value.name === 'Capture immutable lifecycle inputs'); assert.equal(step.length, 1);
  assert.equal(step[0].status, 'completed'); assert.equal(step[0].conclusion, 'success');
  assert.equal(job.check_run_url, `https://api.github.com/repos/zryna/zryna/check-runs/${e.checkRunId}`);
  assert.equal(check.id, e.checkRunId); assert.equal(check.app.slug, 'github-actions'); assert.equal(check.head_sha, head);
  assert.equal(check.check_suite.id, run.check_suite_id); assert.equal(check.status, 'completed'); assert.equal(check.conclusion, 'success');
  assert.equal(artifact.id, e.id); assert.equal(artifact.name, `lifecycle-capture-${head}-${e.run}-${e.attempt}`);
  assert.equal(artifact.size_in_bytes, e.bytes); assert.equal(artifact.digest, 'sha256:' + e.sha256); assert.equal(artifact.expired, false);
  assert.equal(artifact.expires_at, e.expiresAt); assert(Date.parse(e.expiresAt) > Date.now());
  for (const [key, value] of Object.entries({ id: e.run, repository_id: PERFORMANCE_REPOSITORY_ID,
    head_repository_id: PERFORMANCE_REPOSITORY_ID, head_sha: head, head_branch: e.branch })) assert.equal(artifact.workflow_run[key], value);
  const start = Date.parse(job.started_at), end = Date.parse(job.completed_at), created = Date.parse(artifact.created_at);
  assert(start <= created && created <= end && end <= Date.now());
}

export async function acquireCapture(e, head, work) {
  const api = github();
  const prefix = '/repos/zryna/zryna';
  try {
  for (const key of ['run', 'attempt', 'jobId', 'checkRunId', 'id']) assert(Number.isSafeInteger(e[key]) && e[key] > 0);
  const run = await api.request(`${prefix}/actions/runs/${e.run}/attempts/${e.attempt}`);
  const jobs = await api.request(`${prefix}/actions/runs/${e.run}/attempts/${e.attempt}/jobs?per_page=100`);
  const artifact = await api.request(`${prefix}/actions/artifacts/${e.id}`);
  const check = await api.request(`${prefix}/check-runs/${e.checkRunId}`);
  verifyCaptureMetadata(e, run, jobs, artifact, check, head);
  const zip = await downloadPublicationArchive('zryna/zryna', e);
  save(join(work, 'artifact.zip'), zip);
  const entries = zipEntries(zip); // Complete CRC/type/path/framing pass before any consume/write.
  assert.deepEqual(entries, e.entries, 'independently selected complete outer inventory');
  const fixed = ['capture-main.tar.gz', 'capture-after.tar.gz', 'upstream-license', 'download-selection.json',
    'capture-complete.json', 'cc-command.json', 'cc-result.json', 'cc-stdout.raw', 'cc-stderr.raw'];
  assert(entries.every(row => fixed.includes(row.path) || /^binaries\/(compiler|test|supervisor)$/.test(row.path)
    || /^originals\/[a-z0-9-]+$/.test(row.path)), 'no extra transport members');
  const outer = new Map(); zipEntries(zip, (entry, data) => outer.set(entry.path, data));
  for (const path of [...fixed, ...['compiler', 'test', 'supervisor'].map(name => 'binaries/' + name),
    ...ORIGINALS.map(name => 'originals/' + name)]) assert(outer.has(path), 'complete fixed producer evidence');
  const files = [];
  for (const name of ['capture-main.tar.gz', 'capture-after.tar.gz']) {
    assert(outer.has(name));
    const decoded = await decodeTar(outer.get(name), 'capture', 0);
    for (const file of decoded) {
      assert.equal(file.path.startsWith('licenses-after/'), name === 'capture-after.tar.gz');
      files.push({ ...file, path: 'capture/' + file.path });
    }
  }
  const inventory = files.map(file => ({ path: file.path, bytes: file.data.length, sha256: sha256(file.data) }))
    .sort((a, b) => a.path < b.path ? -1 : 1);
  assert.equal(new Set(inventory.map(row => row.path.toLowerCase())).size, inventory.length);
  assert.deepEqual(inventory, e.captureFiles, 'complete original raw capture inventory');
  for (const [path, data] of outer) if (!fixed.slice(0, 2).includes(path)) files.push({ path, data, mode: 0o644 });
  const names = new Set(files.map(file => file.path.toLowerCase())); assert.equal(names.size, files.length);
  for (const file of files) {
    const parts = file.path.toLowerCase().split('/');
    for (let i = 1; i < parts.length; i++) assert(!names.has(parts.slice(0, i).join('/')), 'file/directory alias');
  }
  return files;
  } finally {
    api.captures.forEach((row, i) => save(join(work, 'platform-api-' + i + '.raw'), row.data));
    save(join(work, 'platform-api.json'), api.captures.map(({ data, ...row }) => row));
  }
}
