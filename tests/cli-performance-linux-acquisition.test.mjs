// Frozen pure acquisition controls: fake clock/process/files only; never invoke curl or network.
import assert from 'node:assert/strict';
import test from 'node:test';
import { DOWNLOAD_URL } from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
import { ACQUISITION_PARENT_MS, acquisitionInvocation, checkAcquisitionOutcome, collectAcquisition }
  from '../scripts/stability-gates/cli-performance-linux-acquire-provider.mjs';
const root = '/tmp/m418-linux-provider-AbC123';
const success = () => ({ status: 0, signal: null, error: null, stdout: Buffer.from('200'), stderr: Buffer.alloc(0) });
function fake(result, times) {
  const events = [], retained = new Map(); let index = 0;
  return { events, retained, io: { nowNs: () => times[index++],
    execute: command => { events.push('execute'); assert.equal(command.timeoutMs, ACQUISITION_PARENT_MS); return result; },
    retain: (name, bytes) => { events.push(`retain:${name}`); retained.set(name, Buffer.from(bytes)); },
    check: () => events.push('check'),
    readPackage: () => { events.push('readPackage'); return Buffer.alloc(50776); } } };
}
test('portable acquisition fixes curl URL TLS total deadline size cap and sole private output without caller controls', () => {
  const call = acquisitionInvocation(root);
  assert.equal(call.file, '/usr/bin/curl'); assert.equal(call.timeoutMs, 31000);
  assert.deepEqual(call.args, ['--disable', '--fail', '--silent', '--show-error', '--proto', '=https',
    '--proto-redir', '=https', '--max-redirs', '0', '--max-time', '30', '--connect-timeout', '10',
    '--max-filesize', '131072', '--output', `${root}/time.deb`, '--write-out', '%{http_code}', DOWNLOAD_URL]);
  assert(!call.args.includes('--insecure')); assert(!call.args.includes('--location'));
});
test('portable acquisition completion accepts exact finite success and rejects trickle past total monotonic deadline', () => {
  const startedNs = 1n, result = success();
  checkAcquisitionOutcome({ startedNs, endedNs: 31000000001n, result });
  for (const endedNs of [31000000002n, 0n])
    assert.throws(() => checkAcquisitionOutcome({ startedNs, endedNs, result }), /deadline|monotonic/u);
});
test('portable acquisition trickle cannot reset deadline and failed outcome remains retained before postflight', () => {
  const f = fake(success(), [1n, 31000000002n]);
  assert.throws(() => collectAcquisition({ root, io: f.io }), /deadline/u);
  assert(!f.events.includes('readPackage'));
  assert(f.events.indexOf('retain:acquisition-process.json') < f.events.indexOf('check'));
  assert.equal(JSON.parse(f.retained.get('acquisition.json')).status, 'invalid-or-incomplete');
});
test('portable acquisition EOF package read completion stays inside same total monotonic deadline', () => {
  const f = fake(success(), [1n, 100n, 31000000002n]);
  assert.throws(() => collectAcquisition({ root, io: f.io }), /deadline/u);
  assert(f.events.includes('readPackage'));
  assert.equal(JSON.parse(f.retained.get('acquisition.json')).status, 'invalid-or-incomplete');
});
test('portable acquisition timeout redirect TLS failure and overflow preserve bounded raw outcome plus material check', () => {
  for (const delta of [{ error: 'ETIMEDOUT' }, { status: 22 }, { stdout: Buffer.from('302') },
    { signal: 'SIGKILL' }, { error: 'ENOBUFS', stderr: Buffer.alloc(16385, 120) }]) {
    const f = fake({ ...success(), ...delta }, [1n, 2n]);
    assert.throws(() => collectAcquisition({ root, io: f.io }));
    assert(f.events.includes('check')); assert(!f.events.includes('readPackage'));
    assert(f.retained.get('acquisition.stderr').length <= 16384);
    const failure = JSON.parse(f.retained.get('acquisition.json'));
    assert.equal(failure.status, 'invalid-or-incomplete'); assert.equal(failure.partialPackageRetained, true);
    const raw = JSON.parse(f.retained.get('acquisition-process.json'));
    if (delta.error === 'ENOBUFS') assert.equal(raw.stderrTruncated, true);
  }
});
test('portable acquisition same-size wrong package hash records failure after read and never authorizes extraction', () => {
  const f = fake(success(), [1n, 2n, 3n]);
  assert.throws(() => collectAcquisition({ root, io: f.io }), /package hash/u);
  assert.equal(JSON.parse(f.retained.get('acquisition.json')).status, 'invalid-or-incomplete');
  assert(f.events.includes('check'));
});
