// Completed original campaign joins. No ongoing-job environment or copied report can mint this proof.
import assert from 'node:assert/strict';
import { realpathSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import { canonical, exact, parse, sha256 } from './input.mjs';
import { snapshot } from './source.mjs';
import { githubApi } from '../m4/hosted-api.mjs';
import { linkNumericCampaignPhase, inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { originalNumericPlan, NUMERIC_API_PREFIX } from './cli-performance-anchor-context-v1.mjs';
import { bindNumericCampaignSource, numericCampaignCollectorIdentity } from './cli-performance-campaign-source-v1.mjs';
import { reconcileNumericCampaignArchive, closeNumericCampaignArchive } from './cli-performance-campaign-archive-v1.mjs';
import { reconcileNumericCampaignHost } from './cli-performance-campaign-host-v1.mjs';
import { reconcileNumericCampaignLinuxProvider } from './cli-performance-campaign-linux-provider-v1.mjs';
import { downloadNumericOriginal } from './cli-performance-anchor-baselines-v1.mjs';
import { inspectNumericCampaignRound } from './cli-performance-campaign-normalization-v1.mjs';
import { projectNumericRound } from './cli-performance-campaign-normalization-projection-v1.mjs';
import { completedNumericPhase, completedNumericRunTime, loadCompletedNumericAnchor,
  projectCompletedNumericAnchor, requireNumericAnchorOrder } from './cli-performance-completed-anchor-v1.mjs';
import { replayNumericHeldoutOriginals } from './cli-performance-heldout-replay-v1.mjs';
import { compareProspective } from './cli-performance-regression-v1.mjs';
import { HOSTS } from './cli-performance-threshold-profile-v1.mjs';
const proofs = new WeakMap();
export async function reconcileCompletedNumericCampaign(input) {
  assert.equal(arguments.length, 1); exact(input, ['root', 'anchorRunId', 'heldoutRunId'], 'fixed completed campaign');
  assert(typeof input.root === 'string' && isAbsolute(input.root)); const root = realpathSync.native(input.root);
  for (const id of [input.anchorRunId, input.heldoutRunId]) assert(Number.isSafeInteger(id) && id > 0);
  assert.notEqual(input.anchorRunId, input.heldoutRunId);
  const actual = snapshot(root), api = githubApi(process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN);
  const anchorRun = await api(`${NUMERIC_API_PREFIX}/actions/runs/${input.anchorRunId}`);
  assert.equal(anchorRun.head_sha, actual.commit);
  const source = { commit: actual.commit, tree: actual.tree, ref: `refs/heads/${anchorRun.head_branch}` };
  const complete = await loadCompletedNumericAnchor(root, source, input.anchorRunId, api);
  const bytes = originalNumericPlan(root, source, 'heldout', input.heldoutRunId);
  const selection = await linkNumericCampaignPhase(bytes, sha256(bytes), api);
  const phase = inspectNumericCampaignPhase(selection), sourceKey = bindNumericCampaignSource(selection, root);
  const endedAt = await completedNumericRunTime(phase, api);
  const run = await api(`${NUMERIC_API_PREFIX}/actions/runs/${input.heldoutRunId}`);
  requireNumericAnchorOrder(complete, [run.created_at, run.run_started_at, ...phase.jobs.map(job => job.startedAt)]);
  assert.equal(new Set([complete.loaded.phase.plan.runId, input.anchorRunId, input.heldoutRunId]).size, 3);
  const profile = parse(complete.files.get('numeric-profile.json'));
  const collector = numericCampaignCollectorIdentity(selection, sourceKey);
  assert.deepEqual(collector, complete.original.collector);
  const heldout = [], originals = [];
  for (const job of phase.jobs) {
    const primary = await downloadNumericOriginal(job.artifacts[0], 128 * 1024 * 1024);
    const rawKey = reconcileNumericCampaignArchive(selection, sourceKey, job.artifacts[0].id, primary);
    try {
      const hostKey = reconcileNumericCampaignHost(selection, sourceKey, rawKey);
      const providerKey = job.host === HOSTS[0] ? reconcileNumericCampaignLinuxProvider(selection, sourceKey, rawKey,
        await downloadNumericOriginal(job.artifacts[1], 1024 * 1024)) : null;
      const { raw, host, tools } = inspectNumericCampaignRound(selection, sourceKey, { rawKey, hostKey, providerKey });
      assert.deepEqual(tools, complete.original.tools[HOSTS.indexOf(job.host)], 'same anchored semantic tools');
      const current = { phase: 'heldout', host: job.host, source, runId: run.id, jobId: job.jobId,
        createdAt: run.created_at, runStartedAt: run.run_started_at, startedAt: job.startedAt };
      const replay = replayNumericHeldoutOriginals(selection, sourceKey, rawKey, hostKey, providerKey,
        projectCompletedNumericAnchor(complete, current));
      assert.equal(replay.premeasurementOriginalsVerified, true);
      const round = projectNumericRound({ raw, host, source: { commit: actual.commit, tree: actual.tree },
        collectorSha256: collector.collectorSha256, toolchainSha256: profile.toolchainSha256 });
      heldout.push(round); originals.push({ host: job.host, jobId: job.jobId, artifacts: job.artifacts, replay });
    } finally { closeNumericCampaignArchive(selection, sourceKey, rawKey); }
  }
  const rounds = HOSTS.flatMap((host, index) => {
    const selected = heldout.filter(round => round.host === host); assert.equal(selected.length, 1);
    return [...complete.loaded.normalized.rounds.slice(index * 2, index * 2 + 2), selected[0]];
  });
  const comparison = compareProspective(profile, rounds, complete.loaded.normalized.catalogue);
  for (const selected of [phase, complete.phase, complete.loaded.phase]) {
    assert.deepEqual(await completedNumericPhase(root, source, selected.plan.phase, selected.plan.runId, api), selected);
    const terminal = await completedNumericRunTime(selected, api);
    if (selected === phase) assert.equal(terminal, endedAt);
    if (selected === complete.phase) assert.equal(terminal, complete.completedAt);
  }
  assert.deepEqual(snapshot(root), actual, 'same clean immutable source after all original joins');
  if (comparison.status !== 'within-declared-limits')
    throw Object.assign(new Error('original campaign exceeds frozen numeric regression limits'), { numericComparison: comparison });
  const value = { format: 'zryna.cli-performance-completed-original-campaign.v1', source,
    phases: { baselines: complete.original.baselineRunId, anchor: input.anchorRunId, heldout: input.heldoutRunId },
    anchor: { jobId: complete.job.jobId, artifact: complete.artifact, completedAt: complete.completedAt },
    originals, profileSha256: comparison.profileSha256, collector, comparison,
    counters: { comparedCells: comparison.cells.length, rounds: comparison.inputs.length,
      warmups: comparison.inputs.reduce((n, row) => n + row.warmups, 0),
      measuredAttempts: comparison.inputs.reduce((n, row) => n + row.measuredAttempts, 0) },
    scopedOriginalCampaignVerified: true, scopedNumericRegressionPassed: true,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked',
    pending: ['Full M7 compatibility/security/cleanup/source qualification and final closure remain separate'] };
  const key = Object.freeze({});
  proofs.set(key, { root, actual, value: JSON.parse(canonical(JSON.parse(JSON.stringify(value)))) }); return key;
}
export function inspectCompletedNumericCampaign(key) {
  assert.equal(arguments.length, 1); const own = proofs.get(key);
  assert(own, 'own completed-campaign key required; copied reports cannot qualify');
  assert.deepEqual(snapshot(own.root), own.actual); return structuredClone(own.value);
}
export function closeCompletedNumericCampaign(key) {
  assert.equal(arguments.length, 1); assert(proofs.has(key), 'own completed-campaign key'); proofs.delete(key);
}
