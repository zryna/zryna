// Creation-owned child doubles only; no interpreter, Engine, installed CLI or provider is launched.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EventEmitter } from 'node:events';
import { PassThrough, Writable } from 'node:stream';
import childProcess from 'node:child_process';
import { readFileSync } from 'node:fs';
import { createContainedCliAdapter } from './runtime-adapter.mjs';
import { findOperation } from './plan.mjs';
import { parityPlanSha256 } from './plan.mjs';
import { sha256 } from './records.mjs';

const approval = { format: 'zryna.playground-cli-owner-approval.v1', version: 1,
  image: 'sha256:' + 'a'.repeat(64), layer_sha256: 'b'.repeat(64), image_plan_sha256: 'c'.repeat(64),
  seccomp: readFileSync(new URL('./compiler-seccomp.json', import.meta.url), 'utf8'),
  seccomp_sha256: 'bf4a72a0da350e14260d8b1534967de60bb40dff11f088c0eeae3e455c74c33b',
  command_plan_sha256: 'd3fd1e15a7fd200aa402b05b8164726f2f4000f1489401c2414f71f408261efb' };
function child() {
  const value = new EventEmitter(); value.stdout = new PassThrough(); value.stderr = new PassThrough();
  value.stdin = new Writable({ write(_bytes, _encoding, callback) { callback(); } });
  value.stdio = [value.stdin, value.stdout, value.stderr, new PassThrough()];
  value.kills = []; value.kill = signal => { value.kills.push(signal); return true; };
  return value;
}
const context = signal => ({ kind: 'cli', id: 'build-addition', signal, activeMs: 10000, cleanupMs: 2000 });
function receipt() {
  const stdout = Buffer.from('{"synthetic":"comparison fixture only"}');
  return { format: 'zryna.playground-cli-owner-observation.v1', version: 1, operationId: 'build-addition',
    qualificationOnly: true, cleanupConfirmed: true, cleanupFailure: null, ownershipUnconfirmed: false,
    removed: true, ownedVolumes: [], failure: null, operationStatus: 'captured-source-outcome', exec: { capture: {
      exitCode: 0, stdoutBase64: stdout.toString('base64'), stderrBase64: '', stdoutBytes: stdout.length,
      stderrBytes: 0, stdoutSha256: sha256(stdout), stderrSha256: sha256(Buffer.alloc(0)) } } };
}
function respond(fake, value = receipt(), code = 0) {
  fake.stdout.emit('data', Buffer.from(JSON.stringify(value) + '\n')); fake.emit('close', code);
}
test('actual adjacent Python fixed plan must equal the current comparison plan and approval before any spawn', t => {
  let calls = 0; t.mock.method(childProcess, 'spawn', () => { calls++; return child(); });
  const pinnedBytes = readFileSync(new URL('./fixed-plan.json', import.meta.url));
  assert.equal(sha256(pinnedBytes), parityPlanSha256());
  assert.equal(approval.command_plan_sha256, parityPlanSha256());
  assert.throws(() => createContainedCliAdapter({ ...approval, command_plan_sha256: 'f'.repeat(64) }), /CLI-PARITY/);
  assert.equal(calls, 0);
});
test('adapter invokes only fixed trusted interpreter/owner and fixed opID, holds capture until confirmed child cleanup', async t => {
  const fake = child(); const calls = [];
  t.mock.method(childProcess, 'spawn', (...args) => { calls.push(args); return fake; });
  const owner = createContainedCliAdapter(approval); const controller = new AbortController();
  const pending = owner.adapter.run(findOperation('build-addition'), context(controller.signal));
  assert.equal(calls[0][0], '/usr/bin/python3'); assert.deepEqual(calls[0][1].slice(0, 2), ['-I', '-S']);
  assert.match(calls[0][1][2], /\/cli-parity\/cli_main\.py$/);
  assert.deepEqual(calls[0][2].env, { PATH: '/nonexistent', LC_ALL: 'C.UTF-8', HOME: '/nonexistent' });
  respond(fake); const captured = await pending; assert.equal(captured.stderr.length, 0);
  assert.equal(owner.evidence().length, 0);
  assert.deepEqual(await owner.adapter.cleanup({ cancelled: false, signal: controller.signal }), { confirmed: true });
  assert.equal(owner.evidence().length, 1); assert.equal(owner.poisoned(), false); assert.deepEqual(fake.kills, []);
});
test('changed operation, unsupported context and pre-aborted run never spawn', async t => {
  let calls = 0; t.mock.method(childProcess, 'spawn', () => { calls++; return child(); });
  const owner = createContainedCliAdapter(approval); const controller = new AbortController();
  const changed = structuredClone(findOperation('build-addition')); changed.argv.push('--node=/bin/sh');
  await assert.rejects(owner.adapter.run(changed, context(controller.signal)), /CLI-PARITY/);
  await assert.rejects(owner.adapter.run(findOperation('build-addition'), { ...context(controller.signal), activeMs: 10001 }), /CLI-PARITY/);
  controller.abort(); await assert.rejects(owner.adapter.run(findOperation('build-addition'), context(controller.signal)), /ABORTED/);
  assert.equal(calls, 0);
});
test('caller abort closes control pipe and requires actual closed child and cleanup receipt', async t => {
  const fake = child(); t.mock.method(childProcess, 'spawn', () => fake);
  const owner = createContainedCliAdapter(approval); const controller = new AbortController();
  const pending = owner.adapter.run(findOperation('build-addition'), context(controller.signal));
  const rejected = assert.rejects(pending, /ABORTED/);
  controller.abort(); assert.equal(fake.stdio[3].writableEnded, true); respond(fake);
  await rejected;
  assert.deepEqual(await owner.adapter.cleanup({ cancelled: true, signal: new AbortController().signal }), { confirmed: true });
});
test('unconfirmed or malformed receipt, stdout overflow and unsolicited control output poison admission', async t => {
  for (const mode of ['unknown', 'malformed', 'overflow', 'control-output']) {
    const fake = child(); t.mock.method(childProcess, 'spawn', () => fake);
    const owner = createContainedCliAdapter(approval); const controller = new AbortController();
    const pending = owner.adapter.run(findOperation('build-addition'), context(controller.signal));
    const rejected = assert.rejects(pending, /OWNER-/);
    if (mode === 'unknown') { const value = receipt(); value.cleanupConfirmed = false; respond(fake, value, 70); }
    if (mode === 'malformed') { fake.stdout.emit('data', Buffer.from('{')); fake.emit('close', 70); }
    if (mode === 'overflow') { fake.stdout.emit('data', Buffer.alloc(1048577)); fake.emit('close', 70); }
    if (mode === 'control-output') { fake.stdio[3].emit('data', Buffer.from('unexpected')); respond(fake); }
    await rejected;
    assert.deepEqual(await owner.adapter.cleanup({ cancelled: true, signal: controller.signal }), { confirmed: false });
    assert.equal(owner.poisoned(), true);
    await assert.rejects(owner.adapter.run(findOperation('build-addition'), context(controller.signal)), /CLI-PARITY/);
  }
});
test('cleanup deadline kills only its original created child and remains unknown', async t => {
  t.mock.timers.enable({ apis: ['setTimeout'] });
  const fake = child(); t.mock.method(childProcess, 'spawn', () => fake);
  const owner = createContainedCliAdapter(approval); const controller = new AbortController();
  const pending = owner.adapter.run(findOperation('build-addition'), context(controller.signal));
  const clean = owner.adapter.cleanup({ cancelled: true, signal: controller.signal });
  t.mock.timers.tick(2000); assert.deepEqual(await clean, { confirmed: false });
  assert.deepEqual(fake.kills, ['SIGKILL']); assert.equal(owner.poisoned(), true);
  fake.emit('close', null); await assert.rejects(pending, /OWNER-/);
});
