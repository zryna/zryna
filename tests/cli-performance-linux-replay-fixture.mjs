import { readFileSync } from 'node:fs';
import { canonical, MAX_LOG, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { PROVIDER_SHA256, timeExecutable } from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
const rows = matrixFor(document, 'linux-x86_64');
const WALL = [14000000000, 100, 900, 300, 500, 700], RSS = [99999, 11, 19, 13, 17, 15];
const expected = { wallNs: { median: 500, maximum: 900, spread: 1.6 },
  peakRssKiB: { median: 15, maximum: 19, spread: 8 / 15 } };
export function linuxReplayFixture() {
  const files = new Map(), pin = path => ({ path, bytes: 1, sha256: '1'.repeat(64) });
  const operator = { root: '/source', round: 'baseline-round-1', expectedHead: 'a'.repeat(40),
    expectedTree: 'b'.repeat(40), tools: { node: '/tools/node', rustc: '/tools/rustc' } };
  const oracles = rows.map(row => ({ selector: row.id, manifest: row.expectedResponse.manifest,
    stdout: `${JSON.stringify(row.expectedResponse)}\n`, stderr: '',
    files: [{ path: row.expectedResponse.manifest, bytes: 1, sha256: 'f'.repeat(64) }] }));
  const capture = { format: 'zryna.cli-performance-linux-capture.draft.v2', catalogueSha256: sha256(canonical(document)),
    round: operator.round, cliBuildClass: 'release', sourceCommit: operator.expectedHead, sourceTree: operator.expectedTree,
    host: { platform: 'linux-x86_64', observation: 'synthetic' }, tools: { cli: pin('/tools/zryna'),
      node: pin(operator.tools.node), time: timeExecutable('/tmp/m418-linux-provider-ABC123') },
    environment: { RUSTC: operator.tools.rustc }, hostWallGuardMs: 15000, cleanupGuardMs: 10000,
    combinedOutputBytes: MAX_LOG, reviewRecordSha256: '2'.repeat(64), artifactOracleSha256: sha256(canonical(oracles)),
    providerRoot: '/tmp/m418-linux-provider-ABC123', providerRecordSha256: PROVIDER_SHA256,
    providerReceiptSha256: '3'.repeat(64), sourceAuthenticated: false, performanceQualified: false,
    baselineAuthority: null, thresholds: null, productionReleaseAdmission: false };
  const session = { format: 'zryna.cli-performance-linux-observations.draft.v2', captureSha256: sha256(canonical(capture)),
    catalogueSha256: capture.catalogueSha256, round: capture.round, cliBuildClass: capture.cliBuildClass,
    providerRecordSha256: capture.providerRecordSha256, providerRoot: capture.providerRoot,
    providerReceiptSha256: capture.providerReceiptSha256, memoryMethod: 'gnu-time-wait4-maxrss-kib.v1',
    memoryMeaning: 'GNU time wait4 maxRSS high-water, not simultaneous summed tree RSS',
    status: 'collected-unqualified-observations', closureStatus: 'blocked', sourceAuthenticated: false,
    performanceQualified: false, baselineAuthority: null, thresholds: null, cells: [] };
  for (const [cell, row] of rows.entries()) {
    const attempts = WALL.map((wallNs, index) => {
      const name = `${row.id.replaceAll('/', '_')}_${index}`, role = index ? 'measured' : 'warmup';
      const stdout = Buffer.from(oracles[cell].stdout), stderr = Buffer.from(`ZRYNA_M418_MAXRSS_KIB_V1=${RSS[index]}\n`);
      files.set(`round/${name}.stdout`, stdout); files.set(`round/${name}.stderr`, stderr);
      files.set(`round/${name}.json`, Buffer.from(canonical({ id: name, selector: row.id, index, role,
        call: { executable: capture.tools.time.path, argv: ['--format', 'ZRYNA_M418_MAXRSS_KIB_V1=%M', '--', capture.tools.cli.path,
          ...row.argv.map(v => v === '$SOURCE_ROOT' ? operator.root : v === '$NODE' ? capture.tools.node.path : v)] },
        wallNs: String(wallNs), exitCode: 0, signal: null, error: null, stdoutBytes: stdout.length, stderrBytes: stderr.length })));
      return { id: name, role, wallNs, peakRssKiB: RSS[index] };
    });
    session.cells.push({ selector: row.id, attempts, summary: structuredClone(expected) });
  }
  return { document: structuredClone(document), operator, capture, session, oracles, files, read: path => files.get(path) };
}
export { document, rows, expected };
