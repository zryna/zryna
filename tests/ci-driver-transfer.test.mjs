// Pure workflow declaration controls. Never starts Cargo, a compiler, a guest or CI.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { parseDocument } from 'yaml';
import { withoutDriverTransfer, withoutDriverTransferText } from './ci-driver-transfer-cases.mjs';
const legacy = JSON.parse(readFileSync(new URL('./ci-driver-transfer/legacy.json', import.meta.url)));
const debugProfile = {
  CARGO_PROFILE_DEV_DEBUG: "${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}",
  CARGO_PROFILE_TEST_DEBUG: "${{ matrix.os == 'ubuntu-latest' && '0' || 'line-tables-only' }}",
};
function transferred() {
  const value = structuredClone(legacy), jobs = value.jobs;
  const moved = jobs.rust.steps.splice(8, 7);
  jobs['m2-platform'].steps.splice(7, 0, ...moved);
  jobs.rust.steps.splice(8, 0, structuredClone(moved[1]));
  jobs['m2-platform'].steps[1].with.components = 'rustfmt, clippy';
  jobs.m0.needs.push('m2-platform');
  jobs.m0.steps.at(-1).env.M2_PLATFORM_RESULT = '${{ needs.m2-platform.result }}';
  jobs.m0.steps.at(-1).run += ' && test "$M2_PLATFORM_RESULT" = success';
  assert.doesNotThrow(() => withoutDriverTransfer(value));
  return value;
}
function rejects(mutate) {
  const value = transferred(); mutate(value); assert.throws(() => withoutDriverTransfer(value));
}
test('actual workflow transfers all seven steps before canonical M2 and gates M0', () => {
  const bytes = readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const parsed = parseDocument(bytes); assert.deepEqual(parsed.errors, []);
  assert.deepEqual(parsed.toJS().jobs.rust.env, debugProfile, 'actual Linux CI uses reviewed debug profile');
  assert.deepEqual(withoutDriverTransfer(parsed.toJS()), legacy);
  const text = withoutDriverTransferText(bytes);
  const old = parseDocument(text); assert.deepEqual(old.errors, []); assert.deepEqual(old.toJS(), legacy);
});
test('exact Linux host debug profile reverses without changing historical authority or its input', () => {
  const value = transferred(); value.jobs.rust.env = structuredClone(debugProfile);
  const before = structuredClone(value);
  assert.deepEqual(withoutDriverTransfer(value), legacy);
  assert.deepEqual(value, before);
});
test('debug profile rejects partial unknown or broadened declarations and retains all proof budgets', () => {
  for (const mutate of [
    w => { delete w.jobs.rust.env.CARGO_PROFILE_TEST_DEBUG; },
    w => { w.jobs.rust.env.CARGO_PROFILE_DEV_DEBUG = '0'; },
    w => { w.jobs.rust.env.CARGO_PROFILE_TEST_DEBUG = "${{ matrix.os == 'ubuntu-latest' && '1' || 'line-tables-only' }}"; },
    w => { w.jobs.rust.env.CARGO_PROFILE_TEST_DEBUG = "${{ matrix.os == 'ubuntu-latest' && '0' || '0' }}"; },
    w => { w.jobs.rust.env.CARGO_PROFILE_TEST_OPT_LEVEL = '1'; },
    w => { w.jobs.rust['timeout-minutes'] = 60; },
    w => { w.jobs.rust.steps.find(s => s.id === 'm0-complete').run += ' --skip'; },
    w => { w.jobs.rust.steps.find(s => s.id === 'm7-generic').run = 'true'; },
  ]) {
    const value = transferred(); value.jobs.rust.env = structuredClone(debugProfile);
    mutate(value); assert.throws(() => withoutDriverTransfer(value));
  }
});
test('text inverse rejects a moved debug profile block even when parsed workflow is equivalent', () => {
  const bytes = readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
  const start = bytes.indexOf('  rust:\n'), end = bytes.indexOf('\n  adapter-platform:\n', start);
  const block = '    env:\n' + Object.entries(debugProfile).map(([key, value]) => `      ${key}: "${value}"\n`).join('');
  const rust = bytes.slice(start, end); assert.equal(rust.split(block).length, 2);
  const moved = rust.replace(block, '').replace('    steps:\n', block + '    steps:\n');
  // Moving before runs-on preserves YAML semantics but must not broaden raw-text admission.
  const reordered = moved.replace(block, '').replace('    runs-on:', block + '    runs-on:');
  assert.throws(() => withoutDriverTransferText(bytes.slice(0, start) + reordered + bytes.slice(end)));
});
test('valid transferred fixture reverses exactly without changing its input or legacy expectations', () => {
  const value = transferred(), before = structuredClone(value);
  assert.deepEqual(withoutDriverTransfer(value), legacy); assert.deepEqual(value, before);
  assert.throws(() => withoutDriverTransfer(legacy), /exact seven moved steps/u);
});
test('each moved proof rejects removal duplicate or a wrong command from a valid baseline', () => {
  for (let i = 7; i < 14; i++) {
    rejects(w => w.jobs['m2-platform'].steps.splice(i, 1));
    rejects(w => w.jobs['m2-platform'].steps.push(structuredClone(w.jobs['m2-platform'].steps[i])));
    rejects(w => { w.jobs['m2-platform'].steps[i].run += ' || true'; });
  }
});
test('every moved proof retains exact condition and fail-closed policy', () => {
  for (let i = 7; i < 14; i++) {
    rejects(w => { w.jobs['m2-platform'].steps[i].if = 'false'; });
    rejects(w => { delete w.jobs['m2-platform'].steps[i].if; w.jobs['m2-platform'].steps[i].env = { SKIP: 'true' }; });
    rejects(w => { w.jobs['m2-platform'].steps[i]['continue-on-error'] = true; });
  }
});
test('placement host matrices pinned tools budgets and all M0/M7 commands stay exact', () => {
  for (const mutate of [
    w => { const a = w.jobs['m2-platform'].steps; [a[7], a[8]] = [a[8], a[7]]; },
    w => { w.jobs.rust.steps.splice(8, 0, w.jobs['m2-platform'].steps[7]); },
    w => { w.jobs['m2-platform'].strategy.matrix.os.pop(); },
    w => { w.jobs.rust.strategy['fail-fast'] = true; },
    w => { w.jobs['m2-platform'].steps[1].with.components = 'clippy'; },
    w => { w.jobs['m2-platform'].steps[1].uses += 'bad'; },
    w => { w.jobs['m2-platform']['timeout-minutes'] = 60; },
    w => { w.jobs.rust['timeout-minutes'] = 60; },
    w => { w.jobs['m2-platform'].env = { RUSTFLAGS: '--cfg skip' }; },
    w => { w.jobs.rust.steps.find(s => s.id === 'm0-complete').run += ' --skip'; },
    w => { w.jobs.rust.steps.find(s => s.id === 'm7-generic').run = 'true'; },
    w => { w.env.ZRYNA_STRUCTURE_BASE = 'HEAD'; },
  ]) rejects(mutate);
});
test('M0 requires actual transferred job success with acyclic dependencies', () => {
  for (const mutate of [
    w => { w.jobs.m0.needs.pop(); },
    w => { w.jobs.m0.needs.push('m2-platform'); },
    w => { w.jobs['m2-platform'].needs = ['m0']; },
    w => { w.jobs['m2-platform'].needs = ['m2']; },
    w => { w.jobs.adapter.needs = ['m0']; },
    w => { delete w.jobs.m0.steps.at(-1).env.M2_PLATFORM_RESULT; },
    w => { w.jobs.m0.steps.at(-1).env.M2_PLATFORM_RESULT = 'success'; },
    w => { w.jobs.m0.steps.at(-1).run = w.jobs.m0.steps.at(-1).run.replace(' && test "$M2_PLATFORM_RESULT" = success', ''); },
    w => { w.jobs.m0.steps.at(-1)['continue-on-error'] = true; },
    w => { w.jobs['m2-platform'].if = 'false'; },
  ]) rejects(mutate);
});

test('each remaining job retains its exact independent SQLite prerequisite', () => {
  const value = transferred();
  const expected = legacy.jobs.rust.steps[9];
  assert.deepEqual(value.jobs.rust.steps[8], expected);
  assert.deepEqual(value.jobs['m2-platform'].steps[8], expected);
  assert.notEqual(value.jobs.rust.steps[8], value.jobs['m2-platform'].steps[8]);
  for (const owner of ['rust', 'm2-platform']) {
    for (const change of [
      steps => { steps.splice(8, 1); },
      steps => { steps[8].if = 'false'; },
      steps => { steps[8]['timeout-minutes'] = 4; },
      steps => { steps[8].run = steps[8].run.replace('GITHUB_ENV', 'DISCARDED_ENV'); },
      steps => { steps[8]['continue-on-error'] = true; },
    ]) rejects(w => change(w.jobs[owner].steps));
  }
});
