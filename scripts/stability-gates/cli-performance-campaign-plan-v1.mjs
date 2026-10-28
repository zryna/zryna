// Closed prospective platform topology; legacy four-baseline selectors are unchanged.
import assert from 'node:assert/strict';
import { canonical, exact, MAX_DOCUMENT, parse, sha256 } from './input.mjs';
import { PROSPECTIVE_POLICY_SHA256 } from './cli-performance-prospective-policy-v1.mjs';
import { digest } from './cli-performance-threshold-profile-v1.mjs';
const HOSTS = Object.freeze(['linux-x86_64', 'windows-x86_64']);
export const NUMERIC_WORKFLOWS = Object.freeze(['baselines', 'anchor', 'heldout', 'numeric-collector']
  .map(role => `.github/workflows/cli-performance-${role}-v1.yml`));
// Whole reviewed YAML identity closes permissions, fixed commands, DAG, guards and retention.
// Refreshing a caller's plan digest cannot authorize an alternate workflow implementation.
export const NUMERIC_WORKFLOW_PINS = Object.freeze([
  { bytes: 1055, sha256: '46e576a6589a862ede5f466d5b342b3fd70e627a7d7b17450b8875b2f47e8455' },
  { bytes: 2758, sha256: '241f052f8b8d4039d5f5edfc79109f1482574f98f466a115e23bc9859a6991f7' },
  { bytes: 877, sha256: '119413c5537b95a9bb3779d021457ff4c36feb632f7b60203cfe26e1c6811295' },
  { bytes: 13113, sha256: '6f4f1abeff65de6f18519c26222f294fb7605aec75c0f6c3702ab784daeeee0b' },
].map(Object.freeze));
export const NUMERIC_LABELS = Object.freeze({ 'linux-x86_64': 'ubuntu-24.04', 'windows-x86_64': 'windows-2022' });
export const numericJobName = ({ host, role }) => `${host} / ${role} / collect`;
export function numericSlots(phase) {
  assert(['baselines', 'anchor', 'heldout'].includes(phase), 'closed campaign phase');
  if (phase === 'anchor') return [{ host: 'linux-x86_64', role: 'anchor', name: 'derive immutable numeric table' }];
  return HOSTS.flatMap(host => (phase === 'baselines' ? ['baseline-round-1', 'baseline-round-2'] : ['held-out-candidate'])
    .map(role => ({ host, role, name: numericJobName({ host, role }) })));
}
export function numericArtifactNames(slot, runId) {
  if (slot.role === 'anchor') return [`cli-performance-numeric-anchor-${runId}-1`];
  return [`cli-performance-numeric-${slot.host}-${slot.role}-${runId}-1`,
    ...(slot.host === 'linux-x86_64' ? [`cli-performance-numeric-linux-provider-${slot.role}-${runId}-1`] : [])];
}
export function numericTimestamp(value) {
  assert(typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z$/u.test(value));
  const milliseconds = Date.parse(value);
  assert(Number.isSafeInteger(milliseconds) && new Date(milliseconds).toISOString() === value.replace('Z', '.000Z'));
  return milliseconds;
}
export function parseNumericCampaignPlan(bytes, reviewedSha256) {
  assert(Buffer.isBuffer(bytes) && bytes.length > 0 && bytes.length <= MAX_DOCUMENT, 'bounded original campaign plan before hashing');
  digest(reviewedSha256);
  assert.equal(sha256(bytes), reviewedSha256, 'exact reviewed campaign selection bytes');
  const plan = parse(bytes);
  exact(plan, ['format', 'phase', 'runId', 'source', 'policySha256', 'workflows'], 'numeric campaign plan');
  assert.equal(plan.format, 'zryna.cli-performance-campaign-plan.v1'); numericSlots(plan.phase);
  assert(Number.isSafeInteger(plan.runId) && plan.runId > 0, 'actual platform numeric run ID');
  exact(plan.source, ['commit', 'tree', 'ref'], 'same-source initial calibration');
  digest(plan.source.commit, 40); digest(plan.source.tree, 40);
  assert(typeof plan.source.ref === 'string' && /^refs\/heads\/[A-Za-z0-9_./-]{1,200}$/u.test(plan.source.ref));
  assert(!plan.source.ref.includes('..') && !plan.source.ref.endsWith('/'));
  assert.equal(plan.policySha256, PROSPECTIVE_POLICY_SHA256, 'frozen prospective engineering policy');
  assert(Array.isArray(plan.workflows) && plan.workflows.length === NUMERIC_WORKFLOWS.length);
  plan.workflows.forEach((row, index) => {
    exact(row, ['path', 'bytes', 'sha256'], 'reviewed prospective workflow');
    assert.equal(row.path, NUMERIC_WORKFLOWS[index]); digest(row.sha256);
    assert(Number.isSafeInteger(row.bytes) && row.bytes > 0 && row.bytes <= 65536);
    assert.deepEqual({ bytes: row.bytes, sha256: row.sha256 }, NUMERIC_WORKFLOW_PINS[index], 'fixed reviewed workflow implementation');
  });
  assert.equal(canonical(plan), bytes.toString('utf8'));
  return plan;
}
export function requiredNumericSteps(slot) {
  if (slot.role === 'anchor') return [
    ['Guard fixed independent anchor phase', 'success'], ['Checkout exact observed head', 'success'],
    ['Frozen workflow dependencies outside measurement', 'success'],
    ['Reconcile completed baselines and derive fixed immutable table', 'success'],
    ['Retain independent anchor observations and failures', 'success'],
  ];
  const windows = slot.host === 'windows-x86_64';
  return [['Guard fixed prospective numeric collection', 'success'], ['Checkout exact observed head', 'success'],
    ['Frozen workflow dependencies outside measurement', 'success'],
    ['Verify completed numeric anchor before any release build or measurement', slot.role === 'held-out-candidate' ? 'success' : 'skipped'],
    ['Verify fixed collectors oracles and release capture controls', 'success'],
    ['Locked Rust dependency preparation outside builder and measurement', 'success'],
    ['Build Windows helper and run exactly eight controls once', windows ? 'success' : 'skipped'],
    ['Build fixed Windows parent affinity probe', windows ? 'success' : 'skipped'],
    ['Assemble observed operator inputs and collect one fresh round', 'success'],
    ['Retain Linux provider acquisition and loader observations', windows ? 'skipped' : 'success'],
    ['Retain all observations and failures', 'success']];
}
