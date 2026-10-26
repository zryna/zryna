// Pure fake IO and metadata units. No child, helper, compiler, guest or network runs.
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, posix, resolve } from 'node:path';
import test from 'node:test';
import { MAX_LOG } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { collectCorrectnessOracles, validateOracleObservation } from '../scripts/stability-gates/cli-performance-oracle-capture.mjs';
import { validateWindowsOracleResult } from '../scripts/stability-gates/cli-performance-oracle-windows.mjs';
const document = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
function filesFor(row, host) {
  const files = [{ path: row.expectedResponse.manifest, bytes: 5, sha256: '1'.repeat(64) }];
  const family = document.families.find(f => row.id.startsWith(`${f.id}/`));
  const golden = family.existingBuildGoldens?.find(g => g.target === row.target);
  if (row.phase === 'build' && golden) files.push({ path: `${posix.dirname(row.expectedResponse.manifest)}/golden.bin`,
    bytes: golden.bytes, sha256: golden.sha256 });
  return files.sort((a, b) => a.path < b.path ? -1 : 1);
}
function fixture(host = 'linux-x86_64', change = () => {}) {
  const rows = matrixFor(document, host), saved = new Map(), events = []; let calls = 0;
  const options = { root: resolve('/synthetic/source'), output: resolve('/synthetic/evidence'), document,
    tools: { cli: resolve('/synthetic/zryna'), node: resolve('/synthetic/node'), ...(host === 'windows-x86_64' ? { helper: resolve('/synthetic/helper.exe') } : {}) },
    environment: { PATH: resolve('/synthetic/tools') }, host };
  const io = { toolPins: Object.fromEntries(Object.entries(options.tools).map(([k, path]) => [k, { path, bytes: 1, sha256: '2'.repeat(64) }])),
    retain: (name, bytes) => { assert(!saved.has(name)); saved.set(name, Buffer.from(bytes)); events.push(`retain:${name}`); },
    checkMaterials: () => events.push('materials'), stage: () => events.push('stage'), finishStage: () => events.push('finishStage'),
    beforeAttempt: row => events.push(`before:${row.id}`),
    run: async (call, id) => {
      const index = calls++, row = rows[Math.floor(index / 2)]; events.push(`run:${id}`);
      const stdout = Buffer.from(`${JSON.stringify(row.expectedResponse)}\n`), stderr = Buffer.alloc(0);
      const result = { stdout, stderr, outer: { exitCode: 0, signal: null, error: null, elapsedMs: 1,
        stdout: host === 'linux-x86_64' ? stdout : Buffer.alloc(0), stderr: Buffer.alloc(0) } };
      change({ result, row, index, call, id, events }); return result;
    },
    validateResult: () => events.push('validateResult'),
    snapshotArtifacts: (row, id) => { events.push(`copy:${id}`); return filesFor(row, host); },
    checkArtifacts: (_row, id) => events.push(`artifacts:${id}`),
    cleanup: (row, id) => events.push(`cleanup:${id}`) };
  return { options, io, saved, events, calls: () => calls, rows };
}
async function rejected(f, pattern) {
  await assert.rejects(collectCorrectnessOracles(f.options, f.io), pattern);
  return JSON.parse(f.saved.get('INVALID.json'));
}
test('portable oracle Linux captures 24 same-path pairs and both artifact copies before issuing only unmeasured oracles', async () => {
  const f = fixture(), result = await collectCorrectnessOracles(f.options, f.io);
  assert.equal(f.calls(), 48); assert.equal(result.cells, 24); assert.equal(result.commands, 48);
  assert.equal(result.oracles.length, 24); assert.equal(result.oracles[0].stderr, '');
  assert.equal(result.oracleFile.path, join(f.options.output, 'artifact-oracles.json'));
  assert.equal(result.sourceAuthenticated, false); assert.equal(result.performanceQualified, false);
  assert.equal(result.baselineAuthority, null); assert.equal(result.thresholds, null);
  assert.equal(f.events.filter(e => e.startsWith('copy:')).length, 48);
  assert(f.events.indexOf('copy:00-0') < f.events.indexOf('cleanup:00-0'));
  assert(f.events.indexOf('copy:00-1') < f.events.indexOf('cleanup:00-1'));
  assert(f.saved.has('artifact-oracles.json') && f.saved.has('result.json'));
});
test('portable oracle Windows captures only 20 pairs with explicit helper unchanged native golden and no native-run selector', async () => {
  const f = fixture('windows-x86_64'), result = await collectCorrectnessOracles(f.options, f.io);
  assert.equal(f.calls(), 40); assert.equal(result.cells, 20);
  assert(!result.oracles.some(o => o.selector.endsWith('/run/native')));
  assert(result.tools.helper); assert.equal(f.events.filter(e => e === 'validateResult').length, 40);
  const native = result.oracles.find(o => o.selector === 'm2-modules/build/native');
  const golden = document.families[1].existingBuildGoldens.find(g => g.target === 'native');
  assert(native.files.some(p => p.bytes === golden.bytes && p.sha256 === golden.sha256));
});
test('portable oracle invocation preserves literal V4 argv same output name guard and bound Node without environment dumps', async () => {
  const f = fixture('linux-x86_64', ({ call, row }) => {
    assert.equal(call.file, f.options.tools.cli); assert.equal(call.timeoutMs, 60000);
    assert.deepEqual(call.args, row.argv.map(v => v === '$SOURCE_ROOT' ? f.options.root : v === '$NODE' ? f.options.tools.node : v));
  });
  await collectCorrectnessOracles(f.options, f.io);
  const encodedEnvironmentPath = JSON.stringify(f.options.environment.PATH).slice(1, -1);
  for (const [name, bytes] of f.saved) if (name.endsWith('.json')) assert(!bytes.includes(Buffer.from(encodedEnvironmentPath)));
  const bad = fixture(); bad.options.document = structuredClone(document); bad.options.document.families[0].entry = 'other.zry';
  await assert.rejects(collectCorrectnessOracles(bad.options, bad.io)); assert.equal(bad.calls(), 0);
});
test('portable oracle failed subprocesses retain raw status before unconditional postflight and never clean or retry', async () => {
  for (const delta of [{ error: 'ETIMEDOUT' }, { error: 'CLEANUP' }, { error: 'MAXOUTPUT' }, { signal: 'SIGKILL' }, { exitCode: 9 }]) {
    const f = fixture('linux-x86_64', ({ result }) => Object.assign(result.outer, delta));
    await rejected(f); assert.equal(f.calls(), 1);
    for (const suffix of ['stdout', 'stderr', 'process.json', 'after.json']) assert(f.saved.has(`00-0.${suffix}`));
    assert(f.events.indexOf('retain:00-0.process.json') < f.events.indexOf('retain:00-0.after.json'));
    assert(!f.events.some(e => e.startsWith('cleanup:'))); assert(f.saved.has('failure-after.json'));
  }
  const f = fixture('linux-x86_64', ({ result }) => { result.stdout = Buffer.alloc(MAX_LOG + 1); });
  await rejected(f); assert.equal(f.saved.get('00-0.stdout').length, MAX_LOG);
  assert(f.saved.has('00-0.after.json') && f.saved.has('failure-after.json'));
});
test('portable oracle exact outcomes zero stderr complete inventories and unchanged M2 golden bytes reject drift', async () => {
  for (const change of [({ result }) => { result.stdout = Buffer.from('{bad'); },
    ({ result }) => { result.stdout = Buffer.from('{"wrong":true}'); },
    ({ result }) => { result.stderr = Buffer.from('warning'); }]) {
    const f = fixture('linux-x86_64', change); await rejected(f); assert(f.events.includes('copy:00-0'));
    assert(!f.events.includes('cleanup:00-0'));
  }
  const row = matrixFor(document, 'linux-x86_64').find(r => r.id === 'm2-modules/build/javascript');
  const stdout = Buffer.from(JSON.stringify(row.expectedResponse));
  const good = filesFor(row, 'linux-x86_64'); validateOracleObservation(document, row, 'linux-x86_64', stdout, Buffer.alloc(0), good);
  for (const files of [good.filter(p => p.path !== row.expectedResponse.manifest),
    good.map(p => p.path.endsWith('golden.bin') ? { ...p, sha256: 'f'.repeat(64) } : p), [...good, good[0]]])
    assert.throws(() => validateOracleObservation(document, row, 'linux-x86_64', stdout, Buffer.alloc(0), files));
});
test('portable oracle second replay stream or inventory drift keeps both copied attempts and leaves failing owned output', async () => {
  const f = fixture('linux-x86_64', ({ result, index }) => { if (index === 1) result.stdout = Buffer.concat([result.stdout, Buffer.from(' ')]); });
  await rejected(f, /paired/u); assert.equal(f.calls(), 2);
  assert(f.events.includes('copy:00-0') && f.events.includes('copy:00-1'));
  assert(f.events.includes('cleanup:00-0') && !f.events.includes('cleanup:00-1'));
  const g = fixture(); g.io.snapshotArtifacts = (row, id) => filesFor(row, 'linux-x86_64').map(pin =>
    id === '00-1' ? { ...pin, sha256: 'f'.repeat(64) } : pin);
  await rejected(g, /paired/u); assert.equal(g.calls(), 2); assert(!g.events.includes('cleanup:00-1'));
});
test('portable oracle artifact or material failures retain failure-after receipt and never claim stage cleanup', async () => {
  const f = fixture(); f.io.snapshotArtifacts = () => { throw new Error('copy verification failed'); };
  await rejected(f, /copy verification/u); assert(!f.events.some(e => e.startsWith('cleanup:')));
  assert(!f.events.includes('finishStage')); assert(f.saved.has('failure-after.json'));
  const g = fixture(); let checks = 0; g.io.checkMaterials = () => { if (++checks >= 4) throw new Error('tool or source drift'); };
  await rejected(g, /drift/u);
  assert.equal(JSON.parse(g.saved.get('failure-after.json')).status, 'failed');
  assert(!g.events.includes('cleanup:00-0'));
});
test('portable oracle Windows completion uses original strict helper predicate before accepting public output', () => {
  const stdout = Buffer.from('{}'), stderr = Buffer.alloc(0), call = { args: ['build'] };
  const h = { format: 'zryna.stability.windows-job-metrics.v1', metric: 'job-peak-committed-bytes.v1',
    measurementStatus: 'complete', timeoutMs: 60000, cleanupReserveMs: 10000, combinedStreamLimitBytes: MAX_LOG,
    childArgc: 2, commandUtf16Units: 20, childCreated: true, childPid: 123, mainExitKnown: true, mainExitCode: '0',
    stdoutBytes: stdout.length, stderrBytes: 0, stdoutEof: true, stderrEof: true, jobEmptyConfirmed: true,
    forcedTermination: false, cleanupConfirmed: true, peakQuerySucceeded: true, peakJobMemoryUsedBytes: '1',
    qpcFrequency: '1000', qpcStart: '1', qpcEnd: '2', wallTicks: '1', primaryStage: 0, primaryWin32Error: 0,
    cleanupStage: 0, cleanupWin32Error: 0 };
  const result = { stdout, stderr, readError: null, metadata: Buffer.from('{}\n'),
    outer: { exitCode: 0, signal: null, error: null, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) } };
  validateWindowsOracleResult(result, call, () => h);
  for (const mutate of [m => { m.jobEmptyConfirmed = false; }, m => { m.stdoutEof = false; },
    m => { m.cleanupConfirmed = false; }, m => { m.forcedTermination = true; }, m => { m.mainExitCode = '9'; }]) {
    const bad = structuredClone(h); mutate(bad); assert.throws(() => validateWindowsOracleResult(result, call, () => bad));
  }
  assert.throws(() => validateWindowsOracleResult({ ...result, readError: 'missing metadata' }, call, () => h));
});
