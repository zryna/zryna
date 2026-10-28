// Existing synthetic platform API + original raw fixture readers; no real network/process calls.
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { linkReviewedPerformanceRun } from '../scripts/stability-gates/cli-performance-platform-link.mjs';
import { platformFixture, PREFIX } from './cli-performance-platform-fixture.mjs';
import { linuxReplayFixture } from './cli-performance-linux-replay-fixture.mjs';
import { windowsReplayFixture } from './cli-performance-windows-replay-fixture.mjs';
import { archiveFixture } from './cli-performance-archive-fixture.mjs';
import { summarizeWindowsFive } from '../scripts/stability-gates/cli-performance-windows-metrics.mjs';
export function exactWindows(raw) {
  for (const cell of raw.session.cells) {
    for (const [index, attempt] of cell.attempts.entries()) {
      const path = `round/${attempt.id}.metadata.raw`;
      const h = JSON.parse(raw.files.get(path));
      h.qpcFrequency = '18014398509481984'; h.wallTicks = String(9007199254740992n + BigInt(index));
      h.peakJobMemoryUsedBytes = String(9007199254741000n - BigInt(index));
      h.qpcEnd = String(BigInt(h.qpcStart) + BigInt(h.wallTicks));
      const bytes = Buffer.from(`${JSON.stringify(h)}\n`);
      raw.files.set(path, bytes); raw.files.set(`round/${attempt.id}.sample/result.json`, bytes);
      const readPath = `round/${attempt.id}.sample-read.json`;
      const read = JSON.parse(raw.files.get(readPath)); read.metadataBytes = bytes.length;
      raw.files.set(readPath, Buffer.from(canonical(read)));
      for (const key of ['qpcEnd', 'qpcFrequency', 'wallTicks', 'peakJobMemoryUsedBytes']) attempt[key] = h[key];
    }
    cell.summary = summarizeWindowsFive(cell.attempts.slice(1));
  }
  raw.files.set('round/session.json', Buffer.from(canonical(raw.session)));
}
export async function pair(host = 'linux-x86_64', { jobHost = host, round = 'baseline-round-1', mutate, source } = {}) {
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
