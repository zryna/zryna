import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';

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

// Remove only an authenticated suffix before the unchanged legacy budget projection.
export function withoutOrdinarySanitizerPrefix(candidate) {
  const original = structuredClone(candidate);
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
  mutate(steps.at(-2), steps.at(-1));
  return changed;
}

test('ordinary sanitizer prefix projection preserves the exact legacy compiler and every other workflow field', () => {
  const before = structuredClone(current);
  const expected = structuredClone(current);
  expected.jobs.preflight.steps.splice(-2, 2);
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
    mutate(changed.jobs.preflight.steps);
    assert.throws(() => withoutOrdinarySanitizerPrefix(changed));
  }
});
