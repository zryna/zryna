// Original-byte joins only. The completed coordinator must independently rederive/authenticate the anchor.
import assert from 'node:assert/strict';
import { canonical, exact, MAX_DOCUMENT, parse, sha256 } from './input.mjs';
import { inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { inspectNumericCampaignSource, readNumericCampaignOriginal, numericCampaignCollectorIdentity }
  from './cli-performance-campaign-source-v1.mjs';
import { inspectNumericCampaignArchive, readNumericCampaignArchiveEntry } from './cli-performance-campaign-archive-v1.mjs';
import { inspectNumericCampaignHost, validateNumericOriginalHost } from './cli-performance-campaign-host-v1.mjs';
import { inspectNumericCampaignLinuxProvider } from './cli-performance-campaign-linux-provider-v1.mjs';
import { bindWindowsHostObservations, COMPANION_FLAGS, producerDocument } from './cli-performance-campaign-windows-host-v1.mjs';
import { numericSemanticTools } from './cli-performance-campaign-normalization-projection-v1.mjs';
import { compareHeldoutHost, compareHeldoutTools, HELDOUT_FLAGS } from './cli-performance-heldout-comparison-v1.mjs';
const PREFIX = 'm418-hosted-observations/capture/', PRE = PREFIX + 'numeric-premeasurement/';
export function replayNumericHeldoutOriginals(selection, sourceKey, rawKey, hostKey, providerKey, anchor) {
  assert.equal(arguments.length, 6, 'fixed original heldout owners; no reader or metric override');
  const phase = inspectNumericCampaignPhase(selection), source = inspectNumericCampaignSource(selection, sourceKey);
  const raw = inspectNumericCampaignArchive(selection, sourceKey, rawKey);
  const host = inspectNumericCampaignHost(selection, sourceKey, rawKey, hostKey); // Preserves original before AND after validation.
  assert.equal(phase.plan.phase, 'heldout'); assert.equal(raw.phase, 'heldout'); assert.equal(raw.round, 'held-out-candidate');
  const expectedSource = { commit: source.commit, tree: source.tree }, bindings = [];
  assert.deepEqual({ commit: raw.source.commit, tree: raw.source.tree }, expectedSource);
  assert.equal(anchor.current.runId, raw.run.id); assert.equal(anchor.current.jobId, raw.jobId);
  assert.deepEqual(anchor.current.source, raw.source);
  const read = (path, maximum) => {
    const bytes = readNumericCampaignArchiveEntry(selection, sourceKey, rawKey, path, maximum);
    bindings.push({ path, bytes: bytes.length, sha256: sha256(bytes) }); return bytes;
  };
  const json = path => parse(read(path, MAX_DOCUMENT));
  const operator = json(PREFIX + 'operator-input.json'), observations = [];
  const windows = raw.host === 'windows-x86_64';
  for (const directory of [PREFIX, PRE]) {
    const bytes = read(directory + 'host-before.json', MAX_DOCUMENT), original = parse(bytes);
    const companion = json(directory + 'numeric-host-before.json');
    validateNumericOriginalHost(original, operator, phase, windows);
    assert.equal(original.phase, 'before'); assert.equal(original.platform, raw.host);
    exact(companion, ['format', 'phase', 'platform', 'source', 'collectorNodePid', 'original', 'kernel',
      ...(windows ? ['native'] : []), ...Object.keys(COMPANION_FLAGS)], 'original before companion');
    assert.equal(companion.format, 'zryna.cli-performance-numeric-host-companion.v1');
    assert.equal(companion.phase, 'before'); assert.equal(companion.platform, raw.host);
    assert.deepEqual(companion.source, expectedSource); assert.deepEqual(companion.kernel, original.kernel);
    assert.deepEqual(companion.original, { path: 'host-before.json', bytes: bytes.length, sha256: sha256(bytes) });
    assert.equal(companion.collectorNodePid, host.parent.pid);
    for (const [key, value] of Object.entries(COMPANION_FLAGS)) assert.equal(companion[key], value);
    observations.push({ host: original, companion });
  }
  assert.equal(observations[0].host.cpu.logicalCount, observations[1].host.cpu.logicalCount);
  assert.equal(observations[0].host.memory.totalBytes, observations[1].host.memory.totalBytes);
  if (!windows) {
    // Original clock consistency rejects a copied before pair; it supplies no native birth proof.
    const times = [...observations.map(value => value.host.observedAt), json(PREFIX + 'host-after.json').observedAt].map(Date.parse);
    assert(times.every(Number.isFinite) && times[0] < times[1] && times[1] < times[2],
      'original Linux host clocks bracket the fresh premeasurement observation');
  }
  if (windows) {
    // Explicit logical second-observation mapping only: original records stay phase BEFORE in a distinct directory.
    // Every read below records its PHYSICAL archive path, never a fabricated after filename or rewritten payload.
    const logicalAfter = PREFIX + 'numeric-host-after-affinity', physicalBefore = PRE + 'numeric-host-before-affinity';
    const nativeRead = (path, maximum) => read(path.startsWith(logicalAfter) ? physicalBefore + path.slice(logicalAfter.length) : path, maximum);
    const native = bindWindowsHostObservations({ source: expectedSource, operator, tools: raw.tools,
      companions: observations.map(value => value.companion), read: nativeRead,
      readOriginal: (path, maximum) => readNumericCampaignOriginal(selection, sourceKey, path, maximum) });
    assert.deepEqual(native.parent, host.parent);
    const after = json(PREFIX + 'numeric-host-after.json').native.decoded.originalRecord;
    const [beforeChild, freshChild] = native.decoded.map(value => BigInt(value.originalRecord.childCreated));
    assert(beforeChild < freshChild && freshChild < BigInt(after.childCreated), 'original C births bracket the fresh premeasurement observation');
  }
  let provider = null, calibration = null;
  if (windows) {
    assert.equal(providerKey, null);
    calibration = producerDocument(read('windows-job-metrics-calibration/toolchain-before-build.json', MAX_DOCUMENT));
  } else provider = inspectNumericCampaignLinuxProvider(selection, sourceKey, rawKey, providerKey);
  const tools = numericSemanticTools({ raw, host, provider, calibration });
  const collector = numericCampaignCollectorIdentity(selection, sourceKey);
  assert.deepEqual(collector, anchor.collector);
  const toolSummary = compareHeldoutTools(anchor, raw.host, expectedSource, tools, raw.image.sha256, collector.collectorSha256);
  const profileSha256 = sha256(canonical(anchor.profile));
  for (const [index, name] of ['numeric-gate-before.json', 'numeric-gate-premeasurement.json'].entries()) {
    const summary = compareHeldoutHost(anchor, observations[index], expectedSource);
    assert.equal(summary.classId, host.classId); assert.equal(summary.factsSha256, sha256(canonical(host.facts)));
    assert.deepEqual(json(PREFIX + name), { format: 'zryna.cli-performance-heldout-premeasurement.v1',
      phase: index ? 'immediately-before-timed-round' : 'before-release-build', host: summary,
      ...(index ? { tools: toolSummary } : {}), source: expectedSource, anchor: anchor.anchor, profileSha256,
      observedClassMatched: true, ...(index ? { observedToolsMatched: true } : {}), ...HELDOUT_FLAGS },
    'original gate witness exactly matches freshly replayed original host/tools/source/anchor bytes');
  }
  return { format: 'zryna.cli-performance-heldout-originals-replay.v1', host: raw.host, source: expectedSource,
    jobId: raw.jobId, artifact: raw.artifact, profileSha256, originalBindings: bindings,
    observationPhases: ['original-before', 'premeasurement-before'], premeasurementOriginalsVerified: true,
    scope: 'Original byte joins only; anchor authenticity/server chronology and numeric/final admission belong to the completed coordinator',
    ...HELDOUT_FLAGS };
}
