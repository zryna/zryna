import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parseDocument } from 'yaml';
import { withoutM4Workflow, withoutSQLiteFixture } from './m4-workflow-cases.mjs';
import { classifyWorkflowPaths } from '../scripts/classify-workflow-paths.mjs';

const workflow = name => {
  const result = parseDocument(readFileSync(new URL(`../.github/workflows/${name}`, import.meta.url), 'utf8'));
  assert.deepEqual(result.errors, []);
  return result.toJS();
};
function artifactUploads(job, lane, evidence) {
  const uploads = job.steps.filter(step => step.uses?.startsWith('actions/upload-artifact@'));
  assert.equal(uploads.length, 2, 'exact success/diagnostic uploads');
  const [success, diagnostic] = uploads;
  assert.equal(success.if, 'success()');
  assert.equal(diagnostic.if, 'failure()');
  assert.equal(success.with.path, `${'${{ runner.temp }}'}/${evidence}-retained/**`);
  assert.equal(diagnostic.with.path, `${'${{ runner.temp }}'}/${evidence}/**`);
  assert.equal(success.with.name, `m4-${lane}-${'${{ matrix.os }}'}-${'${{ github.event.pull_request.head.sha || github.sha }}'}-${'${{ github.run_id }}'}-${'${{ github.run_attempt }}'}`);
  assert.equal(diagnostic.with.name, success.with.name.replace(`m4-${lane}-`, `m4-${lane}-diagnostics-`));
  for (const upload of uploads) {
    assert.equal(upload.uses, 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a');
    assert.equal(upload.with['if-no-files-found'], 'error');
    assert.equal(upload.with['compression-level'], 0);
    assert.equal(upload.with['retention-days'], 90);
  }
}
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
    artifactUploads(job, lane, lane === 'browser' ? 'browser-conformance-evidence' : 'installed-h1-evidence');
  }
});
test('M4 successful artifact inventory is distinct from failed diagnostics for every lane', () => {
  artifactUploads(workflow('ci.yml').jobs['m4-platform'], 'core', 'm4-core-evidence');
  for (const [name, key, lane, evidence] of [
    ['ci.yml', 'm4-platform', 'core', 'm4-core-evidence'],
    ['browser-conformance.yml', 'conformance', 'browser', 'browser-conformance-evidence'],
    ['installed-command-h1.yml', 'qualify', 'installed', 'installed-h1-evidence'],
  ]) {
    const original = workflow(name).jobs[key];
    for (const mutate of [
      job => { job.steps.at(-2).if = 'always()'; },
      job => { job.steps.at(-1).if = 'success()'; },
      job => { job.steps.at(-2).with.path = job.steps.at(-1).with.path; },
      job => { job.steps.at(-1).with.name = job.steps.at(-2).with.name; },
      job => { job.steps.at(-2).with['retention-days'] = 7; },
      job => { job.steps.at(-2).uses = 'actions/upload-artifact@main'; },
    ]) {
      const changed = structuredClone(original); mutate(changed);
      assert.throws(() => artifactUploads(changed, lane, evidence));
    }
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


test('SQLite acquisition projects only three authenticated prerequisites without changing any owner', () => {
  const original = workflow('ci.yml');
  const before = structuredClone(original);
  const expected = structuredClone(original);
  for (const id of ['preflight', 'rust', 'm4-platform']) {
    const steps = expected.jobs[id].steps;
    const index = steps.findIndex(step => step.name === 'Prepare pinned SQLite source fixture');
    steps.splice(index, 1);
  }
  assert.deepEqual(withoutSQLiteFixture(original), expected);
  assert.deepEqual(original, before);
});

test('SQLite prerequisites reject changed commands, conditions, deadlines and locator authority', () => {
  for (const id of ['preflight', 'rust', 'm4-platform']) {
    for (const mutate of [
      step => { delete step.if; }, step => { step.if = 'always()'; },
      step => { step.if = "runner.os == 'Windows'"; },
      step => { step['continue-on-error'] = true; },
      step => { step['timeout-minutes'] = 4; }, step => { delete step['timeout-minutes']; },
      step => { step.shell = 'pwsh'; },
      step => { step.run = step.run.replace('python3 -B', 'python3'); },
      step => { step.run += 'true\n'; },
      step => { step.run = step.run.replace(' >> ', ' > '); },
      step => { step.run = step.run.replace('zryna-sqlite-source', 'other-source'); },
      step => { step.env = { ZRYNA_SQLITE_FIXTURE_ROOT: '/tmp/unchecked' }; },
    ]) {
      const changed = workflow('ci.yml');
      mutate(changed.jobs[id].steps.find(step => step.name === 'Prepare pinned SQLite source fixture'));
      assert.throws(() => withoutSQLiteFixture(changed));
    }
  }
});

test('SQLite prerequisite removal, duplication, relocation and extra invocation reject', () => {
  for (const id of ['preflight', 'rust', 'm4-platform']) {
    for (const mutate of [
      (steps, index) => { steps.splice(index, 1); },
      (steps, index) => { steps.push(structuredClone(steps[index])); },
      (steps, index) => { [steps[index], steps[index + 1]] = [steps[index + 1], steps[index]]; },
    ]) {
      const changed = workflow('ci.yml');
      const steps = changed.jobs[id].steps;
      mutate(steps, steps.findIndex(step => step.name === 'Prepare pinned SQLite source fixture'));
      assert.throws(() => withoutSQLiteFixture(changed));
    }
  }
  for (const mutate of [
    value => { value.env.ZRYNA_SQLITE_FIXTURE_ROOT = '/tmp/unchecked'; },
    value => { value.jobs.rust.env = { ZRYNA_SQLITE_FIXTURE_ROOT: '/tmp/unchecked' }; },
    value => { value.jobs['fast-contracts'].steps.push({ run: 'python3 scripts/m5/prepare_sqlite_fixture.py' }); },
  ]) {
    const changed = workflow('ci.yml'); mutate(changed);
    assert.throws(() => withoutSQLiteFixture(changed));
  }
});
