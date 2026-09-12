import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';

const path = resolve(import.meta.dirname, '../.github/workflows/release-qualification.yml');
const text = readFileSync(path, 'utf8');
const document = parseDocument(text);
assert.deepEqual(document.errors, []);
const workflow = document.toJS();
const pin = /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/;

test('qualification workflow is manual, non-cancellable, and read-only', () => {
  assert.deepEqual(workflow.on, { workflow_dispatch: null });
  assert.deepEqual(workflow.permissions, { contents: 'read' });
  assert.deepEqual(Object.keys(workflow.jobs), ['admit', 'build', 'compare']);
  assert.equal(workflow.concurrency['cancel-in-progress'], false);
  for (const job of Object.values(workflow.jobs)) {
    assert.notEqual(job.permissions?.contents, 'write');
    assert.equal(job.permissions?.['id-token'], undefined);
    assert.equal(job.permissions?.attestations, undefined);
    assert.equal(job.environment, undefined);
  }
  assert.doesNotMatch(text, /gh\s+release|refs\/tags|attest|id-token:\s*write/i);
});

test('qualification workflow pins actions and closes the replica topology', () => {
  for (const job of Object.values(workflow.jobs)) {
    for (const step of job.steps) if (step.uses) assert.match(step.uses, pin, step.uses);
  }
  assert.equal(workflow.jobs.build.needs, 'admit');
  assert.equal(workflow.jobs.compare.needs, 'build');
  assert.deepEqual(workflow.jobs.build.strategy.matrix.include.map(({ target, replica }) =>
    `${target}:${replica}`), [
    'x86_64-unknown-linux-gnu:1', 'x86_64-unknown-linux-gnu:2',
    'x86_64-pc-windows-msvc:1', 'x86_64-pc-windows-msvc:2',
  ]);
  assert.match(text, /run-release-qualification\.mjs/);
  assert.match(text, /compare-release-qualifications\.mjs/);
  assert.match(text, /compression-level: 0/);
  assert.match(text, /retention-days: 1/);
});
