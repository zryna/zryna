import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, MAX_LOG, sha256 } from '../scripts/stability-gates/input.mjs';
import { checkedWindowsMetrics, summarizeWindowsFive } from '../scripts/stability-gates/cli-performance-windows-metrics.mjs';
import { HELPER_SOURCE_SHA256, localAbsolute, validateWindowsAdmission } from '../scripts/stability-gates/cli-performance-windows-binding.mjs';
import { validateWindowsOracles } from '../scripts/stability-gates/cli-performance-windows-files.mjs';
import { document, fixture, metadata, rows } from './cli-performance-windows-fixture.mjs';
const stdout = Buffer.from('{}'), stderr = Buffer.alloc(0);
test('Windows exact QPC strings survive values above Number precision and Job COMMIT retains native units', () => {
  const { capture } = fixture(), h = metadata(stdout, stderr, 9);
  const result = checkedWindowsMetrics(h, capture, 9, stdout, stderr);
  assert.equal(result.qpcStart, '9007199254740993000'); assert.equal(result.qpcEnd, '9007199254740993003');
  assert.equal(result.peakJobMemoryUsedBytes, '3'); assert.equal(result.wallTicks, '3');
  assert.equal(Object.hasOwn(result, 'wallNs'), false); assert.equal(Object.hasOwn(result, 'peakRssKiB'), false);
  h.peakJobMemoryUsedBytes = '18446744073709551615';
  assert.equal(checkedWindowsMetrics(h, capture, 9, stdout, stderr).peakJobMemoryUsedBytes, h.peakJobMemoryUsedBytes);
});
test('Windows incomplete main/EOF/Job/query/cleanup, forced cleanup and nonzero exits always invalidate', () => {
  const { capture } = fixture();
  const bad = [h => { h.measurementStatus = 'invalid'; }, h => { h.mainExitCode = '17'; },
    ...['childCreated', 'mainExitKnown', 'stdoutEof', 'stderrEof', 'jobEmptyConfirmed', 'cleanupConfirmed', 'peakQuerySucceeded']
      .map(key => h => { h[key] = false; }),
    h => { h.forcedTermination = true; }, h => { h.primaryStage = 1; }, h => { h.cleanupStage = 1; },
    h => { h.primaryWin32Error = 258; }, h => { h.cleanupWin32Error = 5; }];
  for (const change of bad) {
    const h = metadata(stdout, stderr, 9); change(h);
    assert.throws(() => checkedWindowsMetrics(h, capture, 9, stdout, stderr));
  }
});
test('Windows malformed unsigned64s precision loss and inconsistent QPC or raw lengths reject', () => {
  const { capture } = fixture();
  const bad = [h => { h.extra = true; }, h => { delete h.stdoutEof; }, h => { h.childArgc += 1; },
    h => { h.childPid = 0; }, h => { h.commandUtf16Units = 32767; }, h => { h.stdoutBytes += 1; },
    h => { h.timeoutMs += 1; }, h => { h.cleanupReserveMs += 1; }, h => { h.combinedStreamLimitBytes += 1; },
    h => { h.qpcEnd = h.qpcStart; }, h => { h.wallTicks = '4'; }, h => { h.qpcFrequency = '0'; },
    h => { h.peakJobMemoryUsedBytes = '0'; }, h => { h.qpcFrequency = 1000000000; },
    ...['-1', '01', '1.5', '18446744073709551616', null, true].map(value => h => { h.qpcStart = value; }),
    h => { h.qpcFrequency = '1'; }];
  for (const change of bad) {
    const h = metadata(stdout, stderr, 9); change(h);
    assert.throws(() => checkedWindowsMetrics(h, capture, 9, stdout, stderr));
  }
  const tooMuch = Buffer.alloc(MAX_LOG + 1), h = metadata(tooMuch, stderr, 9);
  assert.throws(() => checkedWindowsMetrics(h, capture, 9, tooMuch, stderr), /stream budget/u);
});
test('Windows all five observations determine exact median maximum and spread without outlier removal', () => {
  const samples = ['10000000000000000', '1', '2', '3', '4'].map(value => ({ wallTicks: value,
    peakJobMemoryUsedBytes: value, qpcFrequency: '1000000000' }));
  const summary = summarizeWindowsFive(samples);
  assert.equal(summary.wallTicks.median, '3'); assert.equal(summary.wallTicks.maximum, '10000000000000000');
  assert.deepEqual(summary.wallTicks.spread, { numerator: '9999999999999999', denominator: '3' });
  assert.equal(samples[0].wallTicks, '10000000000000000');
  for (const invalid of [samples.slice(1), [...samples, samples[0]],
    samples.map((row, index) => index ? row : { ...row, qpcFrequency: '2' }),
    samples.map((row, index) => index ? row : { ...row, wallTicks: '0' })]) assert.throws(() => summarizeWindowsFive(invalid));
});
function admission(capture) {
  return { format: 'zryna.cli-performance-windows-operator-binding.v1', tools: structuredClone(capture.tools),
    helperBuiltFromSha256: HELPER_SOURCE_SHA256, sourceCommit: capture.sourceCommit, sourceTree: capture.sourceTree,
    catalogueSha256: capture.catalogueSha256, artifactOracleSha256: capture.artifactOracleSha256,
    reviewRecordSha256: capture.reviewRecordSha256 };
}
test('Windows operator admission binds exact helper binary/source CLI Node review and frozen oracle bytes', () => {
  const { capture } = fixture(), record = admission(capture), bytes = Buffer.from(canonical(record));
  capture.operatorAdmission.bytes = bytes.length; capture.operatorAdmission.sha256 = sha256(bytes);
  assert.deepEqual(validateWindowsAdmission(capture, bytes), record);
  assert.throws(() => validateWindowsAdmission(capture, Buffer.from(`${canonical(record)} `)));
  for (const change of [r => { r.tools.helper.sha256 = 'f'.repeat(64); }, r => { r.tools.cli.path = 'C:\\other.exe'; },
    r => { r.tools.node.bytes += 1; }, r => { r.helperBuiltFromSha256 = '0'.repeat(64); },
    r => { r.sourceTree = 'f'.repeat(40); }, r => { r.reviewRecordSha256 = 'f'.repeat(64); },
    r => { r.artifactOracleSha256 = 'f'.repeat(64); }, r => { r.allowArbitraryWorkload = true; }]) {
    const mutated = structuredClone(record); change(mutated); const changed = Buffer.from(canonical(mutated));
    const bound = structuredClone(capture); bound.operatorAdmission.bytes = changed.length; bound.operatorAdmission.sha256 = sha256(changed);
    assert.throws(() => validateWindowsAdmission(bound, changed));
  }
});
test('Windows absolute local path admission excludes UNC relative device aliases and alternate streams', () => {
  assert.equal(localAbsolute('C:\\synthetic\\helper.exe'), 'C:\\synthetic\\helper.exe');
  for (const path of ['helper.exe', '/synthetic/helper.exe', '\\\\host\\share\\helper.exe', '\\\\?\\C:\\helper.exe',
    'C:\\synthetic\\..\\helper.exe', 'C:\\helper.exe:extra', 'C:\\helper\n.exe']) assert.throws(() => localAbsolute(path));
});
function oracles() {
  return rows.map(row => ({ selector: row.id, manifest: row.expectedResponse.manifest,
    stdout: JSON.stringify(row.expectedResponse), stderr: '',
    files: [{ path: row.expectedResponse.manifest, bytes: 1, sha256: '0'.repeat(64) }] }));
}
test('Windows frozen correctness oracles require all 20 selectors exact public response and complete pinned inventory', () => {
  const { capture } = fixture(), original = oracles(); capture.artifactOracleSha256 = sha256(canonical(original));
  validateWindowsOracles(document, capture, original);
  const drift = oracles(); drift[0].stdout += ' ';
  assert.throws(() => validateWindowsOracles(document, capture, drift), /externally frozen/u);
  for (const change of [o => { o.pop(); }, o => { o[0].files = []; }, o => { o[0].stdout = '{}'; },
    o => { o[0].files[0].path = '.zryna/out/other/manifest.json'; }, o => { o[0].files.push({ ...o[0].files[0] }); },
    o => { o[0].files[0].sha256 = 'bad'; }, o => { delete o[0].stderr; }]) {
    const changed = oracles(); change(changed); capture.artifactOracleSha256 = sha256(canonical(changed));
    assert.throws(() => validateWindowsOracles(document, capture, changed));
  }
});
