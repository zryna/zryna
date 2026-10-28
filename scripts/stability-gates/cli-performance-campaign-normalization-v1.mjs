// Live owner composition only; returned declarations still carry no baseline/threshold authority.
import assert from 'node:assert/strict';
import { exact, MAX_DOCUMENT, sha256, canonical } from './input.mjs';
import { inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { inspectNumericCampaignSource, numericCampaignCollectorIdentity, readNumericCampaignOriginal }
  from './cli-performance-campaign-source-v1.mjs';
import { inspectNumericCampaignArchive, readNumericCampaignArchiveEntry } from './cli-performance-campaign-archive-v1.mjs';
import { inspectNumericCampaignHost } from './cli-performance-campaign-host-v1.mjs';
import { inspectNumericCampaignLinuxProvider } from './cli-performance-campaign-linux-provider-v1.mjs';
import { producerDocument } from './cli-performance-campaign-windows-host-v1.mjs';
import { parseMethodology } from './cli-performance-methodology-v4.mjs';
import { HOSTS, METHODS, ROLES } from './cli-performance-threshold-profile-v1.mjs';
import { numericSemanticTools, projectNumericRound } from './cli-performance-campaign-normalization-projection-v1.mjs';
export function inspectNumericCampaignRound(selection, sourceKey, keys) {
  assert.equal(arguments.length, 3);
  const phase = inspectNumericCampaignPhase(selection);
  assert(['baselines', 'heldout'].includes(phase.plan.phase));
  exact(keys, ['rawKey', 'hostKey', 'providerKey'], 'live round keys');
  const { rawKey, hostKey, providerKey } = keys;
  const raw = inspectNumericCampaignArchive(selection, sourceKey, rawKey);
  const host = inspectNumericCampaignHost(selection, sourceKey, rawKey, hostKey);
  assert.deepEqual(raw.reconciliations, { rawMetrics: true, releaseImage: true,
    pairedOraclesAndArchitecture: true, sourceFileBytes: true });
  assert.equal(host.hostFactBytesVerified, true); assert.equal(raw.phase, phase.plan.phase);
  assert.equal(host.host, raw.host); assert.equal(host.jobId, raw.jobId); assert.deepEqual(host.artifact, raw.artifact);
  let provider = null, calibration = null;
  if (raw.host === HOSTS[0]) {
    provider = inspectNumericCampaignLinuxProvider(selection, sourceKey, rawKey, providerKey);
    assert.equal(provider.providerOriginalsVerified, true);
  } else {
    assert.equal(raw.host, HOSTS[1]); assert.equal(providerKey, null);
    assert.equal(host.nativeParentBindingVerified, true);
    const bytes = readNumericCampaignArchiveEntry(selection, sourceKey, rawKey,
      'windows-job-metrics-calibration/toolchain-before-build.json', MAX_DOCUMENT);
    assert.equal(bytes.length, host.toolInputs.calibration.bytes);
    assert.equal(sha256(bytes), host.toolInputs.calibration.sha256); calibration = producerDocument(bytes);
  }
  return { raw, host, tools: numericSemanticTools({ raw, host, provider, calibration }) };
}
export function normalizeNumericCampaignBaselines(selection, sourceKey, roundKeys) {
  assert.equal(arguments.length, 3, 'fixed opaque phase/source/raw/host/provider owners; no projection override');
  const phase = inspectNumericCampaignPhase(selection), source = inspectNumericCampaignSource(selection, sourceKey);
  assert.equal(phase.plan.phase, 'baselines');
  assert(Array.isArray(roundKeys) && roundKeys.length === 4, 'all four original baseline slots');
  const records = roundKeys.map(keys => inspectNumericCampaignRound(selection, sourceKey, keys));
  const ordered = HOSTS.flatMap(host => ROLES.slice(0, 2).map(role => {
    const matches = records.filter(record => record.raw.host === host && record.raw.round === role);
    assert.equal(matches.length, 1, 'exactly one of every fixed host/role slot'); return matches[0];
  }));
  const collector = numericCampaignCollectorIdentity(selection, sourceKey);
  const catalogue = parseMethodology(readNumericCampaignOriginal(selection, sourceKey,
    'tests/cli-performance-methodology-v4.json', MAX_DOCUMENT));
  const catalogueSha256 = sha256(canonical(catalogue));
  const tools = HOSTS.map((host, index) => {
    const [first, second] = ordered.slice(index * 2, index * 2 + 2);
    assert.deepEqual(first.tools, second.tools, 'same semantic tool selection across both baselines');
    assert.equal(first.host.classId, second.host.classId); assert.deepEqual(first.host.facts, second.host.facts);
    assert.deepEqual(first.host.acceptedUnknownFacts, second.host.acceptedUnknownFacts);
    assert.equal(first.raw.image.sha256, second.raw.image.sha256, 'same release image, distinct original build records');
    assert(Date.parse(first.raw.completedAt) < Date.parse(second.raw.startedAt), 'serial per-host baseline slots');
    return first.tools;
  });
  const toolchainSha256 = sha256(canonical(tools)), expectedSource = { commit: source.commit, tree: source.tree };
  const rounds = ordered.map(({ raw, host }) => {
    assert.equal(raw.catalogueSha256, catalogueSha256);
    assert.deepEqual({ commit: raw.source.commit, tree: raw.source.tree }, expectedSource);
    return projectNumericRound({ raw, host, source: expectedSource, collectorSha256: collector.collectorSha256,
      toolchainSha256 });
  });
  const hosts = HOSTS.map((host, index) => {
    const row = ordered[index * 2], build = { class: 'release', cliSha256: row.raw.image.sha256 };
    return { host, classId: row.host.classId, facts: structuredClone(row.host.facts),
      acceptedUnknownFacts: structuredClone(row.host.acceptedUnknownFacts), wallMethod: METHODS[host].wall,
      memoryMethod: METHODS[host].memory, referenceBuild: { ...build }, candidateBuild: { ...build } };
  });
  return { format: 'zryna.cli-performance-normalized-baseline-declarations.v1', source: expectedSource,
    catalogue, catalogueSha256, collector, toolchainSha256, tools, hosts, rounds,
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    baselineAdmitted: false, thresholdAuthority: null, closureStatus: 'blocked',
    scope: 'Original owned bytes and complete sample projection; no execution provenance, temporal anchor or numeric table admission' };
}
