import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parseDocument } from 'yaml';
import { withoutM4Workflow } from './m4-workflow-cases.mjs';
import { classifyWorkflowPaths } from '../scripts/classify-workflow-paths.mjs';

const workflow = name => {
  const result = parseDocument(readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), 'utf8'));
  assert.deepEqual(result.errors, []);
  return result.toJS();
};
test('M4 adds all six host lanes and exact candidate receipts without changing historical foundation authority', () => {
  assert.doesNotThrow(() => withoutM4Workflow(workflow('ci.yml')));
  for (const [name, lane, key, baselines] of [
    ['browser-conformance.yml', 'browser', 'conformance', ['ubuntu-22.04', 'windows-2022']],
    ['installed-command-h1.yml', 'installed', 'qualify', ['ubuntu-24.04', 'windows-2022']],
  ]) {
    const value = workflow(name);
    assert.equal(Object.hasOwn(value.on, 'workflow_call'), true);
    assert.equal(Object.hasOwn(value.on, 'pull_request'), true);
    assert.deepEqual(value.permissions, { contents: 'read' });
    const job = value.jobs[key];
    assert.deepEqual(job.strategy.matrix.os, baselines);
    assert.equal(job.env.M4_WORKFLOW_PATH, `.github/workflows/${name}`);
    assert.match(value.concurrency.group, /github\.workflow/);
    assert.equal(job.env.M4_SOURCE_REF, "${{ github.head_ref && format('refs/heads/{0}', github.head_ref) || github.ref }}");
    const execution = job.steps.find(step => step.run?.includes(`--lane ${lane} --evidence`));
    assert(execution);
    assert.match(execution.run, /observe-context\.mjs/);
    assert.match(execution.run, /\$sourceRef = \$env:M4_SOURCE_REF/);
    assert.doesNotMatch(execution.run, /github\.head_ref|github\.ref/);
    assert.match(execution.run, /--expected-commit \$candidate --expected-tree \$tree/);
    assert.equal(execution['continue-on-error'], undefined);
    const upload = job.steps.at(-1);
    assert.equal(upload.if, 'always()');
    assert.equal(upload.with['if-no-files-found'], 'error');
    assert.equal(upload.with['compression-level'], 0);
    assert.match(upload.with.name, /github\.run_id.*github\.run_attempt/);
  }
});
test('M4 source, proofs, schemas, support and examples always select the M4 lane', () => {
  for (const path of ['tests/m4-conformance-v1.json', 'scripts/m4/evidence.mjs',
    'scripts/run-m4-conformance.mjs', 'schemas/zryna-m4-conformance-v1.schema.json',
    'README.md', 'docs/DOCUMENTATION_BUNDLES.md', 'docs/M4_SUPPORT.md', 'docs/STATUS.md', 'docs/CLI.md', 'examples/wasi-command/pure.zry',
    'examples/wasi-server/status.zry', 'apps/zryna/src/support.rs', 'crates/zryna-driver/src/server_runtime.rs']) {
    assert.equal(classifyWorkflowPaths([path]).m4, true, path);
  }
  assert.equal(classifyWorkflowPaths(['editors/vscode-zryna/README.md']).m4, false);
});
test('M4 aggregate mutations cannot normalize into historical passing foundation snapshots', () => {
  const original = workflow('ci.yml');
  for (const mutate of [
    value => { value.jobs.m4.needs.pop(); }, value => { value.jobs.m4.if = 'success()'; },
    value => { value.jobs['m4-platform'].steps[0].with.ref = '${{ github.sha }}'; },
    value => { value.jobs['m4-platform']['continue-on-error'] = true; },
    value => { value.jobs['m4-installed'].if = 'false'; },
  ]) { const changed = structuredClone(original); mutate(changed); assert.throws(() => withoutM4Workflow(changed)); }
});

test('M4 matches preserve conservative foundation routing for unclassified helpers', () => {
  for (const path of [
    'spec/abi/NATIVE_C_INTEROP_V0_REVIEW.md',
    'spec/wasi/new-profile.md', 'spec/interop/new-adapter.md',
    'scripts/m4/new-helper.mjs', 'scripts/scalar-host/new-helper.mjs',
    'tests/browser-component/new-proof.mjs', 'tests/m4-new-proof.test.mjs',
    'schemas/zryna-m4-new.schema.json',
  ]) {
    assert.deepEqual(classifyWorkflowPaths([path]), {
      diagnostics: true, distribution_release: true, package_release: true,
      provider_v4: true, wit: true, m4: true,
    }, path);
  }
});
