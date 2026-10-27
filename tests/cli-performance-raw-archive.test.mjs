import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, MAX_DOCUMENT, sha256 } from '../scripts/stability-gates/input.mjs';
import { linkReviewedPerformanceRun, inspectPerformancePlatformLink } from '../scripts/stability-gates/cli-performance-platform-link.mjs';
import { replayHostedPerformanceArchive } from '../scripts/stability-gates/cli-performance-raw-archive.mjs';
import { platformFixture, PREFIX } from './cli-performance-platform-fixture.mjs';
import { linuxReplayFixture } from './cli-performance-linux-replay-fixture.mjs';
import { windowsReplayFixture } from './cli-performance-windows-replay-fixture.mjs';
import { archiveFixture } from './cli-performance-archive-fixture.mjs';

async function pair(host = 'linux-x86_64', { jobHost = host, round = 'baseline-round-1', mutate, source } = {}) {
  const raw = host === 'linux-x86_64' ? linuxReplayFixture() : await windowsReplayFixture();
  if (mutate) mutate(raw);
  const files = new Map(raw.files);
  for (const [path, value] of [['round/catalogue.json', raw.document], ['round/capture.json', raw.capture],
    ['round/session.json', raw.session], ['operator-input.json', raw.operator], ['round/artifact-oracles.json', raw.oracles]])
    if (!files.has(path)) files.set(path, Buffer.from(canonical(value)));
  const bytes = archiveFixture([...files].map(([path, content]) => ['m418-hosted-observations/capture/' + path, content]));
  const f = platformFixture(), commit = source?.commit ?? raw.capture.sourceCommit, tree = source?.tree ?? raw.capture.sourceTree;
  const before = f.value.source.commit;
  f.value.source.commit = f.value.execution.commit = f.run.head_sha = commit;
  f.value.source.tree = f.value.execution.tree = tree;
  const reference = f.run.referenced_workflows[0]; reference.sha = commit;
  reference.path = reference.path.replace(before, commit);
  for (const [endpoint, value] of [...f.responses]) {
    if (endpoint.endsWith(before)) {
      f.responses.delete(endpoint);
      if (endpoint.includes('/git/commits/')) { value.sha = commit; value.tree.sha = tree; }
      f.responses.set(endpoint.slice(0, -before.length) + commit, value);
    }
  }
  for (const row of f.jobs) row.head_sha = commit;
  for (const row of f.artifacts) row.workflow_run.head_sha = commit;
  for (const job of f.value.jobs) f.responses.get(`${PREFIX}/check-runs/${job.checkRunId}`).head_sha = commit;
  const job = f.value.jobs.find(row => row.host === jobHost && row.round === round);
  const artifact = f.artifacts.find(row => row.id === job.primary.id);
  job.primary.bytes = artifact.size_in_bytes = bytes.length;
  job.primary.sha256 = sha256(bytes); artifact.digest = `sha256:${job.primary.sha256}`;
  const selection = await linkReviewedPerformanceRun(...f.selection());
  return { selection, artifactId: job.primary.id, bytes, raw, platform: f };
}
test('live selection plus complete Linux and Windows archives replay all fixed raw cells without granting qualification', async () => {
  for (const [host, cells, attempts] of [['linux-x86_64', 24, 144], ['windows-x86_64', 20, 120]]) {
    const f = await pair(host), report = replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes);
    assert.equal(report.rawMetricsReconciled, true); assert.equal(report.cells, cells); assert.equal(report.attempts, attempts);
    assert.equal(report.artifact.id, f.artifactId); assert.equal(report.source.commit, f.raw.capture.sourceCommit);
    assert.deepEqual([report.sourceAuthenticated, report.rawAuthenticated, report.hostAuthenticated,
      report.performanceQualified, report.baselineAuthority, report.thresholds], [false, false, false, false, null, null]);
  }
});
test('serialized or edited platform reports and unknown/provider IDs cannot select raw primary evidence', async () => {
  const f = await pair();
  for (const fake of [{}, { ...f.selection }, inspectPerformancePlatformLink(f.selection)])
    assert.throws(() => replayHostedPerformanceArchive(fake, f.artifactId, f.bytes));
  for (const id of [0, '301', 999999, f.platform.value.jobs[0].provider.id])
    assert.throws(() => replayHostedPerformanceArchive(f.selection, id, f.bytes));
});
test('a different archive cannot replace the platform-selected digest or byte count', async () => {
  const f = await pair(); replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes);
  const corrupt = Buffer.from(f.bytes); corrupt[30] ^= 1;
  assert.throws(() => replayHostedPerformanceArchive(f.selection, f.artifactId, corrupt));
  assert.throws(() => replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes.subarray(0, -1)));
});
test('even valid archive bytes cannot relabel source, tree, host or paired round', async () => {
  for (const options of [{ jobHost: 'windows-x86_64' }, { round: 'baseline-round-2' },
    { source: { commit: 'f'.repeat(40), tree: 'e'.repeat(40) } },
    { source: { commit: 'a'.repeat(40), tree: 'e'.repeat(40) } }]) {
    const f = await pair('linux-x86_64', options);
    assert.throws(() => replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes), /selected (source|round|host)/u);
  }
});
test('platform-pinned archives still require the independently frozen complete catalogue', async () => {
  const f = await pair('linux-x86_64', { mutate(raw) {
    const value = structuredClone(raw.document); value.families[0].export = 'other';
    raw.files.set('round/catalogue.json', Buffer.from(canonical(value)));
  } });
  assert.throws(() => replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes), /methodology identity/u);
});
test('oversized retained JSON rejects before allocating its entry copy or parsing it', async () => {
  const f = await pair('linux-x86_64', { mutate(raw) {
    raw.files.set('round/capture.json', Buffer.alloc(MAX_DOCUMENT + 1, 0x20));
  } });
  assert.throws(() => replayHostedPerformanceArchive(f.selection, f.artifactId, f.bytes), /bounded retained entry round\/capture.json/u);
});
