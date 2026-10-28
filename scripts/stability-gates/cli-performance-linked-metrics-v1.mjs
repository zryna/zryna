// Existing platform/raw replay joins only; never host/source/baseline/performance admission.
import assert from 'node:assert/strict';
import { MAX_DOCUMENT, parse, sha256 } from './input.mjs';
import { inspectPerformancePlatformLink } from './cli-performance-platform-link.mjs';
import { replayHostedPerformanceArchive } from './cli-performance-raw-archive.mjs';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry, closePerformanceArchive }
  from './cli-performance-archive.mjs';
const reconciled = new WeakMap();
const PREFIX = 'm418-hosted-observations/capture/';
function document(key, relative) {
  const path = PREFIX + relative;
  const entry = listPerformanceArchive(key).find(row => row.path === path);
  assert(entry && entry.bytes <= MAX_DOCUMENT, `S418 metrics: bounded original ${relative}`);
  const bytes = readPerformanceArchiveEntry(key, path);
  return { value: parse(bytes), binding: { path, bytes: bytes.length, sha256: sha256(bytes) } };
}
function linuxAttempt(attempt, index) {
  // Existing raw replay proves the decimal marker and safe integer before this conversion.
  assert(Number.isSafeInteger(attempt.wallNs) && attempt.wallNs > 0);
  assert(Number.isSafeInteger(attempt.peakRssKiB) && attempt.peakRssKiB > 0);
  return { id: attempt.id, index, role: attempt.role,
    wallNs: String(attempt.wallNs), peakRssKiB: String(attempt.peakRssKiB) };
}
function windowsAttempt(attempt, index) {
  const keys = ['qpcStart', 'qpcEnd', 'wallTicks', 'qpcFrequency', 'peakJobMemoryUsedBytes'];
  assert(keys.every(key => typeof attempt[key] === 'string'), 'S418 metrics: original native decimal strings');
  return { id: attempt.id, index, role: attempt.role, ...Object.fromEntries(keys.map(key => [key, attempt[key]])) };
}
export function reconcileHostedRawMetrics(selection, artifactId, bytes) {
  assert.equal(arguments.length, 3, 'S418 metrics: no caller metric/read callbacks');
  const platform = inspectPerformancePlatformLink(selection);
  assert(Number.isSafeInteger(artifactId) && artifactId > 0);
  const matches = platform.expected.jobs.filter(job => job.primary.id === artifactId);
  assert.equal(matches.length, 1, 'S418 metrics: exact selected primary artifact');
  const job = matches[0];
  // The LIVE existing issuer has already bounded this primary to128MiB. No new limit/refill.
  assert(Buffer.isBuffer(bytes) && bytes.length === job.primary.bytes, 'S418 metrics: original selected byte count');
  const image = Buffer.alloc(job.primary.bytes); // Private, non-pooled; no exposed caller backing store.
  image.set(bytes);
  const replay = replayHostedPerformanceArchive(selection, artifactId, image);
  // The replayer closes its own ZIP key. Reopen the SAME private image to read two original documents.
  const key = openPerformanceArchive(image, job.primary, 'primary');
  try {
    const capture = document(key, 'round/capture.json'), session = document(key, 'round/session.json');
    const attempt = replay.host === 'linux-x86_64' ? linuxAttempt : windowsAttempt;
    const cells = session.value.cells.map(cell => ({ selector: cell.selector,
      attempts: cell.attempts.map((row, index) => attempt(row, index)) }));
    const projection = { format: 'zryna.cli-performance-linked-raw-metrics.v1',
      consistency: 'existing-hosted-raw-replay-completed', reviewedExpectationsSha256: replay.reviewedExpectationsSha256,
      run: replay.run, source: replay.source, host: replay.host, round: replay.round,
      jobId: replay.jobId, checkRunId: replay.checkRunId, artifact: replay.artifact,
      capture: capture.binding, session: session.binding, catalogueSha256: capture.value.catalogueSha256,
      memoryMethod: session.value.memoryMethod, memoryMeaning: session.value.memoryMeaning,
      wallEncoding: replay.host === 'linux-x86_64' ? 'original-wall-ns-decimal' : 'original-qpc-ticks-and-frequency-decimal',
      cellCount: cells.length, attemptCount: cells.reduce((n, cell) => n + cell.attempts.length, 0),
      warmupCount: cells.length, measuredCount: cells.length * 5, cells,
      rawAuthenticated: false, sourceAuthenticated: false, hostAuthenticated: false,
      proofQualified: false, performanceQualified: false, baselineAdmitted: false,
      closureStatus: 'blocked', baselineAuthority: null, thresholdAuthority: null, thresholds: null,
      pending: ['independent source/build/tool and host-fact admission', 'authentic threshold policy/table/time anchor',
        'separate held-out topology and observations'] };
    const result = Object.freeze({});
    reconciled.set(result, { selection, projection });
    return result;
  } finally { closePerformanceArchive(key); }
}
export function inspectHostedRawMetrics(selection, key) {
  assert.equal(arguments.length, 2);
  const value = reconciled.get(key);
  assert(value && value.selection === selection, 'S418 metrics: own key and SAME live platform selection required');
  return structuredClone(value.projection); // Never expose retained objects, buffers or authority capabilities.
}
