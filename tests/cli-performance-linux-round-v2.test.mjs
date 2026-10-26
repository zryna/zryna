// SYNTHETIC controller/tool-output units. No child process, compiler, guest or benchmark runs.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonical, MAX_LOG, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { timeExecutable, PROVIDER_SHA256 } from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
import { captureLinuxDraft } from '../scripts/stability-gates/cli-performance-linux-io-v2.mjs';
import { collectLinuxRound, maxRss, summarizeFive } from '../scripts/stability-gates/cli-performance-linux-round-v2.mjs';
const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
const rows = matrixFor(document, 'linux-x86_64');
const metric = value => Buffer.from(`ZRYNA_M418_MAXRSS_KIB_V1=${value}\n`);
function fixture(change = () => {}) {
  const tool = path => ({ path, bytes: 1, sha256: '0'.repeat(64) });
  const capture = { format: 'zryna.cli-performance-linux-capture.draft.v2',
    catalogueSha256: sha256(canonical(document)), round: 'baseline-round-1', cliBuildClass: 'release',
    providerRecordSha256: PROVIDER_SHA256, providerRoot: '/tmp/m418-linux-provider-ABC123',
    providerReceiptSha256: '4'.repeat(64),
    host: { platform: 'linux-x86_64', synthetic: true }, sourceCommit: '0'.repeat(40), sourceTree: '1'.repeat(40),
    tools: { cli: tool('/synthetic/zryna'), time: timeExecutable('/tmp/m418-linux-provider-ABC123'), node: tool('/synthetic/node') },
    hostWallGuardMs: 1000, cleanupGuardMs: 10000, combinedOutputBytes: MAX_LOG,
    reviewRecordSha256: '2'.repeat(64), artifactOracleSha256: '3'.repeat(64) };
  const saved = new Map(); const events = [];
  let calls = 0, tick = 0n;
  const io = { root: '/synthetic/source', finishRound: () => events.push('finishRound'),
    retain: (name, bytes) => { assert(!saved.has(name)); saved.set(name, Buffer.from(bytes)); events.push(`retain:${name}`); },
    nowNs: () => { tick += 100n; return tick; },
    checkMaterials: () => { events.push('material'); },
    beforeAttempt: row => events.push(`before:${row.id}`),
    checkArtifactsAndStdout: row => events.push(`artifacts:${row.id}`),
    afterAttempt: row => events.push(`after:${row.id}`),
    execute: async (file, argv, guard) => {
      const index = calls++; events.push('execute');
      assert.equal(file, timeExecutable('/tmp/m418-linux-provider-ABC123').path); assert.equal(guard, 1000);
      const row = rows[Math.floor(index / 6)];
      const result = { exitCode: 0, signal: null, error: null,
        stdout: Buffer.from(JSON.stringify(row.expectedResponse)), stderr: metric([100, 5, 1, 3, 2, 4][index % 6]) };
      change({ result, index, argv, capture, events, row }); return result;
    } };
  return { capture, io, saved, events, calls: () => calls };
}
async function rejected(f, pattern) {
  await assert.rejects(collectLinuxRound(document, f.capture, f.io), pattern);
  return JSON.parse(f.saved.get('session.json'));
}
test('successor synthetic 24 cells retain warmup and five observations without becoming baseline authority', async () => {
  const f = fixture(); const session = await collectLinuxRound(document, f.capture, f.io);
  assert.equal(f.calls(), 144); assert.equal(session.cells.length, 24);
  assert(session.cells.every(row => row.attempts.length === 6));
  assert.deepEqual(session.cells[0].summary.peakRssKiB, { median: 3, maximum: 5, spread: 4 / 3 });
  assert.equal(session.status, 'collected-unqualified-observations');
  assert.equal(session.performanceQualified, false); assert.equal(session.sourceAuthenticated, false);
  assert.equal(session.baselineAuthority, null); assert.equal(session.thresholds, null);
  assert.equal(session.closureStatus, 'blocked'); assert.equal(f.saved.size, 435);
});
test('successor exact literal CLI commands replace only owned tool/root placeholders', async () => {
  const f = fixture(({ argv, row }) => {
    assert.deepEqual(argv.slice(0, 4), ['--format', 'ZRYNA_M418_MAXRSS_KIB_V1=%M', '--', '/synthetic/zryna']);
    assert(!argv.some(value => value.includes('$')));
    assert.equal(argv.includes('--node'), row.phase === 'run' && row.target !== 'native');
    assert.equal(argv[4], row.phase);
  });
  await collectLinuxRound(document, f.capture, f.io);
});
test('successor raw exit status stdout and stderr precede every correctness assertion', async () => {
  const f = fixture(({ result }) => { result.stdout = Buffer.from('{"wrong":true}'); });
  const session = await rejected(f, /original typed source outcome/u);
  assert.equal(session.status, 'invalid-or-incomplete'); assert.equal(f.calls(), 1);
  const id = 'm1-wrap_build_javascript_0';
  assert(f.saved.has(`${id}.stdout`) && f.saved.has(`${id}.stderr`) && f.saved.has(`${id}.json`));
  assert(!f.events.some(value => value.startsWith('artifacts:')));
});
test('successor timeout signal nonzero cleanup and spawn exceptions invalidate with no retries', async () => {
  for (const delta of [{ error: 'ETIMEDOUT' }, { signal: 'SIGILL' }, { exitCode: 7 }, { error: 'CLEANUP' }]) {
    const f = fixture(({ result }) => Object.assign(result, delta));
    await rejected(f); assert.equal(f.calls(), 1); assert.equal(f.saved.size, 6);
  }
  const f = fixture(); f.io.execute = async () => { throw new Error('synthetic spawn error'); };
  await rejected(f, /no timeout\/spawn\/cleanup\/output failure/u);
  assert(f.saved.has('m1-wrap_build_javascript_0.json'));
});
test('successor GNU time accepts exactly one positive safe metric and rejects aliases or warnings', () => {
  assert.equal(maxRss(metric(123)), 123);
  for (const bytes of [metric(0), metric(-1), metric('1.5'), metric('9999999999999999'),
    Buffer.from('123\n'), Buffer.concat([metric(1), metric(2)]),
    Buffer.concat([Buffer.from('warning\n'), metric(1)]), Buffer.alloc(129)]) {
    assert.throws(() => maxRss(bytes));
  }
});
test('successor all five observations determine median max and spread without outlier deletion', () => {
  const samples = [1000, 1, 2, 3, 4].map(value => ({ wallNs: value, peakRssKiB: value }));
  assert.deepEqual(summarizeFive(samples).wallNs, { median: 3, maximum: 1000, spread: 333 });
  assert.equal(samples[0].wallNs, 1000);
  for (const rows of [samples.slice(1), [...samples, samples[0]], [{ wallNs: 0 }, ...samples.slice(1)]]) {
    assert.throws(() => summarizeFive(rows));
  }
});
test('successor changed material or artifact failure stops session and retains the attempted raw capture', async () => {
  const f = fixture(); let checked = 0;
  f.io.checkMaterials = () => { if (++checked === 3) throw new Error('synthetic material drift'); };
  await rejected(f, /material drift/u); assert.equal(f.calls(), 1);
  const g = fixture(); g.io.checkArtifactsAndStdout = () => { throw new Error('synthetic artifact mismatch'); };
  await rejected(g, /artifact mismatch/u); assert.equal(g.calls(), 1);
  assert(g.saved.has('m1-wrap_build_javascript_0.stdout'));
});
test('successor guard caps method identity and caller-supplied selectors cannot widen the frozen catalogue', async () => {
  for (const mutate of [c => { c.hostWallGuardMs = 0; }, c => { c.hostWallGuardMs = 900001; },
    c => { c.cleanupGuardMs = 10001; }, c => { c.combinedOutputBytes += 1; },
    c => { c.catalogueSha256 = 'f'.repeat(64); }, c => { c.tools.time.path = '/synthetic/time'; },
    c => { c.host.platform = 'windows-x86_64'; }]) {
    const f = fixture(); mutate(f.capture);
    await assert.rejects(collectLinuxRound(document, f.capture, f.io)); assert.equal(f.calls(), 0);
  }
});

