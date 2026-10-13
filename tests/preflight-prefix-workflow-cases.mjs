import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { withoutBootstrapTiming } from './npm-timing-workflow-cases.mjs';
import { withoutM4Workflow } from './m4-workflow-cases.mjs';
import { withoutNativeSourceResources } from './native-source-resource-runner.test.mjs';
import { withoutStabilityTests } from './stability-gates-test-selection.test.mjs';
import { withoutNativeRecipeTests } from './native-recipe-test-selection.test.mjs';

const pnpmStep = {
  uses: 'pnpm/action-setup@0977fd99725f1db4007ccb2928dbb4e90d06cc86',
  with: { version: '11.18.0' },
};
const captureStep = {
  name: 'Capture ordinary sanitizer compiler prefix',
  'timeout-minutes': 5,
  env: { M5_PREFIX_CANDIDATE: '${{ github.event.pull_request.head.sha || github.sha }}' },
  run: `python3 -B -m unittest discover -s scripts/m5/tests -p 'test_*.py'
python3 -B scripts/m5/collect_ordinary_sanitizer_prefix.py --output "$RUNNER_TEMP/m5-ordinary-sanitizer-prefix"
`,
};
const uploadStep = {
  name: 'Upload ordinary sanitizer compiler prefix',
  if: '${{ !cancelled() }}',
  uses: 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a',
  with: {
    name: 'm5-ordinary-sanitizer-prefix-${{ github.event.pull_request.head.sha || github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}',
    path: '${{ runner.temp }}/m5-ordinary-sanitizer-prefix',
    'if-no-files-found': 'error',
    'retention-days': 7,
  },
};

const runtimeStep = {
  name: 'Execute reviewed ordinary sanitizer runtime',
  'timeout-minutes': 5,
  env: { M5_PREFIX_CANDIDATE: '${{ github.event.pull_request.head.sha || github.sha }}' },
  run: 'python3 -B scripts/m5/collect_owned_sanitizer_runtime.py --output "$RUNNER_TEMP/m5-ordinary-sanitizer-runtime"',
};
const runtimeUploadStep = {
  name: 'Upload reviewed ordinary sanitizer runtime',
  if: '${{ !cancelled() }}',
  uses: 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a',
  with: {
    name: 'm5-ordinary-sanitizer-runtime-${{ github.event.pull_request.head.sha || github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}',
    path: '${{ runner.temp }}/m5-ordinary-sanitizer-runtime',
    'if-no-files-found': 'error',
    'retention-days': 7,
  },
};

function withoutOrdinarySanitizerRuntime(candidate) {
  const original = structuredClone(candidate);
  const steps = original.jobs.preflight.steps;
  const run = steps.filter(step => step.name === runtimeStep.name);
  const upload = steps.filter(step => step.name === runtimeUploadStep.name);
  assert.equal(run.length, 1);
  assert.equal(upload.length, 1);
  assert.deepEqual(run[0], runtimeStep);
  assert.deepEqual(upload[0], runtimeUploadStep);
  assert.equal(steps.at(-3).name, uploadStep.name);
  assert.equal(steps.at(-2), run[0]);
  assert.equal(steps.at(-1), upload[0]);
  steps.splice(-2, 2);
  return original;
}

// Remove only an authenticated suffix before the unchanged legacy budget projection.
export function withoutOrdinarySanitizerPrefix(candidate) {
  const original = withoutOrdinarySanitizerRuntime(candidate);
  const steps = original.jobs.preflight.steps;
  const capture = steps.filter(step => step.name === captureStep.name);
  const upload = steps.filter(step => step.name === uploadStep.name);
  const compiler = steps.filter(step => step.run === 'node scripts/run-preflight.mjs --compiler-contracts');
  assert.equal(capture.length, 1);
  assert.equal(upload.length, 1);
  assert.equal(compiler.length, 1);
  assert.deepEqual(capture[0], captureStep);
  assert.deepEqual(upload[0], uploadStep);
  assert.deepEqual(compiler[0], { run: 'node scripts/run-preflight.mjs --compiler-contracts', 'timeout-minutes': 25 });
  assert.equal(steps.at(-3), compiler[0]);
  assert.equal(steps.at(-2), capture[0]);
  assert.equal(steps.at(-1), upload[0]);
  steps.splice(-2, 2);
  return original;
}

