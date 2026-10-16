import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdtempSync, readFileSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { createDaemonBridge, decodeHostFailure } from './bridge.mjs';

const pin = path => ({ path, bytes: statSync(path).size, ownerUid: statSync(path).uid,
  sha256: createHash('sha256').update(readFileSync(path)).digest('hex') });
const token = 'a'.repeat(64);
const request = Buffer.from(JSON.stringify({ version: 1, revision: 1, source: 'fn main() -> i32 { 1 }' }));
const record = { format: 'zryna.playground-host-error.v1', version: 1,
  reason: 'PLAYGROUND-CANCELLED', cleanupConfirmed: true, owner: null, primary: null, cleanup: null };
const encode = value => Buffer.from(JSON.stringify(value));

test('closed cleanup acknowledgement retains recovery owner and rejects ambiguity', () => {
  assert.equal(decodeHostFailure(encode(record)).cleanupConfirmed, true);
  const failed = { ...record, reason: 'PLAYGROUND-HOST-TEARDOWN', cleanupConfirmed: false,
    owner: 'b'.repeat(32), primary: 'PLAYGROUND-CANCELLED', cleanup: 'PLAYGROUND-HOST-TIMEOUT' };
  const decoded = decodeHostFailure(encode(failed));
  assert.equal(decoded.owner, failed.owner);
  assert.equal(decoded.primary, failed.primary);
  assert.equal(decoded.cleanup, failed.cleanup);
  for (const changed of [{ ...record, extra: true }, { ...record, owner: 'b'.repeat(32) },
    { ...record, cleanupConfirmed: 'true' }, { ...failed, owner: 'not-an-owner' }]) {
    assert.throws(() => decodeHostFailure(encode(changed)));
  }
  for (const bytes of [Buffer.concat([encode(record), encode(record)]),
    Buffer.from(JSON.stringify(record).replace('"version":1', '"version":1,"version":1')),
    Buffer.alloc(4097), encode(record).subarray(0, -1)]) assert.throws(() => decodeHostFailure(bytes));
});

const hostTest = process.platform === 'linux' && process.arch === 'x64';
test('no-follow independent material pins and malformed source fail before child creation',
  { skip: !hostTest }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'playground-bridge-fixture-'));
    try {
      const path = join(root, 'operator.json');
      writeFileSync(path, '{"fixture":"inert; not a daemon approval"}', { mode: 0o600 });
      const policy = { compiler: pin('/usr/bin/false'), approval: pin(path) };
      assert.throws(() => createDaemonBridge({ ...policy,
        compiler: { ...policy.compiler, sha256: '0'.repeat(64) } }), /HOST-MATERIAL/);
      const linked = join(root, 'link.json');
      symlinkSync(path, linked);
      assert.throws(() => createDaemonBridge({ ...policy, approval: { ...policy.approval, path: linked } }));
      const bridge = createDaemonBridge(policy);
      assert.throws(() => bridge.compile(Buffer.from('{"version":1,"revision":1,"source":"","argv":[]}'),
        undefined, token), /SHAPE/);
      const aborted = new AbortController(); aborted.abort();
      assert.throws(() => bridge.compile(request, aborted.signal, token), /CANCELLED/);
      await bridge.close();
    } finally { rmSync(root, { recursive: true }); }
  });

test('actual unknown nonzero helper exit permanently poisons admission',
  { skip: !hostTest }, async () => {
    // /usr/bin/false is a pinned inert negative fixture; it is not a host enforcement claim.
    const root = mkdtempSync(join(tmpdir(), 'playground-bridge-negative-'));
    try {
      const path = join(root, 'operator.json');
      writeFileSync(path, '{}', { mode: 0o600 });
      const bridge = createDaemonBridge({ compiler: pin('/usr/bin/false'), approval: pin(path) });
      await assert.rejects(bridge.compile(request, undefined, token), /HOST-TEARDOWN/);
      assert.ok(bridge.recovery());
      assert.throws(() => bridge.compile(request, undefined, 'b'.repeat(64)), /HOST-TEARDOWN/);
      await assert.rejects(bridge.finish(token, true), /HOST-TEARDOWN/);
      await assert.rejects(bridge.close(), /HOST-TEARDOWN/);
      assert.ok(bridge.recovery(), 'close cannot erase unknown cleanup recovery');
    } finally { rmSync(root, { recursive: true }); }
  });