test('successor real IO entry rejects a synthetic invalid guard before any directory or process operation', async () => {
  const f = fixture(); f.capture.hostWallGuardMs = 0;
  await assert.rejects(captureLinuxDraft({ document, capture: f.capture, oracles: [],
    root: '/synthetic/nonexistent/source', output: '/synthetic/nonexistent/output' }));
  assert.equal(f.calls(), 0);
});

test('successor debug class only permits IO qualification, never a baseline or held-out round', async () => {
  const f = fixture(); f.capture.cliBuildClass = 'debug'; f.capture.round = 'io-qualification';
  const session = await collectLinuxRound(document, f.capture, f.io);
  assert.equal(f.calls(), 144); assert.equal(session.round, 'io-qualification');
  assert.equal(session.cliBuildClass, 'debug'); assert.equal(session.performanceQualified, false);
  for (const round of ['baseline-round-1', 'baseline-round-2', 'held-out-candidate']) {
    const g = fixture(); g.capture.cliBuildClass = 'debug'; g.capture.round = round;
    await assert.rejects(collectLinuxRound(document, g.capture, g.io), /debug-class/u);
    assert.equal(g.calls(), 0);
  }
});
test('successor tool bytes fresh receipt shape and operator record are exact bindings without caller substitution', async () => {
  for (const mutate of [c => { c.tools.time.sha256 = 'f'.repeat(64); },
    c => { c.tools.time.bytes += 1; }, c => { c.providerRecordSha256 = '0'.repeat(64); },
    c => { c.providerReceiptSha256 = 'bad'; }, c => { c.cliBuildClass = 'profiled'; }]) {
    const f = fixture(); mutate(f.capture);
    await assert.rejects(collectLinuxRound(document, f.capture, f.io)); assert.equal(f.calls(), 0);
  }
});
