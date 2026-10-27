// Join a live platform selection to raw consistency; source/host/policy admission stays separate.
import assert from 'node:assert/strict';
import { MAX_DOCUMENT, MAX_LOG, parse, portable } from './input.mjs';
import { inspectPerformancePlatformLink } from './cli-performance-platform-link.mjs';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry, closePerformanceArchive }
  from './cli-performance-archive.mjs';
import { parseMethodology } from './cli-performance-methodology-v4.mjs';
import { replayLinuxRound } from './cli-performance-linux-replay.mjs';
import { replayWindowsRound } from './cli-performance-windows-replay.mjs';

export function replayHostedPerformanceArchive(selection, artifactId, bytes) {
  const platform = inspectPerformancePlatformLink(selection);
  assert(Number.isSafeInteger(artifactId) && artifactId > 0);
  const matches = platform.expected.jobs.filter(job => job.primary.id === artifactId);
  assert.equal(matches.length, 1, 'S418 raw: selected primary artifact required');
  const job = matches[0], key = openPerformanceArchive(bytes, job.primary, 'primary');
  try {
    const prefix = 'm418-hosted-observations/capture/';
    const entries = new Map(listPerformanceArchive(key).map(row => [row.path, row]));
    const read = relative => {
      portable(relative);
      const name = prefix + relative, entry = entries.get(name);
      const maximum = relative.endsWith('.metadata.raw') ? 4096 : relative.endsWith('.json') ? MAX_DOCUMENT : MAX_LOG;
      assert(entry && entry.bytes <= maximum, `S418 raw: bounded retained entry ${relative}`);
      return readPerformanceArchiveEntry(key, name);
    };
    // Whole-document identity is independently frozen by parseMethodology, not learned from this archive.
    const document = parseMethodology(read('round/catalogue.json'));
    const capture = parse(read('round/capture.json')), session = parse(read('round/session.json'));
    const operator = parse(read('operator-input.json')), oracles = parse(read('round/artifact-oracles.json'));
    assert.equal(capture.sourceCommit, platform.expected.source.commit, 'S418 raw: selected source commit');
    assert.equal(capture.sourceTree, platform.expected.source.tree, 'S418 raw: selected source tree');
    assert.equal(capture.round, job.round, 'S418 raw: selected round');
    assert.equal(capture.host.platform, job.host, 'S418 raw: selected host');
    const replay = job.host === 'linux-x86_64' ? replayLinuxRound : replayWindowsRound;
    const result = replay({ document, capture, session, operator, oracles, read });
    return { format: 'zryna.cli-performance-raw-archive.v1', reviewedExpectationsSha256: platform.reviewedSha256,
      run: platform.run, source: platform.expected.source, host: job.host, round: job.round,
      jobId: job.jobId, checkRunId: job.checkRunId, artifact: job.primary, ...result,
      rawAuthenticated: false, hostAuthenticated: false, sourceAuthenticated: false,
      performanceQualified: false, baselineAuthority: null, thresholds: null };
  } finally { closePerformanceArchive(key); }
}
