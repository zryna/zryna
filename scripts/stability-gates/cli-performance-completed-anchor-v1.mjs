// Shared original completed-anchor replay; projections alone issue no capability.
import assert from 'node:assert/strict';
import { parse, sha256 } from './input.mjs';
import { linkNumericCampaignPhase, inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry,
  closePerformanceArchive } from './cli-performance-archive.mjs';
import { numericTimestamp } from './cli-performance-campaign-plan-v1.mjs';
import { NUMERIC_ANCHOR_FILES, decodeNumericAnchorInput,
  deriveNumericAnchorDocuments } from './cli-performance-anchor-documents-v1.mjs';
import { NUMERIC_API_PREFIX, originalNumericPlan } from './cli-performance-anchor-context-v1.mjs';
import { downloadNumericOriginal, loadNumericAnchorBaselines } from './cli-performance-anchor-baselines-v1.mjs';
export async function completedNumericPhase(root, source, phase, runId, api) {
  const bytes = originalNumericPlan(root, source, phase, runId);
  const key = await linkNumericCampaignPhase(bytes, sha256(bytes), api);
  return inspectNumericCampaignPhase(key);
}
export async function completedNumericRunTime(phase, api) {
  const run = await api(`${NUMERIC_API_PREFIX}/actions/runs/${phase.plan.runId}`);
  assert.equal(run.id, phase.plan.runId); assert.equal(run.run_attempt, 1);
  assert.equal(run.head_sha, phase.plan.source.commit); assert.equal(run.head_branch, phase.plan.source.ref.slice(11));
  assert.equal(run.path, `.github/workflows/cli-performance-${phase.plan.phase}-v1.yml`);
  assert.equal(run.event, 'workflow_dispatch'); assert.equal(run.status, 'completed'); assert.equal(run.conclusion, 'success');
  const terminal = numericTimestamp(run.updated_at);
  for (const job of phase.jobs) assert(numericTimestamp(job.completedAt) <= terminal);
  return run.updated_at; // Conservative server terminal bound; never infer from a declaration clock.
}
export async function loadCompletedNumericAnchor(root, source, anchorRunId, api) {
  const phase = await completedNumericPhase(root, source, 'anchor', anchorRunId, api);
  assert.equal(phase.jobs.length, 1); const job = phase.jobs[0], artifact = job.artifacts[0];
  const completedAt = await completedNumericRunTime(phase, api);
  const archiveBytes = await downloadNumericOriginal(artifact, 1024 * 1024);
  const archive = openPerformanceArchive(archiveBytes, artifact, 'linux-provider'); // Reuse unchanged 1MiB/256KiB bounds.
  let files;
  try {
    assert.deepEqual(listPerformanceArchive(archive).map(row => row.path).sort(), NUMERIC_ANCHOR_FILES);
    files = new Map(NUMERIC_ANCHOR_FILES.map(path => [path, readPerformanceArchiveEntry(archive, path)]));
  } finally { closePerformanceArchive(archive); }
  const original = decodeNumericAnchorInput(files.get('anchor-input.json'));
  assert.deepEqual(original.source, source);
  assert.deepEqual(original.anchor, { runId: anchorRunId, jobId: job.jobId, startedAt: job.startedAt });
  const loaded = await loadNumericAnchorBaselines(root, source, original.baselineRunId, api);
  const expected = deriveNumericAnchorDocuments(loaded.normalized, loaded.phase, original.anchor, loaded.policyBytes);
  for (const name of NUMERIC_ANCHOR_FILES)
    assert(files.get(name).equals(expected.get(name)), `original ${name} independently rederived from all four live owners`);
  const baselineCompletedAt = await completedNumericRunTime(loaded.phase, api);
  const anchorRun = await api(`${NUMERIC_API_PREFIX}/actions/runs/${anchorRunId}`);
  assert(numericTimestamp(baselineCompletedAt) < numericTimestamp(anchorRun.created_at));
  assert.deepEqual(await completedNumericPhase(root, source, 'baselines', original.baselineRunId, api), loaded.phase);
  assert.deepEqual(await completedNumericPhase(root, source, 'anchor', anchorRunId, api), phase);
  assert.equal(await completedNumericRunTime(phase, api), completedAt);
  return { phase, job, artifact, completedAt, loaded, original, files };
}
export function requireNumericAnchorOrder(complete, starts) {
  const { artifact, job, completedAt } = complete;
  for (const finish of [artifact.createdAt, job.completedAt, completedAt])
    for (const start of starts) assert(numericTimestamp(finish) < numericTimestamp(start),
      'actual original anchor artifact and completion precede heldout dispatch/run/job');
}
export function projectCompletedNumericAnchor(complete, before) {
  requireNumericAnchorOrder(complete, [before.createdAt, before.runStartedAt, before.startedAt]);
  const { job, artifact, completedAt, loaded, original, files } = complete;
  const anchorRunId = complete.phase.plan.runId;
  return { format: 'zryna.cli-performance-original-numeric-anchor.v1',
    profile: parse(files.get('numeric-profile.json')), tools: original.tools, collector: original.collector,
    baselineSource: loaded.normalized.source, baselineRunId: original.baselineRunId,
    anchor: { runId: anchorRunId, jobId: job.jobId, artifact, completedAt,
      jobCompletedAt: job.completedAt, declaredRecordedAt: new Date(job.startedAt).toISOString() },
    current: before, sourceFileBytesVerified: true, tableOriginalBytesVerified: true, tableTemporalOrderVerified: true,
    preMeasurementClassVerified: false, sourceAuthenticated: false, proofQualified: false,
    performanceQualified: false, baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked',
    scope: 'Original table and server chronology only; current parent/class/tool admission and final M7 closure remain separate' };
}
