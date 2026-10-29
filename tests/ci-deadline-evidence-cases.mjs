// Exact additive diagnostics inverse; the existing whole-workflow authorities still apply.
import assert from 'node:assert/strict';
import { parseDocument } from 'yaml';
const condition = "failure() && runner.os == 'Linux'";
const profile = '0';
const unit = { name: 'Verify bounded original-failure collector controls',
  if: "runner.os == 'Linux'", run: 'python3 -B tests/test_generic_deadline_retention.py' };
const groups = [
  ['rust', 'Retain ordinary nested and clone originals after Linux failure',
    'Upload ordinary original failure evidence', 'generic-deadline-originals'],
  ['m4-platform', 'Retain ordinary nested and clone originals after Linux M4 failure',
    'Upload ordinary M4 original failure evidence', 'generic-deadline-originals-m4'],
];
const action = 'actions/upload-artifact@043fb46d1a93c77aae656e7c1c64a875d1fc6a0a';
const pair = ([, collect, upload, directory]) => [
  { name: collect, if: condition, 'timeout-minutes': 2, shell: 'bash',
    run: 'python3 -B scripts/retain-generic-deadline-failures.py --output "$RUNNER_TEMP/' + directory + '"' },
  { name: upload, if: condition, uses: action, with: {
    name: directory + '-${{ github.run_id }}-${{ github.run_attempt }}',
    path: '${{ runner.temp }}/' + directory, 'if-no-files-found': 'error', 'retention-days': 7 } },
];
function block(group) {
  const [collect, upload] = pair(group);
  return ['      - name: ' + collect.name, '        if: ' + condition,
    '        timeout-minutes: 2', '        shell: bash', '        run: ' + collect.run,
    '      - name: ' + upload.name, '        if: ' + condition,
    '        uses: ' + action + ' # v7.0.1', '        with:',
    '          name: ' + upload.with.name, '          path: ' + upload.with.path,
    '          if-no-files-found: error', '          retention-days: 7', ''].join('\n');
}
const unitText = '      - name: ' + unit.name + '\n        if: ' + unit.if + '\n        run: ' + unit.run + '\n';
const profileText = ['DEV', 'TEST'].map(kind => '      CARGO_PROFILE_' + kind + '_DEBUG: "' + profile + '"\n').join('');
function present(value) {
  return groups.some(([id, name, upload]) => value.jobs[id]?.steps.some(step => [name, upload, unit.name].includes(step.name)))
    || ['DEV', 'TEST'].some(kind => Object.hasOwn(value.jobs['m4-platform']?.env ?? {}, 'CARGO_PROFILE_' + kind + '_DEBUG'));
}
export function withoutDeadlineEvidence(candidate, required = false) {
  const copy = structuredClone(candidate);
  if (!present(copy)) { assert(!required, 'current diagnostics must be present'); return copy; }
  const rust = copy.jobs.rust.steps, index = rust.findIndex(step => step.name === unit.name);
  assert(index > 0); assert.equal(rust[index - 1].name, 'Verify Linux playground helper units');
  assert.equal(rust[index + 1].name, 'Verify Linux compiler publication boundaries');
  assert.deepEqual(rust[index], unit); rust.splice(index, 1);
  for (const group of groups) {
    const steps = copy.jobs[group[0]].steps, offset = steps.findIndex(step => step.name === group[1]);
    assert.equal(offset, steps.length - (group[0] === 'rust' ? 2 : 3));
    assert.deepEqual(steps.slice(offset, offset + 2), pair(group));
    if (group[0] === 'rust') assert.equal(steps[offset - 1].id, 'm0-complete');
    else {
      assert.equal(steps[offset - 1].name, 'Preserve M4 core receipt and complete execution logs');
      assert.equal(steps[offset + 2].name, 'Preserve failed M4 core diagnostics');
    }
    steps.splice(offset, 2);
  }
  for (const kind of ['DEV', 'TEST']) {
    const key = 'CARGO_PROFILE_' + kind + '_DEBUG';
    assert.equal(copy.jobs['m4-platform'].env[key], profile); delete copy.jobs['m4-platform'].env[key];
  }
  assert(!present(copy), 'no duplicate diagnostics');
  return copy;
}
export function withoutDeadlineEvidenceText(text) {
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 512 * 1024);
  const document = parseDocument(text, { uniqueKeys: true }); assert.deepEqual(document.errors, []);
  if (!present(document.toJS())) return text;
  withoutDeadlineEvidence(document.toJS(), true);
  for (const addition of [unitText, ...groups.map(block)]) {
    assert.equal(text.split(addition).length, 2, 'one exact diagnostic text block'); text = text.replace(addition, '');
  }
  const anchor = '      RUST_TEST_THREADS: 3\n';
  assert.equal(text.split(anchor + profileText + '    steps:\n').length, 2, 'exact M4 metadata profile placement');
  return text.replace(anchor + profileText, anchor);
}
