// Finite collection controller. Observations never issue source, baseline or threshold authority.
import assert from 'node:assert/strict';
import { isAbsolute } from 'node:path';
import { canonical, MAX_LOG, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { validateProviderBinding } from './cli-performance-linux-provider.mjs';
export const MEMORY_METHOD = 'gnu-time-wait4-maxrss-kib.v1';
const MARKER = 'ZRYNA_M418_MAXRSS_KIB_V1=';
export function maxRss(stderr) {
  assert(Buffer.isBuffer(stderr) && stderr.length <= 128, 'bounded metric stderr');
  const match = new RegExp(`^${MARKER}([1-9][0-9]{0,14})\\n$`, 'u').exec(stderr.toString('ascii'));
  assert(match && Buffer.from(match[0], 'ascii').equals(stderr), 'exact single GNU time metric');
  const value = Number(match[1]);
  assert(Number.isSafeInteger(value) && value > 0, 'positive safe KiB');
  return value;
}
export function summarizeFive(samples) {
  assert.equal(samples.length, 5, 'all five measured attempts required');
  const metric = key => {
    const values = samples.map(row => row[key]);
    assert(values.every(value => Number.isSafeInteger(value) && value > 0));
    values.sort((a, b) => a - b);
    return { median: values[2], maximum: values[4], spread: (values[4] - values[0]) / values[2] };
  };
  return { wallNs: metric('wallNs'), peakRssKiB: metric('peakRssKiB') };
}
export function validateLinuxCapture(document, capture) {
  canonical(capture);
  assert.equal(capture.format, 'zryna.cli-performance-linux-capture.draft.v2');
  assert.equal(capture.catalogueSha256, sha256(canonical(document)));
  assert(['io-qualification', 'baseline-round-1', 'baseline-round-2', 'held-out-candidate'].includes(capture.round));
  assert.equal(capture.host.platform, 'linux-x86_64');
  for (const field of ['sourceCommit', 'sourceTree']) assert.match(capture[field], /^[a-f0-9]{40}$/u);
  assert(Number.isSafeInteger(capture.hostWallGuardMs) && capture.hostWallGuardMs > 0 &&
    capture.hostWallGuardMs <= 900_000, 'finite reviewed guard, at most unchanged v1 containment');
  assert.equal(capture.cleanupGuardMs, 10_000);
  assert.equal(capture.combinedOutputBytes, MAX_LOG);
  for (const field of ['cli', 'time', 'node']) {
    const tool = capture.tools[field];
    assert(isAbsolute(tool.path) && tool.path.length < 4096);
    assert(Number.isSafeInteger(tool.bytes) && tool.bytes > 0 && tool.bytes <= 256 * 1024 * 1024);
    assert.match(tool.sha256, /^[a-f0-9]{64}$/u);
  }
  validateProviderBinding(capture);
  assert(['debug', 'release'].includes(capture.cliBuildClass));
  assert(capture.cliBuildClass === 'release' || capture.round === 'io-qualification',
    'debug-class CLI may qualify IO only, never baseline or held-out measurements');
  assert.match(capture.reviewRecordSha256, /^[a-f0-9]{64}$/u);
  assert.match(capture.artifactOracleSha256, /^[a-f0-9]{64}$/u);
  assert(Object.keys(capture.host).length >= 2, 'retain actual host metadata');
  // Required review/artifact digests are bindings, not a verifier for their authority.
}
function invocation(row, capture, root) {
  const args = row.argv.map(value => value === '$SOURCE_ROOT' ? root :
    value === '$NODE' ? capture.tools.node.path : value);
  return { executable: capture.tools.time.path,
    argv: ['--format', `${MARKER}%M`, '--', capture.tools.cli.path, ...args] };
}
// IO is explicit: real adapter uses the unchanged owned process helper; units use finite fake outputs.
// No caller-supplied workload commands, source text, expected values, samples or thresholds.
export async function collectLinuxRound(document, capture, io) {
  validateLinuxCapture(document, capture);
  assert(isAbsolute(io.root));
  const rows = matrixFor(document, 'linux-x86_64');
  const session = { format: 'zryna.cli-performance-linux-observations.draft.v2',
    captureSha256: sha256(canonical(capture)), catalogueSha256: capture.catalogueSha256,
    round: capture.round, cliBuildClass: capture.cliBuildClass,
    providerRecordSha256: capture.providerRecordSha256, providerRoot: capture.providerRoot,
    providerReceiptSha256: capture.providerReceiptSha256,
    memoryMethod: MEMORY_METHOD,
    memoryMeaning: 'GNU time wait4 maxRSS high-water, not simultaneous summed tree RSS',
    status: 'invalid-or-incomplete', closureStatus: 'blocked', thresholds: null,
    sourceAuthenticated: false, baselineAuthority: null, performanceQualified: false, cells: [] };
  // Create-only capture must precede any attempted execution, including preflight failures.
  await io.retain('capture.json', Buffer.from(canonical(capture)));
  await io.retain('catalogue.json', Buffer.from(canonical(document)));
  try {
    await io.checkMaterials(document, capture);
    for (const row of rows) {
      const attempts = [];
      for (let index = 0; index < 6; index += 1) {
        await io.checkMaterials(document, capture);
        await io.beforeAttempt(row); // refuse existing owned output; no deletion of foreign bundles
        const call = invocation(row, capture, io.root);
        const id = `${row.id.replaceAll('/', '_')}_${index}`;
        const started = io.nowNs();
        let result;
        try { result = await io.execute(call.executable, call.argv, capture.hostWallGuardMs); }
        catch (error) { result = { exitCode: null, signal: null, error: String(error.message).slice(0, 4096),
          stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) }; }
        const ended = io.nowNs();
        assert(typeof started === 'bigint' && typeof ended === 'bigint' && ended > started);
        const raw = { id, selector: row.id, index, role: index === 0 ? 'warmup' : 'measured',
          call, wallNs: String(ended - started), exitCode: result.exitCode, signal: result.signal,
          error: result.error, stdoutBytes: result.stdout.length, stderrBytes: result.stderr.length };
        // Real adapter bounds chunks before buffering. Retain raw bytes and status BEFORE assertions.
        await io.retain(`${id}.stdout`, result.stdout);
        await io.retain(`${id}.stderr`, result.stderr);
        await io.retain(`${id}.json`, Buffer.from(canonical(raw)));
        assert(Buffer.isBuffer(result.stdout) && Buffer.isBuffer(result.stderr));
        assert(result.stdout.length + result.stderr.length <= MAX_LOG, 'combined log budget');
        assert.equal(result.error, null, 'no timeout/spawn/cleanup/output failure');
        assert.equal(result.signal, null);
        assert.equal(result.exitCode, 0);
        const wallNs = Number(ended - started);
        assert(Number.isSafeInteger(wallNs) && wallNs > 0);
        const peakRssKiB = maxRss(result.stderr);
        const response = JSON.parse(result.stdout.toString('utf8'));
        canonical(response); // bounded JSON shape; exact independently frozen raw oracle checked below
        assert.deepEqual(response, row.expectedResponse, 'original typed source outcome');
        await io.checkArtifactsAndStdout(row, result.stdout);
        await io.checkMaterials(document, capture);
        await io.afterAttempt(row); // remove only this verified owned bundle, outside timer
        attempts.push({ id, role: raw.role, wallNs, peakRssKiB });
      }
      session.cells.push({ selector: row.id, attempts, summary: summarizeFive(attempts.slice(1)) });
    }
    assert.equal(session.cells.length, 24);
    await io.finishRound(); // fixture-stage cleanup is outside individual timers and must succeed
    session.status = 'collected-unqualified-observations';
  } catch (error) {
    session.failure = String(error.message).slice(0, 4096);
    await io.retain('session.json', Buffer.from(canonical(session)));
    throw error;
  }
  await io.retain('session.json', Buffer.from(canonical(session)));
  return session;
}
