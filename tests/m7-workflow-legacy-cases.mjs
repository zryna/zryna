import { withoutDriverTransfer } from './ci-driver-transfer-cases.mjs';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';

const editor = {
  name: 'Verify and package editor client',
  run: 'pnpm editor:check\npnpm editor:package\n',
};
const unit = {
  name: 'Verify M7 acceptance evidence controls',
  run: 'node --test tests/m7-generic-acceptance.test.mjs',
};
const m0 = { id: 'm0-complete', run: 'node scripts/run-m0-conformance.mjs' };
const packet = {
  name: 'Verify public bounded-generics acceptance packet', id: 'm7-generic',
  env: {
    ZRYNA_M7_UPSTREAM_M0_RESULT: '${{ steps.m0-complete.outcome }}',
    ZRYNA_M7_PR_HEAD: "${{ github.event.pull_request.head.sha || '' }}",
  },
  run: 'node scripts/run-m7-generic-acceptance.mjs --evidence "${{ runner.temp }}/m7-generic-acceptance" --expected-source-revision "${{ github.sha }}"',
};
const upload = {
  name: 'Retain M7 generic acceptance commands and actual counts',
  if: "${{ !cancelled() && steps.m7-generic.outcome != 'skipped' }}",
  uses: 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a',
  with: {
    name: 'm7-generic-${{ matrix.os }}-${{ github.sha }}-${{ github.run_id }}-${{ github.run_attempt }}',
    path: '${{ runner.temp }}/m7-generic-acceptance',
    'if-no-files-found': 'error', 'retention-days': 90,
  },
};

// Recognize only the reviewed addition before the unchanged legacy workflow comparison.
export function withoutM7Workflow(candidate) {
  const original = structuredClone(candidate);
  const steps = original.jobs.rust.steps;
  const index = steps.findIndex(step => step.name === unit.name);
  assert(index > 0);
  assert.equal(index, steps.length - 4, 'the reviewed finite packet is the exact final suffix');
  assert.deepEqual(steps[index - 1], editor, 'preserve the canonical editor boundary');
  assert.deepEqual(steps.slice(index), [unit, m0, packet, upload], 'no changed policy or extra fields');
  // The exact match above permits only this id addition to the original canonical M0 step.
  steps.splice(index, 4, { run: m0.run });
  assert(!/m7|bounded-generics|m0-complete/i.test(JSON.stringify(original)),
    'no unrecognized M7 step, id, command, environment grant or bypass elsewhere');
  return original;
}

const parsed = parseDocument(readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'));
assert.deepEqual(parsed.errors, []);
const workflow = withoutDriverTransfer(parsed.toJS());

test('M7 legacy projection removes only exact reviewed additions without mutating input', () => {
  const before = structuredClone(workflow);
  const projected = withoutM7Workflow(workflow);
  assert.deepEqual(workflow, before);
  const index = workflow.jobs.rust.steps.findIndex(step => step.name === unit.name);
  assert.deepEqual(projected.jobs.rust.steps, [
    ...workflow.jobs.rust.steps.slice(0, index), { run: m0.run },
  ]);
  const restored = structuredClone(projected);
  restored.jobs.rust.steps.splice(index, 1, unit, m0, packet, upload);
  assert.deepEqual(restored, workflow, 'every other workflow property remains exact');
});

test('M7 legacy projection rejects removal, bypass, duplicates and relocation', () => {
  const mutations = [
    (w, s, i) => { s.splice(i, 1); },
    (w, s, i) => { s.splice(i + 1, 1); },
    (w, s, i) => { s.splice(i + 2, 1); },
    (w, s, i) => { s.pop(); },
    (w, s, i) => { s.push(structuredClone(s[i])); },
    (w, s, i) => { [s[i], s[i + 1]] = [s[i + 1], s[i]]; },
    (w, s, i) => { [s[i + 1], s[i + 2]] = [s[i + 2], s[i + 1]]; },
    (w, s, i) => { s[i].run += ' || true'; },
    (w, s, i) => { s[i + 1].shell = 'pwsh'; },
    (w, s, i) => { delete s[i + 1].id; },
    (w, s, i) => { s[i + 1].run += ' || true'; },
    (w, s, i) => { s[i + 2].env.ZRYNA_M7_UPSTREAM_M0_RESULT = 'success'; },
    (w, s, i) => { s[i + 2].run = s[i + 2].run.replace('github.sha', 'github.event.pull_request.head.sha'); },
    (w, s, i) => { s[i + 3].uses = 'actions/upload-artifact@main'; },
    (w, s, i) => { s[i + 3].if = '${{ always() }}'; },
    (w, s, i) => { s[i + 3].with['if-no-files-found'] = 'ignore'; },
    (w, s, i) => { s[i + 3].with.path = '.'; },
    (w, s, i) => { s[i + 3].with['retention-days'] = 7; },
    (w, s, i) => { s[i - 1].run += 'true\n'; },
    (w, s) => { s.unshift({ id: 'm0-complete', run: m0.run }); },
    (w, s) => { w.jobs.preflight.steps.push(structuredClone(packet)); },
  ];
  for (let offset = 0; offset < 4; offset++) {
    mutations.push((w, s, i) => { s[i + offset]['continue-on-error'] = true; });
    mutations.push((w, s, i) => { s[i + offset].if = 'false'; });
  }
  for (const mutate of mutations) {
    const changed = structuredClone(workflow), steps = changed.jobs.rust.steps;
    const index = steps.findIndex(step => step.name === unit.name);
    mutate(changed, steps, index);
    assert.throws(() => withoutM7Workflow(changed));
  }
});
