import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sha256, utf8 } from '../examples/playground/restricted/limits.mjs';
import { fixture, frame, sourceRequest, deferred, expectError, startUpload } from './playground-fixtures/service.mjs';
import { observation, turn } from './playground-fixtures/supervised.mjs';

// These synthetic frames establish service ownership contracts, not compiler/runtime acceptance.
async function supervised(t, overrides = {}) {
  const evaluations = [];
  const f = await fixture(t, { async evaluate(token, payload, signal) {
    evaluations.push({ token, payload, signal }); return observation(payload.selection);
  }, ...overrides }, { evaluationMode: 'supervised-host' });
  return { ...f, evaluations };
}

async function admitted(f, input = sourceRequest) {
  const reply = await f.send('/compile', input);
  assert.equal(reply.status, 200);
  const length = new DataView(reply.bytes.buffer, reply.bytes.byteOffset, reply.bytes.byteLength).getUint32(0, true);
  const metadata = JSON.parse(new TextDecoder().decode(reply.bytes.subarray(4, 4 + length)));
  await f.send('/finish', { token: reply.headers['x-zryna-job'], cancelled: false });
  return { reply, selection: { revision: input.revision, sourceSha256: metadata.sourceSha256,
    componentSha256: metadata.identity.componentSha256, logical: 'value', args: [] } };
}

test('supervised service advertises only the fixed version2 mode and transfers internally captured compiler bytes', async t => {
  const f = await supervised(t); const session = (await f.send('/session')).json();
  assert.deepEqual(Object.keys(session).sort(), ['authority', 'evaluationMode', 'template', 'version', 'witSha256']);
  assert.equal(session.version, 2); assert.equal(session.evaluationMode, 'supervised-host');
  const { reply, selection } = await admitted(f);
  const result = await f.send('/evaluate', selection);
  assert.equal(result.status, 200);
  const response = result.json();
  assert.deepEqual(response.observation, observation(selection));
  assert.match(response.token, /^[a-f0-9]{64}$/);
  const [{ token, payload, signal }] = f.evaluations;
  assert.equal(token, response.token); assert.equal(signal.aborted, false);
  assert.deepEqual(Object.keys(payload).sort(), ['authority', 'frameBytes', 'receipt', 'selection', 'sourceRequest']);
  assert.deepEqual(payload.sourceRequest, sourceRequest);
  assert.deepEqual(payload.frameBytes, reply.bytes);
  assert.deepEqual(payload.selection, selection); assert.deepEqual(payload.authority, f.authority);
  assert.equal(payload.receipt.frameSha256, await sha256(reply.bytes));
  assert.equal(payload.receipt.sourceSha256, await sha256(utf8(sourceRequest.source)));
  assert.equal(f.calls.evaluate.length, 0, 'legacy beginEvaluation is never armed');
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  await f.send('/finish', { token: response.token, cancelled: false });
});

test('supervised service captures immutable frame bytes independently of a mutable compiler response', async t => {
  const generated = await frame(sourceRequest); const original = generated.bytes.slice();
  const f = await supervised(t, { async compile() { return generated.bytes; } });
  const { selection } = await admitted(f);
  generated.bytes.fill(0);
  const result = await f.send('/evaluate', selection);
  assert.equal(result.status, 200);
  assert.deepEqual(f.evaluations[0].payload.frameBytes, original);
  assert.equal(f.evaluations[0].payload.receipt.frameSha256, await sha256(original));
});

test('supervised service refuses caller provenance modes paths arity and stale selection before daemon admission', async t => {
  const f = await supervised(t); const { selection } = await admitted(f);
  for (const extra of ['frameBytes', 'frameBase64', 'sourceRequest', 'receipt', 'authority',
    'evaluationMode', 'path', 'argv', 'command']) {
    await expectError(await f.send('/evaluate', { ...selection, [extra]: 'unavailable' }), 'PLAYGROUND-SHAPE');
    // A failed owned job invalidates reuse, requiring a fresh authenticated compilation.
    await admitted(f);
  }
  for (const change of [{ revision: selection.revision + 1 }, { sourceSha256: 'f'.repeat(64) },
    { componentSha256: 'f'.repeat(64) }, { logical: 'missing' }, { args: [1] }, { args: null }]) {
    assert.equal((await f.send('/evaluate', { ...selection, ...change })).status, 400);
    await admitted(f);
  }
  assert.equal(f.evaluations.length, 0);
});

test('supervised service validates signed i32 arguments with exact compiled export arity', async t => {
  const f = await supervised(t, { async compile() {
    return (await frame(sourceRequest, { exports: [{ logical: 'value', component: 'zryna-export-76616c7565',
      core: 'value', arity: 2 }] })).bytes;
  } });
  const { selection } = await admitted(f);
  const result = await f.send('/evaluate', { ...selection, args: [-2147483648, 2147483647] });
  assert.equal(result.status, 200);
  await f.send('/finish', { token: result.json().token, cancelled: false });
  for (const args of [[2147483648, 0], [0, -2147483649], [0.5, 0], ['0', 0], [0]]) {
    assert.equal((await f.send('/evaluate', { ...selection, args })).status, 400);
    await admitted(f);
  }
  assert.equal(f.evaluations.length, 1);
});

