// A live phase plus independently owned source bytes; no host/table/performance admission.
import assert from 'node:assert/strict';
import { MAX_DOCUMENT, MAX_LOG, parse, portable, sha256 } from './input.mjs';
import { inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { inspectNumericCampaignSource, readNumericCampaignOriginal, reconcileNumericCampaignCensuses }
  from './cli-performance-campaign-source-v1.mjs';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry, closePerformanceArchive }
  from './cli-performance-archive.mjs';
import { parseMethodology } from './cli-performance-methodology-v4.mjs';
import { replayLinuxRound } from './cli-performance-linux-replay.mjs';
import { replayWindowsRound } from './cli-performance-windows-replay.mjs';
import { replayNumericReleaseImage } from './release-image-replay.mjs';
import { replayOracleArchitecture } from './cli-performance-oracle-replay.mjs';
import { retainNumericArchiveSamples } from './cli-performance-campaign-samples-v1.mjs';
const owned = new WeakMap(), PREFIX = 'm418-hosted-observations/capture/';
function originalMaterials(document) {
  const pins = new Map(document.materials.map(pin => [pin.path, pin]));
  for (const family of document.families) for (const pin of [...family.sources, family.oracle.material]) {
    if (pins.has(pin.path)) assert.deepEqual(pins.get(pin.path), pin);
    pins.set(pin.path, pin);
  }
  return [...pins.values()].map(pin => ({ path: pin.path, bytes: pin.bytes, sha256: pin.sha256 }));
}
export function reconcileNumericCampaignArchive(selection, sourceKey, artifactId, bytes) {
  assert.equal(arguments.length, 4, 'fixed archive owner; no caller reader, metric or source overrides');
  assert(Buffer.isBuffer(bytes) && bytes.length >= 22 && bytes.length <= 128 * 1024 * 1024,
    'bounded original primary ZIP before private copy');
  const platform = inspectNumericCampaignPhase(selection);
  assert(['baselines', 'heldout'].includes(platform.plan.phase), 'measurement phase only');
  const source = inspectNumericCampaignSource(selection, sourceKey);
  assert.equal(source.commit, platform.plan.source.commit); assert.equal(source.tree, platform.plan.source.tree);
  assert(Number.isSafeInteger(artifactId) && artifactId > 0);
  const matches = platform.jobs.filter(job => job.artifacts[0].id === artifactId);
  assert.equal(matches.length, 1, 'exact live selected primary, never provider/anchor');
  const job = matches[0], artifact = job.artifacts[0];
  assert.equal(bytes.length, artifact.bytes, 'same live selected original byte count');
  const image = Buffer.alloc(bytes.length); image.set(bytes); // Private non-pooled snapshot before replay.
  const key = openPerformanceArchive(image, artifact, 'primary');
  let retained = false;
  try {
    const entries = new Map(listPerformanceArchive(key).map(entry => [entry.path, entry]));
    const read = (relative, requested) => {
      portable(relative);
      const maximum = relative.endsWith('.metadata.raw') ? 4096 : relative.endsWith('.json') ? MAX_DOCUMENT : MAX_LOG;
      const bound = requested === undefined ? maximum : requested;
      assert(Number.isSafeInteger(bound) && bound >= 0 && bound <= 64 * 1024 * 1024);
      const entry = entries.get(PREFIX + relative);
      assert(entry && entry.bytes <= bound, `bounded original numeric capture entry ${relative}`);
      return readPerformanceArchiveEntry(key, PREFIX + relative);
    };
    const record = (relative, canonical = true) => {
      const raw = read(relative, MAX_DOCUMENT);
      return { value: canonical ? parse(raw) : JSON.parse(raw.toString('utf8')),
        binding: { path: PREFIX + relative, bytes: raw.length, sha256: sha256(raw) } };
    };
    const catalogue = record('round/catalogue.json'), document = parseMethodology(read('round/catalogue.json'));
    const capture = record('round/capture.json'), session = record('round/session.json'), operator = record('operator-input.json');
    assert.equal(capture.value.sourceCommit, platform.plan.source.commit);
    assert.equal(capture.value.sourceTree, platform.plan.source.tree);
    assert.equal(capture.value.host.platform, job.host); assert.equal(capture.value.round, job.role);
    assert.equal(operator.value.round, job.role); assert.equal(session.value.round, job.role);
    assert.equal(operator.value.environment.GITHUB_RUN_ID, String(platform.plan.runId));
    assert.equal(operator.value.environment.GITHUB_RUN_ATTEMPT, '1');
    const oracles = record('round/artifact-oracles.json');
    const raw = (job.host === 'linux-x86_64' ? replayLinuxRound : replayWindowsRound)({ document,
      capture: capture.value, session: session.value, operator: operator.value, oracles: oracles.value, read });
    const build = replayNumericReleaseImage({ host: job.host,
      expectedSource: { commit: platform.plan.source.commit, tree: platform.plan.source.tree }, read });
    const oracle = replayOracleArchitecture({ document, host: job.host, operator: operator.value, capture: capture.value,
      read, readOriginal: (path, maximum) => readNumericCampaignOriginal(selection, sourceKey, path, maximum) });
    // The release replayer already validates both permitted producer/canonical build spellings.
    const release = record('release-build/capture.json', false), after = record('source-after-round.json');
    const censuses = reconcileNumericCampaignCensuses(selection, sourceKey,
      { before: release.value.source.before, after: release.value.source.after, afterRound: after.value }, read);
    const oracleInput = record('oracles/input.json');
    const cells = retainNumericArchiveSamples(job.host, session.value);
    const projection = { format: 'zryna.cli-performance-campaign-archive.v1',
      reviewedPlanSha256: platform.reviewedSha256, phase: platform.plan.phase,
      run: { id: platform.plan.runId, attempt: 1 }, source: platform.plan.source, host: job.host, round: job.role,
      jobId: job.jobId, checkRunId: job.checkRunId, startedAt: job.startedAt, completedAt: job.completedAt, artifact,
      providerArtifact: job.artifacts[1] ?? null,
      originals: { catalogue: catalogue.binding, capture: capture.binding, session: session.binding,
        operator: operator.binding, releaseBuild: release.binding, artifactOracles: oracles.binding,
        oracleInput: oracleInput.binding, sourceAfterRound: after.binding },
      reconciliations: { rawMetrics: raw.rawMetricsReconciled, releaseImage: build.rawBuildImageReconciled,
        pairedOraclesAndArchitecture: oracle.rawOracleArchitectureReconciled, sourceFileBytes: censuses.sourceFileBytesVerified },
      sourceCensuses: censuses, originalMaterialPins: originalMaterials(document),
      tools: { buildBefore: release.value.tools.before, buildAfter: release.value.tools.after,
        sdk: release.value.sdk, measured: capture.value.tools, oracle: oracleInput.value.tools },
      image: build.image, catalogueSha256: capture.value.catalogueSha256,
      memoryMethod: session.value.memoryMethod, memoryMeaning: session.value.memoryMeaning,
      wallEncoding: job.host === 'linux-x86_64' ? 'original-wall-ns-decimal' : 'original-qpc-ticks-and-frequency-decimal',
      cellCount: cells.length, attemptCount: cells.length * 6, warmupCount: cells.length, measuredCount: cells.length * 5, cells,
      sourceAuthenticated: false, rawAuthenticated: false, hostAuthenticated: false, proofQualified: false,
      performanceQualified: false, baselineAdmitted: false, hostAdmitted: false,
      baselineAuthority: null, thresholdAuthority: null, thresholds: null, closureStatus: 'blocked',
      pending: ['provider acquisition and actual source/tool/build execution provenance; host/class/method admission',
        'eligible numeric table, immutable temporal anchor and separate heldout joins'] };
    const result = Object.freeze({});
    owned.set(result, { selection, sourceKey, projection, archive: key, entries }); retained = true; return result;
  } finally { if (!retained) closePerformanceArchive(key); }
}
function retainedArchive(selection, sourceKey, key) {
  const value = owned.get(key);
  assert(value && value.selection === selection && value.sourceKey === sourceKey,
    'own archive key and SAME live phase/source keys required');
  inspectNumericCampaignPhase(selection); inspectNumericCampaignSource(selection, sourceKey);
  return value;
}
export function inspectNumericCampaignArchive(selection, sourceKey, key) {
  assert.equal(arguments.length, 3);
  return structuredClone(retainedArchive(selection, sourceKey, key).projection);
}
export function readNumericCampaignArchiveEntry(selection, sourceKey, key, path, maximum) {
  assert.equal(arguments.length, 5, 'fixed retained ZIP reader; no filesystem or callback');
  const value = retainedArchive(selection, sourceKey, key); portable(path);
  assert(Number.isSafeInteger(maximum) && maximum >= 0 && maximum <= 64 * 1024 * 1024);
  const entry = value.entries.get(path);
  assert(entry && entry.bytes <= maximum, 'bounded exact original archive member');
  return readPerformanceArchiveEntry(value.archive, path); // Existing reader returns a private copy.
}
export function closeNumericCampaignArchive(selection, sourceKey, key) {
  assert.equal(arguments.length, 3);
  const value = retainedArchive(selection, sourceKey, key);
  closePerformanceArchive(value.archive); owned.delete(key);
}
