import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, MAX_LOG, parse, sha256 } from '../scripts/stability-gates/input.mjs';
import { replayLinuxRound } from '../scripts/stability-gates/cli-performance-linux-replay.mjs';
import { linuxReplayFixture as fixture, rows, expected } from './cli-performance-linux-replay-fixture.mjs';
const id = 'm1-wrap_build_javascript_0';
function raw(f, change, name = id) {
  const path = `round/${name}.json`, value = parse(f.files.get(path)); change(value);
  f.files.set(path, Buffer.from(canonical(value)));
}
function rejected(change) { const f = fixture(); change(f); assert.throws(() => replayLinuxRound(f)); }
test('complete 24-cell 144-attempt replay independently reconciles known five-sample vectors excluding warmup', () => {
  const result = replayLinuxRound(fixture());
  assert.equal(result.cells, 24); assert.equal(result.attempts, 144); assert.equal(result.warmups, 24);
  assert.equal(result.measuredAttempts, 120); assert(result.rawMetricsReconciled);
  assert(result.summaries.every(row => JSON.stringify(row.summary) === JSON.stringify(expected)));
  for (const key of ['rawAuthenticated', 'sourceAuthenticated', 'hostAuthenticated', 'performanceQualified']) assert.equal(result[key], false);
  assert.equal(result.baselineAuthority, null); assert.equal(result.thresholds, null);
});
test('complete fixed GNU time invocation rejects executable wrapper flags CLI root Node and workload substitutions', () => {
  for (const change of [r => { r.call.executable = '/usr/bin/time'; }, r => { r.call.argv.push('--other'); },
    r => { r.call.argv[1] = '%M'; }, r => { r.call.argv[3] = '/other/cli'; },
    r => { r.call.argv[r.call.argv.indexOf('--root') + 1] = '/other/source'; },
    r => { r.call.argv[r.call.argv.indexOf('--target') + 1] = 'native'; }]) rejected(f => raw(f, change));
  rejected(f => raw(f, r => { r.call.argv[r.call.argv.indexOf('--node') + 1] = '/other/node'; }, 'm1-wrap_run_javascript_0'));
  rejected(f => raw(f, r => { r.call.argv[r.call.argv.indexOf('--arg=i32:1')] = '--arg=i32:2'; }, 'm1-wrap_run_javascript_0'));
});
test('raw process outcomes lengths identities indices and roles are exact rather than successful-looking labels', () => {
  for (const change of [r => { r.exitCode = 1; }, r => { r.exitCode = '0'; }, r => { r.error = 'CLEANUP'; },
    r => { r.signal = 'SIGTERM'; }, r => { r.stdoutBytes++; }, r => { r.stderrBytes = '32'; },
    r => { r.id += '_other'; }, r => { r.selector = rows[1].id; }, r => { r.index = '0'; },
    r => { r.role = 'measured'; }]) rejected(f => raw(f, change));
});
test('typed stdout and exact paired oracle bytes reject numeric type value whitespace and raw UTF8 drift', () => {
  for (const stdout of [Buffer.from('{}'), Buffer.from('null'), Buffer.from([255]),
    Buffer.from(`${JSON.stringify(rows[0].expectedResponse)} \n`)]) rejected(f => {
    f.files.set(`round/${id}.stdout`, stdout); raw(f, r => { r.stdoutBytes = stdout.length; });
  });
  rejected(f => {
    const name = 'm1-wrap_run_javascript_0', response = structuredClone(rows[3].expectedResponse);
    response.results[0].outcome.value.type = 'bool'; const stdout = Buffer.from(JSON.stringify(response));
    f.files.set(`round/${name}.stdout`, stdout); raw(f, r => { r.stdoutBytes = stdout.length; }, name);
  });
});
test('GNU wait4 maximum RSS KiB marker rejects warnings duplicate records units zero unsafe integers and Job COMMIT', () => {
  for (const text of ['0', '-1', '1.5', '9007199254740992', '1 KiB', '1\nZRYNA_M418_MAXRSS_KIB_V1=2']) rejected(f => {
    const stderr = Buffer.from(`ZRYNA_M418_MAXRSS_KIB_V1=${text}\n`);
    f.files.set(`round/${id}.stderr`, stderr); raw(f, r => { r.stderrBytes = stderr.length; });
  });
  for (const text of ['warning\nZRYNA_M418_MAXRSS_KIB_V1=1\n', 'peakJobMemoryUsedBytes=1\n']) rejected(f => {
    const stderr = Buffer.from(text); f.files.set(`round/${id}.stderr`, stderr); raw(f, r => { r.stderrBytes = stderr.length; });
  });
});
test('wall nanoseconds require exact positive bounded integer strings and all raw files must exist', () => {
  for (const wallNs of ['0', '-1', '1.5', '01', 100, '15000000001', '9007199254740992'])
    rejected(f => raw(f, r => { r.wallNs = wallNs; }));
  rejected(f => f.files.delete(`round/${id}.stderr`));
  rejected(f => f.files.set(`round/${id}.json`, Buffer.from('{}\n')));
});
test('session cells samples warmup roles and every median maximum spread must reconcile without omissions or alternatives', () => {
  for (const change of [f => f.session.cells.pop(), f => f.session.cells.reverse(),
    f => { f.session.cells[1] = structuredClone(f.session.cells[0]); },
    f => f.session.cells[0].attempts.pop(), f => { f.session.cells[0].attempts[1].role = 'warmup'; },
    f => { f.session.cells[0].attempts[5].peakRssKiB++; }]) rejected(change);
  for (const metric of ['wallNs', 'peakRssKiB']) for (const key of ['median', 'maximum', 'spread'])
    rejected(f => { f.session.cells[0].summary[metric][key]++; });
});
test('Linux method status catalogue capture operator and honest authority bindings cannot be relabeled', () => {
  for (const change of [f => { f.session.memoryMethod = 'job-peak-committed-bytes.v1'; },
    f => { f.session.memoryMeaning = 'summed RSS'; }, f => { f.session.status = 'invalid-or-incomplete'; },
    f => { f.session.round = 'baseline-round-2'; }, f => { f.session.captureSha256 = '0'.repeat(64); },
    f => { f.session.catalogueSha256 = '0'.repeat(64); }, f => { f.session.providerReceiptSha256 = '0'.repeat(64); },
    f => { f.operator.expectedTree = 'c'.repeat(40); }, f => { f.operator.root = '/other/source'; },
    f => { f.capture.artifactOracleSha256 = '0'.repeat(64); f.session.captureSha256 = sha256(canonical(f.capture)); }]) rejected(change);
  for (const where of ['capture', 'session']) for (const [key, value] of [['sourceAuthenticated', true],
    ['performanceQualified', true], ['baselineAuthority', {}], ['thresholds', {}]]) rejected(f => {
    f[where][key] = value; f.session.captureSha256 = sha256(canonical(f.capture));
  });
});