test('actual cancelled helper teardown acknowledgement retains owner and poisons future jobs',
  { skip: !hostTest }, async () => {
    // Explicit inert protocol fault fixture, never an actual daemon enforcement claim.
    const root = mkdtempSync(join(tmpdir(), 'playground-bridge-teardown-'));
    try {
      const path = join(root, 'operator.json');
      writeFileSync(path, '{}', { mode: 0o600 });
      const helper = join(root, 'terminal-fixture');
      const acknowledgement = { ...record, reason: 'PLAYGROUND-HOST-TEARDOWN', cleanupConfirmed: false,
        owner: 'd'.repeat(32), primary: 'PLAYGROUND-CANCELLED', cleanup: 'PLAYGROUND-HOST-TIMEOUT' };
      writeFileSync(helper, `#!${process.execPath}\n` +
        `setTimeout(() => { process.stderr.write(${JSON.stringify(JSON.stringify(acknowledgement) + '\n')});` +
        'process.exitCode = 1; }, 25);\n', { mode: 0o700 });
      const bridge = createDaemonBridge({ compiler: pin(helper), approval: pin(path) });
      const cancellation = new AbortController();
      const result = bridge.compile(request, cancellation.signal, token);
      cancellation.abort();
      await assert.rejects(result, error => {
        assert.equal(error.message, 'PLAYGROUND-HOST-TEARDOWN');
        assert.equal(error.owner, acknowledgement.owner);
        assert.equal(error.primary, acknowledgement.primary);
        assert.equal(error.cleanup, acknowledgement.cleanup);
        assert.equal(error.cleanupConfirmed, false);
        return true;
      });
      assert.equal(bridge.recovery().owner, acknowledgement.owner);
      assert.throws(() => bridge.compile(request, undefined, 'b'.repeat(64)), /HOST-TEARDOWN/);
      await assert.rejects(bridge.close(), /HOST-TEARDOWN/);
      assert.equal(bridge.recovery().owner, acknowledgement.owner);
    } finally { rmSync(root, { recursive: true }); }
  });

test('actual truncated and extra helper terminal bytes cannot acknowledge cleanup',
  { skip: !hostTest }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'playground-bridge-terminal-bytes-'));
    try {
      const path = join(root, 'operator.json');
      writeFileSync(path, '{}', { mode: 0o600 });
      for (const [index, terminal] of [JSON.stringify(record).slice(0, -1),
        JSON.stringify(record) + JSON.stringify(record)].entries()) {
        const helper = join(root, `terminal-fixture-${index}`);
        writeFileSync(helper, `#!${process.execPath}\n` +
          `process.stderr.write(${JSON.stringify(terminal)}); process.exitCode = 1;\n`, { mode: 0o700 });
        const bridge = createDaemonBridge({ compiler: pin(helper), approval: pin(path) });
        await assert.rejects(bridge.compile(request, undefined, token), /HOST-TEARDOWN/);
        assert.ok(bridge.recovery());
        await assert.rejects(bridge.close(), /HOST-TEARDOWN/);
      }
    } finally { rmSync(root, { recursive: true }); }
  });

test('forced reaping cannot erase an already observed unconfirmed cleanup owner',
  { skip: !hostTest }, async () => {
    const root = mkdtempSync(join(tmpdir(), 'playground-bridge-stalled-ack-'));
    try {
      const path = join(root, 'operator.json');
      writeFileSync(path, '{}', { mode: 0o600 });
      const helper = join(root, 'terminal-fixture');
      const acknowledgement = { ...record, reason: 'PLAYGROUND-HOST-TEARDOWN', cleanupConfirmed: false,
        owner: 'e'.repeat(32), primary: 'PLAYGROUND-CANCELLED', cleanup: 'PLAYGROUND-HOST-TIMEOUT' };
      writeFileSync(helper, `#!${process.execPath}\n` +
        `process.stderr.write(${JSON.stringify(JSON.stringify(acknowledgement) + '\n')});` +
        'setInterval(() => {}, 1000);\n', { mode: 0o700 });
      const bridge = createDaemonBridge({ compiler: pin(helper), approval: pin(path) });
      const cancellation = new AbortController();
      const pending = bridge.compile(request, cancellation.signal, token);
      cancellation.abort();
      await assert.rejects(pending, error => error.message === 'PLAYGROUND-HOST-TEARDOWN' &&
        error.owner === acknowledgement.owner && error.cleanupConfirmed === false);
      assert.equal(bridge.recovery().owner, acknowledgement.owner);
      await assert.rejects(bridge.close(), /HOST-TEARDOWN/);
      assert.equal(bridge.recovery().owner, acknowledgement.owner);
    } finally { rmSync(root, { recursive: true }); }
  });
