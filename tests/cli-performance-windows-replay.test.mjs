import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { replayWindowsRound } from '../scripts/stability-gates/cli-performance-windows-replay.mjs';

import { windowsReplayFixture as complete } from './cli-performance-windows-replay-fixture.mjs';
const id = 'round/m1-wrap_build_javascript_0';
function edit(f, path, change) { const value = JSON.parse(f.read(path)); change(value); f.put(path, value); }
function sessionEdit(f, change) { change(f.session); f.put('round/session.json', f.session); }
function helperEdit(f, change) {
  edit(f, `${id}.metadata.raw`, change);
  f.put(`${id}.sample/result.json`, f.read(`${id}.metadata.raw`));
  edit(f, `${id}.sample-read.json`, p => p.metadataBytes = f.read(`${id}.metadata.raw`).length);
}
test('complete 20 x 6 replay preserves independent exact five-vectors and no authority', async () => {
  const result = replayWindowsRound(await complete());
  assert.equal(result.rawMetricsReconciled, true);
  assert.deepEqual([result.cells, result.attempts, result.warmups, result.measuredAttempts], [20, 120, 20, 100]);
  assert.deepEqual(result.summaries[0].summary, {
    wallTicks: { median: '3', maximum: '5', qpcFrequency: '1000000000', spread: { numerator: '4', denominator: '3' } },
    peakJobMemoryUsedBytes: { median: '15', maximum: '19', spread: { numerator: '8', denominator: '15' } } });
  assert.equal(result.sourceAuthenticated, false); assert.equal(result.performanceQualified, false);
  assert.equal(result.baselineAuthority, null); assert.equal(result.thresholds, null);
});
const cases = {
  'QPC delta': f => helperEdit(f, h => h.qpcEnd = String(BigInt(h.qpcEnd) + 1n)),
  'COMMIT mismatch': f => helperEdit(f, h => h.peakJobMemoryUsedBytes = '998'),
  'lossy QPC number': f => helperEdit(f, h => h.qpcStart = Number(h.qpcStart)),
  'whole argv': f => edit(f, `${id}.process.json`, p => p.call.argv.push('--retry')),
  'helper executable': f => edit(f, `${id}.process.json`, p => p.call.executable = 'C:\\other.exe'),
  'sample directory': f => edit(f, `${id}.process.json`, p => p.call.sampleDirectory += 'x'),
  'process error': f => edit(f, `${id}.process.json`, p => p.error = 'ETIMEDOUT'),
  'process exit': f => edit(f, `${id}.process.json`, p => p.exitCode = 1),
  'process signal': f => edit(f, `${id}.process.json`, p => p.signal = 'SIGKILL'),
  'helper truncation': f => edit(f, `${id}.process.json`, p => p.helperStreams.stdout.truncated = true),
  'helper nonempty': f => f.put(`${id}.helper.stdout`, Buffer.from('x')),
  'sample read length': f => edit(f, `${id}.sample-read.json`, p => p.stdoutBytes++),
  'sample duplicate': f => f.put(`${id}.sample/stdout.raw`, Buffer.from('different')),
  'cleanup': f => helperEdit(f, h => h.cleanupConfirmed = false),
  'forced termination': f => helperEdit(f, h => h.forcedTermination = true),
  'unknown EOF': f => helperEdit(f, h => h.stdoutEof = false),
  'nonempty Job': f => helperEdit(f, h => h.jobEmptyConfirmed = false),
  'failed COMMIT query': f => helperEdit(f, h => h.peakQuerySucceeded = false),
  'RSS substitution': f => helperEdit(f, h => { delete h.peakJobMemoryUsedBytes; h.peakRssBytes = '999'; }),
  'role': f => sessionEdit(f, s => s.cells[0].attempts[0].role = 'measured'),
  'summary': f => sessionEdit(f, s => s.cells[0].summary.wallTicks.median = '4'),
  'outcome': f => {
    for (const p of [`${id}.stdout`, `${id}.sample/stdout.raw`]) f.put(p, Buffer.from('{}'));
    helperEdit(f, h => h.stdoutBytes = 2); edit(f, `${id}.sample-read.json`, p => p.stdoutBytes = 2);
  },
  'incomplete cell': f => sessionEdit(f, s => s.cells.pop()),
  'missing attempt': f => sessionEdit(f, s => s.cells[0].attempts.pop()),
  'memory meaning': f => sessionEdit(f, s => s.memoryMeaning = 'RSS resident bytes'),
  'wall meaning': f => sessionEdit(f, s => s.wallMeaning = 'elapsed milliseconds'),
  'false complete': f => sessionEdit(f, s => s.status = 'invalid-or-incomplete'),
  'round': f => { f.operator.round = 'baseline-round-2'; f.put('operator-input.json', f.operator); },
  'catalogue': f => f.put('round/catalogue.json', Buffer.from('{}\n')),
  'qualification': f => sessionEdit(f, s => s.sourceAuthenticated = true),
  'duplicate helper keys': f => {
    const raw = Buffer.from(f.read(`${id}.metadata.raw`).toString().replace('{', '{"wallTicks":"1000",'));
    f.put(`${id}.metadata.raw`, raw); f.put(`${id}.sample/result.json`, raw);
    edit(f, `${id}.sample-read.json`, p => p.metadataBytes = raw.length);
  },
};
for (const [name, mutate] of Object.entries(cases)) test(`reject ${name}`, async () => {
  const f = await complete(); mutate(f); assert.throws(() => replayWindowsRound(f));
});
