// Offline raw consistency only; no source, host, artifact, or baseline authentication.
import assert from 'node:assert/strict';
import { posix } from 'node:path';
import { TextDecoder } from 'node:util';
import { canonical, exact, MAX_DOCUMENT, MAX_LOG, parse, sha256 } from './input.mjs';
import { matrixFor } from './cli-performance-methodology-v4.mjs';
import { validateLinuxCapture } from './cli-performance-linux-round-v2.mjs';
const METHOD = 'gnu-time-wait4-maxrss-kib.v1';
const MEANING = 'GNU time wait4 maxRSS high-water, not simultaneous summed tree RSS';
const ROLE = index => index === 0 ? 'warmup' : 'measured';
function authority(record, required) {
  for (const [key, value] of Object.entries({ sourceAuthenticated: false, performanceQualified: false,
    baselineAuthority: null, thresholds: null, productionReleaseAdmission: false })) {
    if (required.includes(key) || Object.hasOwn(record, key)) assert.equal(record[key], value, key);
  }
}
function summary(samples) {
  return Object.fromEntries(['wallNs', 'peakRssKiB'].map(key => {
    const values = samples.map(sample => sample[key]).sort((a, b) => a - b);
    return [key, { median: values[2], maximum: values[4], spread: (values[4] - values[0]) / values[2] }];
  }));
}
export function replayLinuxRound({ document, capture, session, operator, oracles, read }) {
  validateLinuxCapture(document, capture);
  canonical(session); canonical(operator);
  const flags = ['sourceAuthenticated', 'performanceQualified', 'baselineAuthority', 'thresholds'];
  authority(capture, [...flags, 'productionReleaseAdmission']); authority(session, flags); authority(operator, []);
  assert.equal(capture.hostWallGuardMs, 15000, 'reviewed 15-second raw round');
  exact(capture.tools, ['cli', 'node', 'time'], 'Linux tools');
  for (const path of [operator.root, ...Object.values(capture.tools).map(tool => tool.path)])
    assert(typeof path === 'string' && posix.isAbsolute(path) && posix.resolve(path) === path);
  assert.equal(operator.round, capture.round); assert.equal(operator.expectedHead, capture.sourceCommit);
  assert.equal(operator.expectedTree, capture.sourceTree); assert.equal(operator.tools.node, capture.tools.node.path);
  assert.equal(capture.environment.RUSTC, operator.tools.rustc);
  exact(session, ['format', 'captureSha256', 'catalogueSha256', 'round', 'cliBuildClass', 'providerRecordSha256',
    'providerRoot', 'providerReceiptSha256', 'memoryMethod', 'memoryMeaning', 'status', 'closureStatus',
    ...flags, 'cells'], 'Linux session');
  assert.equal(session.format, 'zryna.cli-performance-linux-observations.draft.v2');
  assert.equal(session.status, 'collected-unqualified-observations'); assert.equal(session.closureStatus, 'blocked');
  assert.equal(session.memoryMethod, METHOD); assert.equal(session.memoryMeaning, MEANING);
  assert.equal(session.captureSha256, sha256(canonical(capture)));
  for (const key of ['catalogueSha256', 'round', 'cliBuildClass', 'providerRecordSha256', 'providerRoot', 'providerReceiptSha256'])
    assert.equal(session[key], capture[key], key);
  assert.equal(sha256(canonical(oracles)), capture.artifactOracleSha256, 'paired oracle binding');
  const rows = matrixFor(document, 'linux-x86_64');
  assert.deepEqual(oracles.map(row => row.selector), rows.map(row => row.id));
  assert.deepEqual(session.cells.map(cell => cell.selector), rows.map(row => row.id));
  const load = (path, limit) => { const bytes = read(path); assert(Buffer.isBuffer(bytes) && bytes.length <= limit); return bytes; };
  const summaries = rows.map((row, cellIndex) => {
    const cell = session.cells[cellIndex], oracle = oracles[cellIndex];
    exact(cell, ['selector', 'attempts', 'summary'], 'Linux cell'); assert.equal(cell.attempts.length, 6);
    exact(oracle, ['selector', 'manifest', 'stdout', 'stderr', 'files'], 'paired oracle');
    assert.equal(oracle.manifest, row.expectedResponse.manifest); assert.equal(oracle.stderr, '');
    assert(typeof oracle.stdout === 'string' && Buffer.byteLength(oracle.stdout) <= MAX_DOCUMENT);
    assert(Array.isArray(oracle.files) && oracle.files.length > 0 && oracle.files.length <= 64);
    assert.deepEqual(JSON.parse(oracle.stdout), row.expectedResponse);
    const samples = cell.attempts.map((declared, index) => {
      const id = `${row.id.replaceAll('/', '_')}_${index}`, role = ROLE(index);
      const raw = parse(load(`round/${id}.json`, MAX_DOCUMENT));
      exact(raw, ['id', 'selector', 'index', 'role', 'call', 'wallNs', 'exitCode', 'signal', 'error', 'stdoutBytes', 'stderrBytes'], 'raw attempt');
      assert.equal(raw.id, id); assert.equal(raw.selector, row.id); assert.equal(raw.index, index); assert.equal(raw.role, role);
      assert.deepEqual(raw.call, { executable: capture.tools.time.path,
        argv: ['--format', 'ZRYNA_M418_MAXRSS_KIB_V1=%M', '--', capture.tools.cli.path,
          ...row.argv.map(value => value === '$SOURCE_ROOT' ? operator.root : value === '$NODE' ? capture.tools.node.path : value)] });
      assert.equal(raw.exitCode, 0); assert.equal(raw.signal, null); assert.equal(raw.error, null);
      assert(typeof raw.wallNs === 'string' && /^[1-9][0-9]{0,10}$/u.test(raw.wallNs));
      const wallNs = Number(raw.wallNs); assert(wallNs <= 15000000000, 'positive wall time within 15 seconds');
      const stdout = load(`round/${id}.stdout`, MAX_LOG), stderr = load(`round/${id}.stderr`, 128);
      assert(stdout.length + stderr.length <= MAX_LOG);
      assert.equal(raw.stdoutBytes, stdout.length); assert.equal(raw.stderrBytes, stderr.length);
      const response = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(stdout)); canonical(response);
      assert.deepEqual(response, row.expectedResponse, 'original typed outcome');
      assert(stdout.equals(Buffer.from(oracle.stdout)), 'exact paired public bytes');
      const match = /^ZRYNA_M418_MAXRSS_KIB_V1=([1-9][0-9]{0,14})\n$/u.exec(stderr.toString('utf8'));
      assert(match && Buffer.from(match[0]).equals(stderr), 'exact GNU wait4 maximum-RSS KiB marker');
      const peakRssKiB = Number(match[1]); assert(Number.isSafeInteger(peakRssKiB) && peakRssKiB > 0);
      const sample = { id, role, wallNs, peakRssKiB };
      assert.deepEqual(declared, sample, 'raw/session sample agreement'); return sample;
    });
    const computed = summary(samples.slice(1));
    assert.deepEqual(cell.summary, computed, 'all five measured samples; warmup excluded');
    return { selector: row.id, summary: computed };
  });
  return { rawMetricsReconciled: true, cells: 24, attempts: 144, measuredAttempts: 120, warmups: 24,
    memoryMethod: METHOD, summaries, rawAuthenticated: false, sourceAuthenticated: false, hostAuthenticated: false,
    performanceQualified: false, baselineAuthority: null, thresholds: null };
}
