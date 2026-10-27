import assert from 'node:assert/strict';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { collectWindowsRound } from '../scripts/stability-gates/cli-performance-windows-round.mjs';
import { document, fixture, rows } from './cli-performance-windows-fixture.mjs';
export async function windowsReplayFixture() {
  const f = fixture(({ metadata: h, index }) => { h.peakJobMemoryUsedBytes = String([999, 13, 17, 11, 19, 15][index % 6]); });
  const oracles = rows.map(row => ({ selector: row.id, manifest: row.expectedResponse.manifest,
    stdout: JSON.stringify(row.expectedResponse), stderr: '',
    files: [{ path: row.expectedResponse.manifest, bytes: 1, sha256: '0'.repeat(64) }] }));
  Object.assign(f.capture, { artifactOracleSha256: sha256(canonical(oracles)), sourceAuthenticated: false,
    performanceQualified: false, baselineAuthority: null, thresholds: null });
  const operator = { root: f.io.root, round: f.capture.round, expectedHead: f.capture.sourceCommit,
    expectedTree: f.capture.sourceTree };
  f.capture.operatorAdmission.path = 'C:\\synthetic\\windows-operator-binding.json';
  const admission = { format: 'zryna.cli-performance-windows-operator-binding.v1', tools: f.capture.tools,
    sourceCommit: f.capture.sourceCommit, sourceTree: f.capture.sourceTree,
    helperBuiltFromSha256: 'ea0a832ad83b3d6473c99a359da96fb5f7a1587d1c76254f12e4a9dbeb188e3a',
    catalogueSha256: f.capture.catalogueSha256, artifactOracleSha256: f.capture.artifactOracleSha256,
    reviewRecordSha256: f.capture.reviewRecordSha256 };
  const bytes = Buffer.from(canonical(admission));
  Object.assign(f.capture.operatorAdmission, { bytes: bytes.length, sha256: sha256(bytes) });
  f.io.output = 'C:\\synthetic\\round';
  const session = await collectWindowsRound(document, f.capture, f.io);
  const files = new Map([...f.saved].map(([path, value]) => [`round/${path}`, value]));
  const put = (path, value) => files.set(path, Buffer.isBuffer(value) ? value : Buffer.from(canonical(value)));
  put('operator-input.json', operator); put('round/operator-admission.json', bytes);
  put('round/artifact-oracles.json', oracles);
  for (const cell of session.cells) for (const attempt of cell.attempts)
    for (const [original, copy] of [['metadata.raw', 'result.json'], ['stdout', 'stdout.raw'], ['stderr', 'stderr.raw']])
      put(`round/${attempt.id}.sample/${copy}`, files.get(`round/${attempt.id}.${original}`));
  return { document, capture: f.capture, session, operator, oracles, files, put,
    read: path => { assert(files.has(path), `missing ${path}`); return files.get(path); } };
}
