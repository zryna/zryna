// Fixed original acquisition and opaque-owner composition; no copied report can replace a round.
import assert from 'node:assert/strict';
import { sha256 } from './input.mjs';
import { downloadPublicationArchive } from '../m4/retain-publication.mjs';
import { PERFORMANCE_REPOSITORY } from './cli-performance-hosted-expectations.mjs';
import { linkNumericCampaignPhase, inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { bindNumericCampaignSource, readNumericCampaignOriginal } from './cli-performance-campaign-source-v1.mjs';
import { reconcileNumericCampaignArchive, closeNumericCampaignArchive } from './cli-performance-campaign-archive-v1.mjs';
import { reconcileNumericCampaignHost } from './cli-performance-campaign-host-v1.mjs';
import { reconcileNumericCampaignLinuxProvider } from './cli-performance-campaign-linux-provider-v1.mjs';
import { normalizeNumericCampaignBaselines } from './cli-performance-campaign-normalization-v1.mjs';
import { originalNumericPlan } from './cli-performance-anchor-context-v1.mjs';
export async function downloadNumericOriginal(artifact, maximum) {
  assert(Number.isSafeInteger(artifact.bytes) && artifact.bytes >= 22 && artifact.bytes <= maximum,
    'new role byte bound enforced before existing downloader');
  assert([1024 * 1024, 128 * 1024 * 1024].includes(maximum));
  return downloadPublicationArchive(PERFORMANCE_REPOSITORY, artifact,
    { token: process.env.GH_TOKEN ?? process.env.GITHUB_TOKEN });
}
export async function loadNumericAnchorBaselines(root, source, runId, api) {
  const bytes = originalNumericPlan(root, source, 'baselines', runId);
  const selection = await linkNumericCampaignPhase(bytes, sha256(bytes), api);
  const phase = inspectNumericCampaignPhase(selection), sourceKey = bindNumericCampaignSource(selection, root);
  const keys = [];
  try {
    for (const job of phase.jobs) {
      const primary = await downloadNumericOriginal(job.artifacts[0], 128 * 1024 * 1024);
      const rawKey = reconcileNumericCampaignArchive(selection, sourceKey, job.artifacts[0].id, primary);
      const row = { rawKey, hostKey: null, providerKey: null }; keys.push(row);
      row.hostKey = reconcileNumericCampaignHost(selection, sourceKey, rawKey);
      if (job.host === 'linux-x86_64') {
        const provider = await downloadNumericOriginal(job.artifacts[1], 1024 * 1024);
        row.providerKey = reconcileNumericCampaignLinuxProvider(selection, sourceKey, rawKey, provider);
      }
    }
    const normalized = normalizeNumericCampaignBaselines(selection, sourceKey, keys);
    const policyBytes = readNumericCampaignOriginal(selection, sourceKey,
      'tests/cli-performance-prospective-policy-v1.json', 262144);
    return { phase, normalized, policyBytes };
  } finally {
    for (const row of keys) closeNumericCampaignArchive(selection, sourceKey, row.rawKey);
  }
}
