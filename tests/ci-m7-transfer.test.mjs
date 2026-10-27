// Pure declaration checks: never executes Cargo, M0 or M7 acceptance.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { test } from 'node:test';
import { parseDocument } from 'yaml';
import { withoutDriverTransfer, withoutDriverTransferText } from './ci-driver-transfer-cases.mjs';
const raw = readFileSync(new URL('../.github/workflows/ci.yml', import.meta.url), 'utf8');
const workflow = () => { const doc = parseDocument(raw, { uniqueKeys: true }); assert.deepEqual(doc.errors, []); return doc.toJS(); };
test('actual dedicated M7 matrix preserves canonical Rust and waits for its real upstream result', () => {
  const value = workflow(), job = value.jobs['m7-platform']; assert(job, 'dedicated deadline budget required');
  assert.deepEqual(job.needs, ['rust']); assert.deepEqual(job.strategy.matrix.os, ['ubuntu-latest', 'windows-latest']);
  assert.equal(job['timeout-minutes'], "${{ matrix.os == 'windows-latest' && 90 || 40 }}");
  assert.equal(value.jobs.rust.steps.at(-1).id, 'm0-complete');
  const packet = job.steps.find(step => step.id === 'm7-generic');
  assert.equal(packet.env.ZRYNA_M7_UPSTREAM_M0_RESULT, '${{ needs.rust.result }}');
  assert.equal(value.jobs.m0.steps.at(-1).env.M7_PLATFORM_RESULT, '${{ needs.m7-platform.result }}');
  const old = withoutDriverTransfer(value); assert(old.jobs.rust.steps.some(step => step.id === 'm7-generic'));
  assert(!Object.hasOwn(old.jobs, 'm7-platform')); assert.doesNotThrow(() => withoutDriverTransferText(raw));
});
test('registry correction changes only same-job wording and preserves every prior command/test/material value', () => {
  const bytes = readFileSync(new URL('./m7-generic-acceptance-v1.json', import.meta.url));
  const registry = JSON.parse(bytes);
  assert.equal(registry.inherited.canonical_m0.counts,
    'not independently counted by this packet; corroborate actual same-source upstream Rust matching-host canonical M0 logs');
  registry.inherited.canonical_m0.counts = 'not independently counted by this packet; corroborate actual same-job canonical M0 logs';
  assert.equal(createHash('sha256').update(JSON.stringify(registry, null, 2) + '\n').digest('hex'),
    'e6df417bc1c7f61cec3948e1d9cdfbb84531560e2afae333dda56f8de44fb814');
});
test('changed deadline/layout/command/retention or upstream/aggregate bindings reject before historical authority', () => {
  const base = workflow(); assert(base.jobs['m7-platform']);
  for (const change of [
    w => { w.jobs['m7-platform']['timeout-minutes'] = 120; },
    w => { w.jobs['m7-platform'].needs = []; }, w => { w.jobs['m7-platform'].needs = ['m0']; },
    w => { w.jobs['m7-platform'].strategy.matrix.os.pop(); },
    w => { w.jobs['m7-platform'].strategy['fail-fast'] = true; },
    w => { w.jobs['m7-platform'].if = 'always()'; },
    w => { w.jobs['m7-platform'].steps.find(s => s.id === 'm7-generic').env.ZRYNA_M7_UPSTREAM_M0_RESULT = 'success'; },
    w => { w.jobs['m7-platform'].steps.find(s => s.id === 'm7-generic').run += ' || true'; },
    w => { w.jobs['m7-platform'].steps.find(s => s.id === 'm7-generic')['continue-on-error'] = true; },
    w => { w.jobs['m7-platform'].steps.at(-1).with['retention-days'] = 7; },
    w => { w.jobs['m7-platform'].steps.at(-1).with['if-no-files-found'] = 'ignore'; },
    w => { w.jobs['m7-platform'].steps.at(-1).if = 'success()'; },
    w => { w.jobs['m7-platform'].steps[0].with.ref = '${{ github.event.pull_request.head.sha }}'; },
    w => { w.jobs.rust['timeout-minutes'] = 90; },
    w => { w.jobs.rust.steps.at(-1).run += ' --skip'; },
    w => { w.jobs.m0.needs.pop(); }, w => { w.jobs.m0.needs.push('m7-platform'); },
    w => { w.jobs.m0.steps.at(-1).env.M7_PLATFORM_RESULT = 'success'; },
    w => { w.jobs.m0.steps.at(-1).run = w.jobs.m0.steps.at(-1).run.replace(' && test "$M7_PLATFORM_RESULT" = success', ''); },
  ]) { const changed = structuredClone(base); change(changed); assert.throws(() => withoutDriverTransfer(changed)); }
  for (const changed of [raw.replace('    # Three unchanged 25-minute commands', '    # changed comment'),
    raw.replace('          M7_PLATFORM_RESULT:', '          M7_PLATFORM_RESULT: success #')])
    assert.throws(() => withoutDriverTransferText(changed));
});
