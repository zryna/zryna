import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parseDocument } from 'yaml';

const prepare = { name: 'Prepare isolated npm timing directory', run: 'node scripts/collect-npm-timing.mjs prepare' };
const collect = { name: 'Collect numeric npm bootstrap timing', if: '${{ !cancelled() }}', run: 'node scripts/collect-npm-timing.mjs collect' };
const timing = { npm_config_timing: 'true', npm_config_logs_dir: '${{ runner.temp }}/zryna-npm-bootstrap-timing' };

export function withoutBootstrapTiming(candidate) {
  const original = structuredClone(candidate);
  const steps = original.jobs.preflight.steps;
  const index = steps.findIndex(step => step.uses?.startsWith('pnpm/action-setup@'));
  assert(index > 0);
  assert.deepEqual(steps[index - 1], prepare);
  assert.deepEqual(steps[index + 1], collect);
  assert.deepEqual(steps[index].env, timing);
  delete steps[index].env;
  steps.splice(index + 1, 1);
  steps.splice(index - 1, 1);
  // Nothing outside this finite addition may configure timing or invoke collection.
  assert(!/npm_config_(?:timing|logs_dir)|collect-npm-timing|zryna-npm-bootstrap-timing/i.test(JSON.stringify(original)));
  return original;
}

const parsed = parseDocument(readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8'));
assert.deepEqual(parsed.errors, []);
const workflow = parsed.toJS();
test('npm timing changes only the preflight bootstrap and preserves fail-closed collection', () => {
  const original = withoutBootstrapTiming(workflow);
  assert.equal(original.jobs.preflight['timeout-minutes'], 30);
  assert.equal(original.jobs.preflight['runs-on'], 'ubuntu-latest');
  assert.equal(workflow.jobs.preflight['continue-on-error'], undefined);
});
test('timing scope, placement, freshness and failure-policy mutations reject', () => {
  const mutations = [
    (w, s, i) => { s.splice(i - 1, 1); },
    (w, s, i) => { s.splice(i + 1, 1); },
    (w, s, i) => { [s[i + 1], s[i + 2]] = [s[i + 2], s[i + 1]]; },
    (w, s, i) => { s[i].env.npm_config_timing = 'false'; },
    (w, s, i) => { s[i].env.npm_config_logs_dir = '~/.npm/_logs'; },
    (w, s, i) => { s[i].env.npm_config_audit = 'false'; },
    (w, s, i) => { s[i - 1].run += ' --reuse'; },
    (w, s, i) => { s[i - 1]['continue-on-error'] = true; },
    (w, s, i) => { s[i + 1]['continue-on-error'] = true; },
    (w, s, i) => { s[i + 1].if = '${{ always() }}'; },
    (w, s, i) => { delete s[i + 1].if; },
    w => { w.env = { npm_config_timing: 'true' }; },
    w => { w.jobs.rust.env = timing; },
    (w, s) => { s.push({ ...collect }); },
  ];
  for (const mutate of mutations) {
    const changed = structuredClone(workflow);
    const steps = changed.jobs.preflight.steps;
    mutate(changed, steps, steps.findIndex(step => step.uses?.startsWith('pnpm/action-setup@')));
    assert.throws(() => withoutBootstrapTiming(changed));
  }
});
