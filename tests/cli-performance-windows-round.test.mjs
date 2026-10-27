import assert from 'node:assert/strict';
import test from 'node:test';
import { MAX_LOG } from '../scripts/stability-gates/input.mjs';
import { collectWindowsRound } from '../scripts/stability-gates/cli-performance-windows-round.mjs';
import { captureWindowsDraft } from '../scripts/stability-gates/cli-performance-windows-io.mjs';
import { document, fixture, rows } from './cli-performance-windows-fixture.mjs';
const first = 'm1-wrap_build_javascript_0';
async function rejected(f, pattern) {
  await assert.rejects(collectWindowsRound(document, f.capture, f.io), pattern);
  return JSON.parse(f.saved.get('session.json'));
}
function retained(f) {
  for (const suffix of ['helper.stdout', 'helper.stderr', 'process.json', 'metadata.raw', 'stdout', 'stderr', 'sample-read.json']) assert(f.saved.has(`${first}.${suffix}`));
}
test('Windows synthetic fixed 20 cells retain warmup and five samples without authority', async () => {
  const f = fixture(), session = await collectWindowsRound(document, f.capture, f.io);
  assert.equal(f.calls(), 120); assert.equal(session.cells.length, 20);
  assert.deepEqual(session.cells.map(row => row.selector), rows.map(row => row.id));
  assert(session.cells.every(row => row.attempts.length === 6 && row.attempts[0].role === 'warmup'));
  assert.equal(session.cells[0].attempts[0].wallTicks, '1000');
  assert.deepEqual(session.cells[0].summary.wallTicks, { median: '3', maximum: '5',
    spread: { numerator: '4', denominator: '3' }, qpcFrequency: '1000000000' });
  assert.equal(session.status, 'collected-unqualified-observations');
  assert.equal(session.sourceAuthenticated, false); assert.equal(session.performanceQualified, false);
  assert.equal(session.baselineAuthority, null); assert.equal(session.thresholds, null); assert.equal(session.closureStatus, 'blocked');
  assert.equal(session.memoryMethod, 'job-peak-committed-bytes.v1'); assert.equal(f.saved.size, 843);
  assert.equal(f.events.at(-2), 'finishRound');
});
test('Windows helper exact invocation has only fixed CLI argv and frozen placeholders', async () => {
  const f = fixture(({ argv, row }) => {
    assert.deepEqual(argv.slice(0, 6), ['--timeout-ms', '1000', '--output',
      argv[3], '--', 'C:\\synthetic\\zryna.exe']);
    assert(argv[3].startsWith('C:\\synthetic\\capture\\') && argv[3].endsWith('.sample'));
    const expected = row.argv.map(value => value === '$SOURCE_ROOT' ? 'C:\\synthetic\\source' :
      value === '$NODE' ? 'C:\\synthetic\\node.exe' : value);
    assert.deepEqual(argv.slice(6), expected); assert(!argv.some(value => value.includes('$')));
    assert(!(row.phase === 'run' && row.target === 'native'));
  });
  await collectWindowsRound(document, f.capture, f.io);
});
test('Windows raw process metadata and streams precede parser, correctness and postflight assertions', async () => {
  const f = fixture(({ raw }) => { raw.stdout = Buffer.from('{"wrong":true}'); });
  await rejected(f); retained(f); assert.equal(f.calls(), 1);
  const parsed = f.events.indexOf('parseHelper');
  for (const suffix of ['helper.stdout', 'helper.stderr', 'process.json', 'metadata.raw', 'stdout', 'stderr', 'sample-read.json'])
    assert(f.events.indexOf(`retain:${first}.${suffix}`) < parsed);
  assert(!f.events.some(event => event.startsWith('after:')));
  const g = fixture(); let checks = 0;
  g.io.checkMaterials = () => { if (++checks === 3) throw new Error('synthetic postflight drift'); };
  await rejected(g, /postflight drift/u); retained(g); assert.equal(g.calls(), 1);
});
test('Windows timeout, signal, nonzero, overflow and unknown cleanup never retry or parse as success', async () => {
  for (const delta of [{ error: 'ETIMEDOUT' }, { signal: 'SIGKILL' }, { exitCode: 1 },
    { error: 'ENOBUFS' }, { error: 'CLEANUP' }]) {
    const f = fixture(({ result }) => Object.assign(result, delta));
    const session = await rejected(f); retained(f);
    assert.equal(f.calls(), 1); assert.equal(session.status, 'invalid-or-incomplete');
    assert(!f.events.includes('parseHelper')); assert.equal(f.events.filter(value => value === 'material').length, 3);
  }
  const f = fixture(); f.io.execute = async () => { throw new Error('synthetic spawn'); };
  await rejected(f, /no helper timeout/u); retained(f);
});
test('Windows malformed, missing or changed helper metadata and wrong public outcomes retain attempted bytes', async () => {
  for (const change of [({ raw }) => { raw.metadata = Buffer.from('{bad'); },
    ({ raw }) => { raw.error = 'missing metadata'; },
    ({ metadata }) => { metadata.mainExitCode = '17'; },
    ({ raw, metadata }) => { raw.stdout = Buffer.from('{"wrong":true}'); metadata.stdoutBytes = raw.stdout.length; }]) {
    const f = fixture(change); await rejected(f); retained(f); assert.equal(f.calls(), 1);
  }
});
test('Windows invalid sample five invalidates the whole cell with no substitute observations', async () => {
  const f = fixture(({ index, metadata }) => { if (index === 5) metadata.stdoutEof = false; });
  const session = await rejected(f); assert.equal(f.calls(), 6); assert.equal(session.cells.length, 0);
  assert(f.saved.has('m1-wrap_build_javascript_5.metadata.raw'));
});
test('Windows artifact failures never delete a bundle or complete stage cleanup', async () => {
  const f = fixture(); f.io.checkArtifactsAndStreams = () => { throw new Error('synthetic artifact mismatch'); };
  await rejected(f, /artifact mismatch/u); retained(f); assert.equal(f.calls(), 1);
  assert(!f.events.some(value => value.startsWith('after:'))); assert(!f.events.includes('finishRound'));
});
test('Windows bad guards tool substitutions extra tools and wrong catalogue reject before execution', async () => {
  for (const mutate of [c => { c.hostWallGuardMs = 0; }, c => { c.hostWallGuardMs = 900001; },
    c => { c.cleanupGuardMs += 1; }, c => { c.combinedOutputBytes = MAX_LOG + 1; },
    c => { c.catalogueSha256 = 'f'.repeat(64); }, c => { c.tools.helper.path = '\\\\host\\share\\helper.exe'; },
    c => { c.tools.cli.path = 'C:\\synthetic\\zryna.exe:alternate'; },
    c => { c.tools.extra = c.tools.cli; }, c => { c.host.platform = 'linux-x86_64'; }]) {
    const f = fixture(); mutate(f.capture);
    await assert.rejects(collectWindowsRound(document, f.capture, f.io)); assert.equal(f.calls(), 0);
  }
});
test('Windows debug class permits IO qualification only and never opens a synthetic real adapter', async () => {
  const f = fixture(); f.capture.cliBuildClass = 'debug'; f.capture.round = 'io-qualification';
  assert.equal((await collectWindowsRound(document, f.capture, f.io)).performanceQualified, false);
  for (const round of ['baseline-round-1', 'baseline-round-2', 'held-out-candidate']) {
    const g = fixture(); g.capture.cliBuildClass = 'debug'; g.capture.round = round;
    await assert.rejects(collectWindowsRound(document, g.capture, g.io), /debug-class/u); assert.equal(g.calls(), 0);
  }
  f.capture.hostWallGuardMs = 0;
  await assert.rejects(captureWindowsDraft({ document, capture: f.capture, oracles: [],
    root: 'C:\\synthetic\\nonexistent', output: 'C:\\synthetic\\nonexistent-output' }));
});
