import assert from 'node:assert/strict';
import { test } from 'node:test';
import { controllerFixture, observation, turn } from './playground-fixtures/supervised.mjs';
import { deferred, source } from './playground-fixtures/service.mjs';

// Deterministic host doubles prove admission and cleanup only; no executable proof is implied.
test('supervised controller returns only the actual closed host observation and creates no UI Worker', async () => {
  const f = controllerFixture(); const metadata = await f.controller.compile();
  const result = await f.controller.evaluate('value', []);
  assert.deepEqual(result, { revision: 2, logical: 'value', value: 42,
    componentSha256: metadata.identity.componentSha256 });
  assert.deepEqual(f.calls[0].selection, { revision: 2, logical: 'value', args: [],
    sourceSha256: metadata.sourceSha256, componentSha256: metadata.identity.componentSha256 });
  assert.deepEqual(f.events.at(-1), { token: '2'.repeat(64), cancelled: false });
  assert.equal(f.workers.length, 0); assert.equal(f.timers.size, 0);
});

test('supervised controller rejects arity, scalar, export and cross-revision reuse before the host', async () => {
  const f = controllerFixture(); await f.controller.compile();
  for (const [logical, args] of [['missing', []], ['value', [1]], ['value', [-0]],
    ['value', [Infinity]], ['value', 'args']]) {
    await assert.rejects(f.controller.evaluate(logical, args), /ARGUMENTS/);
  }
  assert.equal(f.calls.length, 0);
  f.controller.edit(source + '\n');
  await assert.rejects(f.controller.evaluate('value', []), /COMPILE-FIRST/);
  assert.equal(f.workers.length, 0);
});

test('supervised controller rejects forged revision hash export scalar and extra observation fields', async () => {
  const f = controllerFixture(); await f.controller.compile();
  for (const change of [{ revision: 1 }, { logical: 'other' }, { componentSha256: 'f'.repeat(64) },
    { value: -0 }, { value: 2 ** 31 }, { value: 1.5 }, { value: '42' }, { extra: true }]) {
    f.host.evaluate = async selection => ({ token: '2'.repeat(64),
      observation: { ...observation(selection), ...change } });
    await assert.rejects(f.controller.evaluate('value', []), /OBSERVATION|SHAPE/);
    assert.equal(f.events.at(-1).token, '2'.repeat(64));
  }
  f.host.evaluate = async selection => ({ token: '2'.repeat(64),
    observation: observation(selection), authority: {} });
  await assert.rejects(f.controller.evaluate('value', []), /SHAPE/);
  assert.equal(f.workers.length, 0);
});

test('supervised controller cannot evaluate malformed compiler bytes or a substituted compiler receipt', async () => {
  for (const fault of ['frame', 'receipt']) {
    const f = controllerFixture(); const compile = f.host.compile;
    f.host.compile = async input => {
      const result = await compile(input);
      if (fault === 'frame') result.bytes = new Uint8Array([0]);
      else result.receipt.compilerSha256 = 'f'.repeat(64);
      return result;
    };
    await assert.rejects(f.controller.compile(), /FRAME|COMPILER-AUTHORITY/);
    await assert.rejects(f.controller.evaluate('value', []), /COMPILE-FIRST/);
    assert.equal(f.calls.length, 0); assert.equal(f.workers.length, 0);
  }
});

test('supervised cancellation keeps admission owned until teardown and ignores a late host result', async () => {
  const f = controllerFixture(); await f.controller.compile();
  const response = deferred(); const entered = deferred(); const cleanup = deferred();
  f.host.evaluate = async selection => { entered.resolve(selection); return response.promise; };
  f.host.finish = async () => cleanup.promise;
  const pending = f.controller.evaluate('value', []);
  const outcome = assert.rejects(pending, /CANCELLED/);
  const selection = await entered.promise;
  assert.equal(f.controller.cancel(), true); await turn();
  assert.equal(f.controller.busy, true);
  await assert.rejects(f.controller.compile(), /BUSY/);
  response.resolve({ token: '2'.repeat(64), observation: observation(selection) });
  cleanup.resolve(false); await outcome;
  await assert.rejects(f.controller.evaluate('value', []), /COMPILE-FIRST/);
  assert.equal(f.workers.length, 0);
});

for (const action of ['cancel', 'edit']) {
  test(`supervised ${action} during successful teardown cannot publish the queued observation`, async () => {
    const f = controllerFixture(); await f.controller.compile();
    const entered = deferred(); const cleanup = deferred();
    f.host.finish = async () => { entered.resolve(); return cleanup.promise; };
    const pending = f.controller.evaluate('value', []);
    const outcome = assert.rejects(pending, /CANCELLED/);
    await entered.promise;
    assert.equal(f.controller.busy, true);
    if (action === 'cancel') f.controller.cancel(); else f.controller.edit(source + '\n');
    await assert.rejects(f.controller.compile(), /BUSY/);
    cleanup.resolve(false); await outcome;
    await assert.rejects(f.controller.evaluate('value', []), /COMPILE-FIRST/);
    assert.equal(f.workers.length, 0);
  });
}

test('supervised deadline waits for teardown and teardown failure poisons all subsequent admission', async () => {
  const f = controllerFixture(); await f.controller.compile();
  f.host.evaluate = async () => new Promise(() => {});
  const cleanup = deferred(); const entered = deferred();
  f.host.finish = async () => { entered.resolve(); return cleanup.promise; };
  const pending = f.controller.evaluate('value', []);
  const outcome = assert.rejects(pending, /HOST-TEARDOWN/);
  const timer = [...f.timers.values()][0]; assert.equal(timer.ms, 5000); timer.fn();
  await entered.promise; assert.equal(f.controller.busy, true);
  cleanup.reject(new Error('daemon is still populated'));
  await outcome;
  await assert.rejects(f.controller.compile(), /HOST-TEARDOWN/);
  assert.equal(f.workers.length, 0);
});

test('supervised scalar selection admits exact i32 endpoints and refuses holes or malformed arguments locally', async () => {
  const f = controllerFixture(2); await f.controller.compile();
  assert.equal((await f.controller.evaluate('value', [-2147483648, 2147483647])).value, 42);
  for (const args of [[2147483648, 0], [-2147483649, 0], [0, -0], [0.5, 0],
    ['0', 0], [undefined, 0], new Array(2)]) {
    await assert.rejects(f.controller.evaluate('value', args), /ARGUMENTS/);
  }
  assert.equal(f.calls.length, 1); assert.equal(f.workers.length, 0);
});