export function withoutPreflightBudgets(candidate) {
  const original = structuredClone(candidate);
  const job = original.jobs.preflight;
  assert.deepEqual(Object.keys(job).sort(), ['env', 'name', 'runs-on', 'steps', 'timeout-minutes']);
  assert.deepEqual(job.env, { CARGO_PROFILE_DEV_DEBUG: 0, CARGO_PROFILE_TEST_DEBUG: 0 });
  delete job.env;
  assert.equal(job['timeout-minutes'], 40);
  const bootstrap = job.steps.filter(step => step.uses?.startsWith('pnpm/action-setup@'));
  const execution = job.steps.filter(step => step.run === 'node scripts/run-preflight.mjs --compiler-contracts');
  assert.equal(bootstrap.length, 1);
  assert.equal(execution.length, 1);
  assert.deepEqual(bootstrap[0], { ...pnpmStep, 'timeout-minutes': 10 });
  assert.deepEqual(execution[0], { run: 'node scripts/run-preflight.mjs --compiler-contracts', 'timeout-minutes': 25 });
  assert.equal(job.steps.at(-1), execution[0]);
  assert(job.steps.indexOf(bootstrap[0]) < job.steps.indexOf(execution[0]));
  assert.equal(job['timeout-minutes'] - bootstrap[0]['timeout-minutes'] - execution[0]['timeout-minutes'], 5);
  delete bootstrap[0]['timeout-minutes'];
  delete execution[0]['timeout-minutes'];
  for (const other of Object.values(original.jobs)) {
    for (const step of other.steps) assert.equal(step['timeout-minutes'], undefined);
  }
  return original;
}