test('invalid compilation and a superseding source revision can never authorize an old evaluation', async t => {
  const f = await supervised(t); const { selection } = await admitted(f);
  f.host.compile = async () => new Uint8Array([0]);
  assert.equal((await f.send('/compile', sourceRequest)).status, 400);
  await expectError(await f.send('/evaluate', selection), 'PLAYGROUND-STALE');
  f.host.compile = async bytes => (await frame(JSON.parse(new TextDecoder().decode(bytes)))).bytes;
  await admitted(f, { ...sourceRequest, revision: 8, source: sourceRequest.source + '\n' });
  await expectError(await f.send('/evaluate', selection), 'PLAYGROUND-STALE');
  assert.equal(f.evaluations.length, 0);
});

test('supervised service rejects every wrong closed host observation and acknowledges cleanup first', async t => {
  for (const change of [{ revision: 1 }, { logical: 'missing' }, { componentSha256: 'f'.repeat(64) },
    { value: -0 }, { value: 1.5 }, { value: 2147483648 }, { value: '42' }, { extra: true }]) {
    const f = await supervised(t, { async evaluate(_token, payload) {
      return { ...observation(payload.selection), ...change };
    } });
    const { selection } = await admitted(f);
    assert.equal((await f.send('/evaluate', selection)).status, 400);
    assert.equal(f.calls.finish.at(-1).cancelled, true);
  }
});

test('supervised cancellation retains the reservation through independent daemon teardown', async t => {
  const entered = deferred(); const cleanup = deferred(); const f = await supervised(t, {
    async evaluate(_token, _payload, signal) {
      entered.resolve(); return await new Promise((resolve, reject) => {
        signal.addEventListener('abort', () => reject(new Error('PLAYGROUND-CANCELLED')), { once: true });
      });
    },
  });
  const { selection } = await admitted(f);
  f.host.finish = async () => { await cleanup.promise; };
  const pending = f.send('/evaluate', selection); await entered.promise;
  const cancellation = f.send('/cancel', { cancelled: true }); await turn();
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  cleanup.resolve();
  assert.deepEqual((await cancellation).json(), { finished: true });
  await expectError(await pending, 'PLAYGROUND-CANCELLED');
  await expectError(await f.send('/evaluate', selection), 'PLAYGROUND-STALE');
});

test('a cancelled supervised upload cannot execute after cleanup has been acknowledged', async t => {
  const f = await supervised(t); const { selection } = await admitted(f);
  const upload = await startUpload(t, f, '/evaluate', selection);
  assert.deepEqual((await f.send('/cancel', { cancelled: true })).json(), { finished: true });
  upload.complete(); await expectError(await upload.pending, 'PLAYGROUND-CANCELLED');
  assert.equal(f.evaluations.length, 0);
});

test('supervised teardown failure closes future compilation admission permanently', async t => {
  const f = await supervised(t); const { selection } = await admitted(f);
  f.host.finish = async () => { throw new Error('owned daemon remains'); };
  const result = await f.send('/evaluate', selection); assert.equal(result.status, 200);
  await expectError(await f.send('/finish', { token: result.json().token, cancelled: false }), 'PLAYGROUND-HOST-TEARDOWN');
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-HOST-TEARDOWN');
  // Let fixture cleanup finish without suppressing the already established poisoned state.
  f.host.finish = async () => false;
});

test('supervised HTTP preserves UTF8 rejection before host dispatch', async t => {
  const f = await supervised(t);
  await expectError(await f.send('/compile', null, { raw: Buffer.from([0xff]) }), 'PLAYGROUND-UTF8');
  assert.equal(f.calls.compile.length, 0);
  assert.equal(f.evaluations.length, 0);
  assert.equal(f.calls.finish.at(-1).cancelled, true);
});

test('supervised HTTP preserves bounded policy codes and sanitizes unknown host failures', async t => {
  const f = await supervised(t);
  for (const [message, expected] of [
    ['PLAYGROUND-CODE8', 'PLAYGROUND-CODE8'],
    [`PLAYGROUND-${'A'.repeat(100)}`, `PLAYGROUND-${'A'.repeat(100)}`],
    [`PLAYGROUND-${'A'.repeat(101)}`, 'PLAYGROUND-HOST'],
    ['private fixture failure /unavailable/path', 'PLAYGROUND-HOST'],
    ['PLAYGROUND-', 'PLAYGROUND-HOST'],
    ['PLAYGROUND-BAD_CODE', 'PLAYGROUND-HOST'],
  ]) {
    f.host.compile = async () => { throw new Error(message); };
    await expectError(await f.send('/compile', sourceRequest), expected);
  }
});
