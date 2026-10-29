// Pure chronology and ownership rejection; actual completed API positives require original campaign inputs.
import assert from 'node:assert/strict';
import test from 'node:test';
import { requireNumericAnchorOrder } from '../scripts/stability-gates/cli-performance-completed-anchor-v1.mjs';
import { numericAnchorArguments } from '../scripts/stability-gates/run-cli-performance-numeric-anchor-v1.mjs';
import { reconcileCompletedNumericCampaign, inspectCompletedNumericCampaign,
  closeCompletedNumericCampaign } from '../scripts/stability-gates/cli-performance-completed-campaign-v1.mjs';
test('actual anchor creation job completion and run terminal bound each precede every heldout server start', () => {
  const complete = { artifact: { createdAt: '2030-01-01T00:00:01Z' },
    job: { completedAt: '2030-01-01T00:00:02Z' }, completedAt: '2030-01-01T00:00:03Z' };
  const starts = ['2030-01-01T00:00:04Z', '2030-01-01T00:00:05Z', '2030-01-01T00:00:06Z'];
  requireNumericAnchorOrder(complete, starts);
  for (const field of ['artifact', 'job', 'run']) {
    const altered = structuredClone(complete);
    if (field === 'artifact') altered.artifact.createdAt = starts[0];
    else if (field === 'job') altered.job.completedAt = starts[0];
    else altered.completedAt = starts[0];
    assert.throws(() => requireNumericAnchorOrder(altered, starts), /precede heldout/u);
  }
  assert.throws(() => requireNumericAnchorOrder(complete, [complete.completedAt, ...starts]));
});
test('completed comparison has a strict separate mode and leaves both original modes unchanged', () => {
  const output = process.platform === 'win32' ? 'C:\\numeric-output' : '/tmp/numeric-output';
  const compare = numericAnchorArguments(['--mode', 'compare', '--anchor-run-id', '123', '--heldout-run-id', '456', '--output', output]);
  assert.deepEqual(compare, { mode: 'compare', runId: 123, heldoutRunId: 456, output, platform: null });
  assert.deepEqual(numericAnchorArguments(['--mode', 'build-anchor', '--baseline-run-id', '123', '--output', output]),
    { mode: 'build-anchor', runId: 123, output, platform: null });
  assert.deepEqual(numericAnchorArguments(['--mode', 'verify-anchor', '--anchor-run-id', '123', '--platform', 'linux-x86_64', '--output', output]),
    { mode: 'verify-anchor', runId: 123, output, platform: 'linux-x86_64' });
  assert.throws(() => numericAnchorArguments(['--mode', 'compare', '--anchor-run-id', '123', '--platform', 'linux-x86_64', '--output', output]));
});
test('copied completed reports cannot mint or revoke campaign proof ownership', () => {
  for (const key of [{}, { scopedOriginalCampaignVerified: true, scopedNumericRegressionPassed: true }]) {
    assert.throws(() => inspectCompletedNumericCampaign(key)); assert.throws(() => closeCompletedNumericCampaign(key));
  }
});
test('completed proof API rejects source API metric profile and live-context substitution before inspecting Git or network', async () => {
  const base = { root: '/not-opened', anchorRunId: 123, heldoutRunId: 456 };
  for (const field of ['api', 'source', 'raw', 'profile', 'context'])
    await assert.rejects(reconcileCompletedNumericCampaign({ ...base, [field]: {} }));
  await assert.rejects(reconcileCompletedNumericCampaign({ ...base, heldoutRunId: 123 }));
});