const document = parseDocument(readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'));
assert.deepEqual(document.errors, []);
const current = document.toJS();

function changeSuffix(mutate) {
  const changed = structuredClone(current);
  const steps = changed.jobs.preflight.steps;
  mutate(steps.at(-4), steps.at(-3));
  return changed;
}

test('ordinary sanitizer prefix projection preserves the exact legacy compiler and every other workflow field', () => {
  const before = structuredClone(current);
  const expected = structuredClone(current);
  expected.jobs.preflight.steps.splice(-4, 4);
  const projected = withoutOrdinarySanitizerPrefix(current);
  assert.deepEqual(projected, expected);
  assert.deepEqual(current, before);
  assert.deepEqual(projected.jobs.preflight.steps.at(-1), {
    run: 'node scripts/run-preflight.mjs --compiler-contracts', 'timeout-minutes': 25,
  });
});

test('ordinary sanitizer prefix projection rejects changed commands budgets bindings and upload authority', () => {
  for (const mutate of [
    capture => { delete capture['timeout-minutes']; },
    capture => { capture['timeout-minutes'] = 6; },
    capture => { capture.if = 'always()'; },
    capture => { capture['continue-on-error'] = true; },
    capture => { capture.env.M5_PREFIX_CANDIDATE = '${{ github.sha }}'; },
    capture => { capture.env.ZRYNA_RUST_ORDINARY_PREFIX_CAPTURE = '1'; },
    capture => { capture.run = capture.run.replace('python3 -B -m', 'python3 -m'); },
    capture => { capture.run = capture.run.replace('python3 -B scripts/', 'python3 scripts/'); },
    capture => { capture.run += 'true\n'; },
    capture => { capture.run = capture.run.replace('/m5-ordinary-sanitizer-prefix', '/other-prefix'); },
    (_, upload) => { upload.if = 'success()'; },
    (_, upload) => { upload['continue-on-error'] = true; },
    (_, upload) => { upload.uses = 'actions/upload-artifact@main'; },
    (_, upload) => { upload.with.name = 'm5-ordinary-sanitizer-prefix'; },
    (_, upload) => { upload.with.path = '${{ github.workspace }}'; },
    (_, upload) => { upload.with['if-no-files-found'] = 'ignore'; },
    (_, upload) => { upload.with['retention-days'] = 8; },
    (_, upload) => { upload.with['include-hidden-files'] = true; },
  ]) {
    assert.throws(() => withoutOrdinarySanitizerPrefix(changeSuffix(mutate)));
  }
});

test('ordinary sanitizer prefix projection rejects absent duplicate reordered or trailing steps', () => {
  for (const mutate of [
    steps => { steps.pop(); },
    steps => { steps.splice(-2, 1); },
    steps => { steps.splice(-3, 1); },
    steps => { steps.push(structuredClone(steps.at(-1))); },
    steps => { steps.splice(-1, 0, structuredClone(steps.at(-2))); },
    steps => { const last = steps.pop(); steps.splice(-1, 0, last); },
    steps => { const capture = steps.splice(-2, 1)[0]; steps.splice(-2, 0, capture); },
    steps => { steps.push({ run: 'true' }); },
  ]) {
    const changed = structuredClone(current);
    const steps = changed.jobs.preflight.steps;
    const runtime = steps.splice(-2, 2);
    mutate(steps);
    steps.push(...runtime);
    assert.throws(() => withoutOrdinarySanitizerPrefix(changed));
  }
});

function changeRuntimeSuffix(mutate) {
  const changed = structuredClone(current);
  const steps = changed.jobs.preflight.steps;
  mutate(steps.at(-2), steps.at(-1));
  return changed;
}

test('reviewed ordinary runtime suffix authenticates exact execution and upload before legacy budget projection', () => {
  const projected = withoutOrdinarySanitizerPrefix(current);
  assert.equal(projected.jobs.preflight['timeout-minutes'], 40);
  assert.equal(projected.jobs.preflight.steps.at(-1)['timeout-minutes'], 25);
  const legacy = withoutNativeSourceResources(
    withoutStabilityTests(withoutNativeRecipeTests(withoutM4Workflow(projected))));
  assert.deepEqual(legacy.jobs['portable-setup'], {
    name: 'portable setup', needs: ['route-contracts', 'm0'],
    if: "needs.route-contracts.outputs.distribution_release == 'true'",
    uses: './.github/workflows/portable-setup.yml',
  });
  delete legacy.jobs['portable-setup'];
  assert.deepEqual(withoutPreflightBudgets(withoutBootstrapTiming(legacy)).jobs.preflight.steps.at(-1), {
    run: 'node scripts/run-preflight.mjs --compiler-contracts',
  });
});

test('reviewed ordinary runtime suffix rejects alternate commands profiles budgets and upload authority', () => {
  for (const mutate of [
    run => { delete run['timeout-minutes']; },
    run => { run['timeout-minutes'] = 6; },
    run => { run.if = 'always()'; },
    run => { run['continue-on-error'] = true; },
    run => { run.env.M5_PREFIX_CANDIDATE = '${{ github.sha }}'; },
    run => { run.env.ZRYNA_RUST_ORDINARY_PREFIX_CAPTURE = '1'; },
    run => { run.run = run.run.replace('python3 -B', 'python3'); },
    run => { run.run += '\ntrue'; },
    run => { run.run += ' --profile unchecked'; },
    run => { run.run = run.run.replace('/m5-ordinary-sanitizer-runtime', '/other-runtime'); },
    (_, upload) => { upload.if = 'success()'; },
    (_, upload) => { upload['continue-on-error'] = true; },
    (_, upload) => { upload.uses = 'actions/upload-artifact@main'; },
    (_, upload) => { upload.with.name = 'm5-ordinary-sanitizer-runtime'; },
    (_, upload) => { upload.with.path = '${{ github.workspace }}'; },
    (_, upload) => { upload.with['if-no-files-found'] = 'ignore'; },
    (_, upload) => { upload.with['retention-days'] = 8; },
    (_, upload) => { upload.with['include-hidden-files'] = true; },
  ]) assert.throws(() => withoutOrdinarySanitizerPrefix(changeRuntimeSuffix(mutate)));
});

test('reviewed ordinary runtime suffix rejects omissions duplicates reordered phases and trailing steps', () => {
  for (const mutate of [
    steps => { steps.pop(); },
    steps => { steps.splice(-2, 1); },
    steps => { steps.push(structuredClone(steps.at(-1))); },
    steps => { steps.splice(-1, 0, structuredClone(steps.at(-2))); },
    steps => { const last = steps.pop(); steps.splice(-1, 0, last); },
    steps => { const run = steps.splice(-2, 1)[0]; steps.splice(-4, 0, run); },
    steps => { steps.push({ run: 'true' }); },
  ]) {
    const changed = structuredClone(current);
    mutate(changed.jobs.preflight.steps);
    assert.throws(() => withoutOrdinarySanitizerPrefix(changed));
  }
});
