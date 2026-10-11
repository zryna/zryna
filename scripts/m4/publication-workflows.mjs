import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { join } from 'node:path';
import { parseDocument } from 'yaml';
import { digest, readStableFile } from './registry-files.mjs';

const MAX_JOBS = 100;
const MAX_WORKFLOWS = 16;
const WORKFLOW = /^\.github\/workflows\/[A-Za-z0-9_-]+\.ya?ml$/u;
const scalar = value => typeof value === 'string' && value.length > 0 && value.length <= 256
  && !value.includes('${{');

function matrixRows(matrix) {
  if (matrix === undefined) return [{}];
  assert(matrix && typeof matrix === 'object' && !Array.isArray(matrix), 'static matrix object required');
  const keys = Object.keys(matrix);
  assert(keys.length > 0 && !keys.includes('exclude'), 'explicit static matrix without exclusions required');
  if (keys.includes('include')) {
    assert.deepEqual(keys, ['include'], 'only standalone include matrices are supported');
    assert(Array.isArray(matrix.include) && matrix.include.length > 0 && matrix.include.length <= MAX_JOBS);
    return matrix.include.map(row => {
      assert(row && typeof row === 'object' && !Array.isArray(row) && Object.keys(row).length > 0);
      for (const [key, value] of Object.entries(row)) assert(/^[a-z][a-z0-9_]*$/u.test(key) && scalar(value), 'static matrix scalar');
      return row;
    });
  }
  let rows = [{}];
  for (const key of keys) {
    assert(/^[a-z][a-z0-9_]*$/u.test(key), 'supported matrix axis');
    const axis = matrix[key];
    assert(Array.isArray(axis) && axis.length > 0 && axis.every(scalar), 'bounded static string matrix axis');
    assert.equal(new Set(axis).size, axis.length, 'unique matrix values');
    assert(rows.length * axis.length <= MAX_JOBS, 'matrix expansion bound');
    rows = rows.flatMap(row => axis.map(value => ({ ...row, [key]: value })));
  }
  return rows;
}

function renderedName(template, row) {
  assert(typeof template === 'string' && template.length > 0 && template.length <= 512, 'explicit bounded job name');
  const name = template.replace(/\$\{\{\s*matrix\.([a-z][a-z0-9_]*)\s*\}\}/gu, (_, axis) => {
    assert(Object.hasOwn(row, axis), 'job name references an absent matrix axis');
    return row[axis];
  });
  assert(!name.includes('${{') && !name.includes('\n'), 'unsupported job-name expression');
  return name;
}

function requiredOnFullDispatch(condition) {
  if (condition === undefined || condition === 'always()') return;
  assert(typeof condition === 'string' && /^needs\.route-contracts\.outputs\.(?:diagnostics|distribution_release|package_release|provider_v4|wit|m4) == 'true'$/u.test(condition),
    'unsupported job condition; review its full-main required outcome explicitly');
  // The full dispatch contract requires every route-gated job to run successfully.
  // No YAML condition is used to excuse a missing/skipped hosted outcome.
}

export async function loadPublicationWorkflowInventory({ repository, source, root, api,
  readWorkflow = file => readStableFile(join(root, file), 1024 * 1024) }) {
  assert.equal(repository, 'zryna/zryna'); assert.match(source.commit, /^[a-f0-9]{40}$/u);
  const workflows = new Map();
  const jobs = [];
  async function expand(file, prefix = '', stack = []) {
    assert(WORKFLOW.test(file), 'local registered workflow path required');
    assert(!stack.includes(file) && stack.length < 8, 'bounded acyclic reusable workflows');
    if (!workflows.has(file)) {
      assert(workflows.size < MAX_WORKFLOWS, 'workflow inventory bound');
      const bytes = readWorkflow(file);
      const observed = await api(`/repos/${repository}/contents/${file}?ref=${source.commit}`);
      assert.equal(observed.type, 'file'); assert.equal(observed.path, file); assert.equal(observed.encoding, 'base64');
      assert.equal(observed.size, bytes.length);
      const remote = Buffer.from(observed.content.replace(/\n/gu, ''), 'base64');
      assert(remote.equals(bytes), 'exact authenticated immutable workflow source bytes');
      const blob = createHash('sha1').update(Buffer.from(`blob ${bytes.length}\0`)).update(bytes).digest('hex');
      assert.equal(observed.sha, blob, 'authenticated Git workflow blob identity');
      const parsed = parseDocument(new TextDecoder('utf-8', { fatal: true }).decode(bytes), { uniqueKeys: true });
      assert.equal(parsed.errors.length, 0, 'strict workflow YAML');
      const workflow = parsed.toJS({ maxAliasCount: 0 });
      assert(workflow?.jobs && typeof workflow.jobs === 'object' && !Array.isArray(workflow.jobs));
      if (stack.length === 0) assert(Object.hasOwn(workflow.on ?? {}, 'workflow_dispatch'), 'full dispatch entrypoint');
      else assert(Object.hasOwn(workflow.on ?? {}, 'workflow_call'), 'reusable workflow entrypoint');
      workflows.set(file, { workflow, descriptor: { path: file, bytes: bytes.length, sha256: digest(bytes), gitBlob: blob } });
    }
    const { workflow } = workflows.get(file);
    for (const [key, job] of Object.entries(workflow.jobs)) {
      assert(/^[A-Za-z0-9_-]+$/u.test(key) && job && typeof job === 'object' && !Array.isArray(job), 'static job definition');
      requiredOnFullDispatch(job.if);
      for (const row of matrixRows(job.strategy?.matrix)) {
        const name = `${prefix}${renderedName(job.name ?? key, row)}`;
        if (job.uses !== undefined) {
          assert(typeof job.uses === 'string' && job.uses.startsWith('./') && WORKFLOW.test(job.uses.slice(2)),
            'unsupported reusable workflow reference; immutable local source required');
          assert(job.with === undefined && job.secrets === undefined, 'reusable inputs/secrets require reviewed expansion semantics');
          await expand(job.uses.slice(2), `${name} / `, [...stack, file]);
        } else {
          assert(jobs.length < MAX_JOBS, 'expanded hosted job inventory bound');
          jobs.push({ name, workflowPath: file, jobKey: key, matrix: row });
        }
      }
    }
  }
  await expand('.github/workflows/ci.yml');
  assert(jobs.length > 0); assert.equal(new Set(jobs.map(job => job.name)).size, jobs.length, 'unique expanded hosted names');
  return { jobs, workflowSources: [...workflows.values()].map(item => item.descriptor) };
}

export function verifyCompletePublicationJobs(observed, inventory, expected) {
  assert(Array.isArray(observed.jobs) && observed.total_count === observed.jobs.length && observed.total_count <= MAX_JOBS,
    'complete bounded Jobs API response');
  const wanted = inventory.jobs.map(job => job.name).sort();
  assert.deepEqual(observed.jobs.map(job => job.name).sort(), wanted, 'complete source-derived hosted job inventory');
  for (const job of observed.jobs) {
    assert.equal(job.run_id, expected.runId); assert.equal(job.run_attempt, expected.attempt);
    assert.equal(job.head_sha, expected.commit); assert.equal(job.status, 'completed');
    assert.equal(job.conclusion, 'success', `required source job must succeed: ${job.name}`);
  }
}
