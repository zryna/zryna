import assert from 'node:assert/strict';
import test from 'node:test';
import { sha256 } from '../scripts/stability-gates/input.mjs';
import { replayOracleArchitecture } from '../scripts/stability-gates/cli-performance-oracle-replay.mjs';
import { oracleReplayFixture } from './cli-performance-oracle-replay-fixture.mjs';
const prefix = 'oracles/00-0';
function edit(f, name, change) { const value = JSON.parse(f.files.get(name)); change(value); f.put(name, value); }
function reject(change, windows = false) { const f = oracleReplayFixture(windows); change(f); assert.throws(() => replayOracleArchitecture(f)); }
test('full Linux/Windows synthetic pairs and architecture reconcile without execution qualification', () => {
  for (const windows of [false, true]) {
    const f = oracleReplayFixture(windows), result = replayOracleArchitecture(f);
    assert.deepEqual([result.cells, result.untimedAttempts, result.originalMaterials, result.stagingAliases], [windows ? 20 : 24, windows ? 40 : 48, 27, 2]);
    assert.equal(result.rawOracleArchitectureReconciled, true);
    assert.deepEqual([result.sourceAuthenticated, result.performanceQualified, result.hostAuthenticated, result.baselineAuthority, result.thresholds], [false, false, false, null, null]);
  }
});
test('complete calls statuses streams material-after claims and final oracle result cannot be relabeled', () => {
  for (const mutate of [p => p.call.args.push('--retry'), p => p.call.file += 'other', p => p.call.timeoutMs++,
    p => p.exitCode = 1, p => p.signal = 'SIGTERM', p => p.error = 'CLEANUP', p => p.readError = 'missing',
    p => p.stdoutBytes++, p => p.stdoutTruncated = true, p => p.outerCall = {}])
    reject(f => edit(f, `${prefix}.process.json`, mutate));
  reject(f => edit(f, `${prefix}.after.json`, p => p.status = 'failed'));
  for (const mutate of [r => r.cells--, r => r.commands++, r => r.sourceAuthenticated = true, r => r.status = 'invalid-or-incomplete'])
    reject(f => edit(f, 'oracles/result.json', mutate));
  reject(f => f.files.delete('oracles/23-1.process.json'));
});
test('both paired streams/inventories/artifacts and frozen manifest source semantics are checked', () => {
  reject(f => { const path = 'oracles/00-1.stdout', bytes = Buffer.concat([f.files.get(path), Buffer.from(' ')]);
    f.put(path, bytes); edit(f, 'oracles/00-1.process.json', p => p.stdoutBytes = bytes.length); });
  reject(f => { const name = f.oracles[0].files[0].path; f.put(`oracles/artifacts/00-0/${name}`, Buffer.from('changed')); });
  reject(f => { const path = f.oracles[0].manifest;
    for (const id of ['00-0', '00-1']) {
      const name = `oracles/artifacts/${id}/${path}`, m = JSON.parse(f.files.get(name)); m.entrypoint = 'other.zry';
      const raw = Buffer.from(JSON.stringify(m) + '\n'); f.put(name, raw);
      edit(f, `oracles/${id}.inventory.json`, pins => { const pin = pins.find(p => p.path === path); pin.bytes = raw.length; pin.sha256 = sha256(raw); });
      f.oracles[0].files = JSON.parse(f.files.get(`oracles/${id}.inventory.json`));
    }
    f.seal(); // All raw pins and final oracle digests agree; source semantics must still reject.
  });
  reject(f => f.readOriginal = () => Buffer.from('substituted original'));
  reject(f => { f.document = structuredClone(f.document); f.document.families[2].staging.copies[0].from = '../other'; });
  reject(f => f.put('round/artifact-oracles.json', []));
});
test('Windows original helper samples, native predicates and immutable invocation stay complete', () => {
  for (const mutate of [p => p.outerCall.args.push('--other'), p => p.outerCall.timeoutMs--, p => p.helperStdoutBytes = 1])
    reject(f => edit(f, `${prefix}.process.json`, mutate), true);
  for (const key of ['cleanupConfirmed', 'stdoutEof', 'stderrEof', 'jobEmptyConfirmed', 'peakQuerySucceeded']) reject(f => {
    const h = JSON.parse(f.files.get(`${prefix}.metadata.raw`)); h[key] = false;
    const raw = Buffer.from(JSON.stringify(h) + '\n'); f.put(`${prefix}.metadata.raw`, raw); f.put(`${prefix}.sample/result.json`, raw);
  }, true);
  reject(f => { const h = JSON.parse(f.files.get(`${prefix}.metadata.raw`)); h.wallTicks = '999';
    const raw = Buffer.from(JSON.stringify(h) + '\n'); f.put(`${prefix}.metadata.raw`, raw); f.put(`${prefix}.sample/result.json`, raw); }, true);
  reject(f => { const raw = Buffer.from(f.files.get(`${prefix}.metadata.raw`).toString().replace('{', '{"wallTicks":"1000",'));
    f.put(`${prefix}.metadata.raw`, raw); f.put(`${prefix}.sample/result.json`, raw); }, true);
  reject(f => f.put(`${prefix}.sample/stdout.raw`, Buffer.from('other')), true);
  reject(f => f.put(`${prefix}.helper.stderr`, Buffer.from('warning')), true);
});
test('architecture identity status logs, bounded reads and frozen input snapshots are mandatory', () => {
  for (const mutate of [c => c.args.push('--skip'), c => c.executable += 'other', c => c.cwd += 'other', c => c.timeoutMs++])
    reject(f => edit(f, 'architecture.command.json', mutate));
  for (const mutate of [r => r.exitCode = 1, r => r.error = 'ETIMEDOUT', r => r.signal = 'SIGKILL', r => r.logs.stdout.sha256 = '0'.repeat(64)])
    reject(f => edit(f, 'architecture.result.json', mutate));
  reject(f => f.put('architecture.stdout.raw', Buffer.from('other')));
  reject(f => f.put('oracles/input.json', Buffer.alloc(262145)));
  const f = oracleReplayFixture(), read = f.read;
  f.read = (name, maximum) => { f.capture.sourceCommit = '9'.repeat(40); return read(name, maximum); };
  assert.equal(replayOracleArchitecture(f).cells, 24);
});
