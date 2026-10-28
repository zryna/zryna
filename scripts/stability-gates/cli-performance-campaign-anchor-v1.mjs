// Fixed production coordinator. A copied four-file table or verification report cannot mint this key.
import assert from 'node:assert/strict';
import { isAbsolute } from 'node:path';
import { realpathSync } from 'node:fs';
import { canonical, exact } from './input.mjs';
import { snapshot } from './source.mjs';
import { githubApi } from '../m4/hosted-api.mjs';
import { numericTimestamp } from './cli-performance-campaign-plan-v1.mjs';
import { loadNumericAnchorBaselines } from './cli-performance-anchor-baselines-v1.mjs';
import { numericRuntimeContext, observeCurrentNumericJob } from './cli-performance-anchor-context-v1.mjs';
import { deriveNumericAnchorDocuments } from './cli-performance-anchor-documents-v1.mjs';
import { completedNumericPhase, completedNumericRunTime, loadCompletedNumericAnchor,
  projectCompletedNumericAnchor, requireNumericAnchorOrder } from './cli-performance-completed-anchor-v1.mjs';
const anchors = new WeakMap();
function rootPath(root) {
  assert(typeof root === 'string' && isAbsolute(root)); return realpathSync.native(root);
}
function id(value) { assert(Number.isSafeInteger(value) && value > 0); return value; }
function unchangedSource(context) { assert.deepEqual(snapshot(context.root), context.actual, 'same clean immutable checkout'); }
export async function buildNumericCampaignAnchor(input) {
  assert.equal(arguments.length, 1); exact(input, ['root', 'baselineRunId'], 'fixed anchor builder');
  const root = rootPath(input.root), baselineRunId = id(input.baselineRunId);
  const context = numericRuntimeContext(root, 'anchor', 'linux-x86_64', baselineRunId);
  const api = githubApi(process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN);
  const before = await observeCurrentNumericJob(context, api);
  const loaded = await loadNumericAnchorBaselines(root, context.source, baselineRunId, api);
  const baselineCompletedAt = await completedNumericRunTime(loaded.phase, api);
  assert(numericTimestamp(baselineCompletedAt) < numericTimestamp(before.createdAt),
    'anchor dispatch follows completed original baselines');
  const files = deriveNumericAnchorDocuments(loaded.normalized, loaded.phase,
    { runId: before.runId, jobId: before.jobId, startedAt: before.startedAt }, loaded.policyBytes);
  assert.deepEqual(await completedNumericPhase(root, context.source, 'baselines', baselineRunId, api), loaded.phase);
  assert.deepEqual(await observeCurrentNumericJob(context, api), before); unchangedSource(context);
  return files; // Declarations only. This ongoing job has no completed immutable artifact yet.
}
export async function verifyNumericCampaignAnchor(input) {
  assert.equal(arguments.length, 1); exact(input, ['root', 'anchorRunId', 'platform'], 'fixed anchor verifier');
  const root = rootPath(input.root), anchorRunId = id(input.anchorRunId);
  const context = numericRuntimeContext(root, 'heldout', input.platform, anchorRunId);
  const api = githubApi(process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN);
  const before = await observeCurrentNumericJob(context, api);
  const complete = await loadCompletedNumericAnchor(root, context.source, anchorRunId, api);
  requireNumericAnchorOrder(complete, [before.createdAt, before.runStartedAt, before.startedAt]);
  assert.deepEqual(await observeCurrentNumericJob(context, api), before); unchangedSource(context);
  const value = projectCompletedNumericAnchor(complete, before);
  // Private bounded snapshot; no returned object exposes the stored profile or original document buffers.
  const key = Object.freeze({}); anchors.set(key, { context, value: JSON.parse(canonical(value)) }); return key;
}
export function inspectNumericCampaignAnchor(key) {
  assert.equal(arguments.length, 1); const value = anchors.get(key);
  assert(value, 'live original numeric anchor key required; copied reports grant no authority');
  const { context } = value;
  assert.equal(process.env.GITHUB_RUN_ID, String(context.runId)); assert.equal(process.env.GITHUB_RUN_ATTEMPT, '1');
  assert.equal(process.env.GITHUB_SHA, context.source.commit); assert.equal(process.env.GITHUB_REF, context.source.ref);
  assert.equal(process.env.M418_HOST, context.host); unchangedSource(context);
  return structuredClone(value.value);
}
export function closeNumericCampaignAnchor(key) {
  assert.equal(arguments.length, 1); assert(anchors.has(key), 'live anchor key'); anchors.delete(key);
}
