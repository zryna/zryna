// Original provider bytes only. Live ownership is separate from pure historical replay.
import assert from 'node:assert/strict';
import { sha256 } from './input.mjs';
import { inspectNumericCampaignArchive, readNumericCampaignArchiveEntry } from './cli-performance-campaign-archive-v1.mjs';
import { replayNumericLinuxProviderOriginals } from './cli-performance-campaign-linux-provider-originals-v1.mjs';
export { replayNumericLinuxProviderOriginals } from './cli-performance-campaign-linux-provider-originals-v1.mjs';
const owned = new WeakMap(), PREFIX = 'm418-hosted-observations/capture/';
export function reconcileNumericCampaignLinuxProvider(selection, sourceKey, rawKey, bytes) {
  assert.equal(arguments.length, 4, 'fixed provider owner; no caller reader or artifact override');
  assert(Buffer.isBuffer(bytes) && bytes.length >= 22 && bytes.length <= 1024 * 1024,
    'bounded original Linux provider ZIP before private copy');
  const raw = inspectNumericCampaignArchive(selection, sourceKey, rawKey);
  assert.equal(raw.host, 'linux-x86_64'); assert(raw.providerArtifact, 'same selected Linux provider required');
  const read = (name, maximum) => readNumericCampaignArchiveEntry(selection, sourceKey, rawKey, PREFIX + name, maximum);
  const capture = read('round/capture.json', 262144);
  assert.equal(sha256(capture), raw.originals.capture.sha256);
  const result = replayNumericLinuxProviderOriginals({ artifact: raw.providerArtifact, capture,
    binding: read('provider-binding.json', 16384), admission: read('round/provider-admission.json', 16384),
    receipt: read('round/provider-runtime-receipt.json', 65536) }, bytes);
  const key = Object.freeze({});
  owned.set(key, { selection, sourceKey, rawKey, projection: { ...result,
    format: 'zryna.cli-performance-campaign-linux-provider.v1', reviewedPlanSha256: raw.reviewedPlanSha256,
    phase: raw.phase, run: raw.run, source: raw.source, round: raw.round, jobId: raw.jobId,
    checkRunId: raw.checkRunId, primaryArtifact: raw.artifact } });
  return key;
}
export function inspectNumericCampaignLinuxProvider(selection, sourceKey, rawKey, key) {
  assert.equal(arguments.length, 4, 'fixed same-phase/source/raw provider inspector');
  const value = owned.get(key);
  assert(value && value.selection === selection && value.sourceKey === sourceKey && value.rawKey === rawKey,
    'own provider key and SAME live phase/source/raw keys required');
  inspectNumericCampaignArchive(selection, sourceKey, rawKey); // Closed raw lifetimes cannot be resurrected.
  return structuredClone(value.projection);
}
