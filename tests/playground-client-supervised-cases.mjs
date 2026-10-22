import assert from 'node:assert/strict';
import { test } from 'node:test';
import { connect } from '../examples/playground/restricted/client.mjs';
import { observation } from './playground-fixtures/supervised.mjs';

// Fetch doubles prove handshake and teardown correlation; no compiler/executable evidence.
const capability = 'a'.repeat(64); const token = 'b'.repeat(64);
const authority = { compilerSha256: 'c'.repeat(64), toolkitSha256: 'd'.repeat(64),
  policySha256: 'e'.repeat(64), bindingTemplateSha256: 'f'.repeat(64) };
const session = { version: 2, evaluationMode: 'supervised-host', authority,
  witSha256: '1'.repeat(64), template: Buffer.from('transport fixture').toString('base64') };
const selection = { revision: 7, sourceSha256: '2'.repeat(64), componentSha256: '3'.repeat(64),
  logical: 'value', args: [] };
const json = value => new Response(JSON.stringify(value));

function fixture(t, routes = {}) {
  const calls = [];
  t.mock.method(globalThis, 'fetch', async (path, options) => {
    const value = options.body === undefined ? null : JSON.parse(options.body);
    calls.push({ path, options, value });
    assert.equal(options.headers.authorization, `Bearer ${capability}`);
    assert.equal(options.mode, 'same-origin'); assert.equal(options.credentials, 'omit');
    assert.equal(options.redirect, 'error'); assert.equal(options.cache, 'no-store');
    if (routes[path]) return routes[path](options, value);
    if (path === '/session') return json(session);
    if (path === '/evaluate') return json({ token, observation: observation(value) });
    if (path === '/finish' || path === '/cancel') return json({ finished: true });
    throw new Error('unexpected route');
  });
  return calls;
}

test('version2 client carries the trusted fixed mode and the same scalar observation through finish', async t => {
  const calls = fixture(t); const options = await connect(capability);
  assert.equal(options.evaluationMode, 'supervised-host');
  const reply = await options.host.evaluate(selection, new AbortController().signal);
  assert.deepEqual(reply, { token, observation: observation(selection) });
  assert.deepEqual(calls.at(-1).value, selection);
  await options.host.finish(null, false);
  assert.deepEqual(calls.at(-1).value, { token, cancelled: false });
  await assert.rejects(options.host.beginEvaluation(selection, new AbortController().signal), /EVALUATION-MODE/);
});

test('version2 session cannot select arbitrary evaluation modes extra fields or missing mode', async t => {
  for (const change of [{ evaluationMode: 'worker' }, { evaluationMode: 'remote' },
    { version: 3 }, { route: '/execute' }]) {
    fixture(t, { '/session': () => json({ ...session, ...change }) });
    await assert.rejects(connect(capability), /SESSION|SHAPE/);
  }
  const { evaluationMode, ...missing } = session;
  fixture(t, { '/session': () => json(missing) });
  await assert.rejects(connect(capability), /SHAPE/);
});

test('legacy version1 session cannot be upgraded by choosing the new evaluation method', async t => {
  const { evaluationMode, ...legacy } = session;
  const calls = fixture(t, { '/session': () => json({ ...legacy, version: 1 }) });
  const options = await connect(capability);
  assert.equal(options.evaluationMode, 'worker');
  await assert.rejects(options.host.evaluate(selection, new AbortController().signal), /EVALUATION-MODE/);
  assert.deepEqual(calls.map(call => call.path), ['/session']);
});

test('malformed supervised observation retains its known token for independent cleanup', async t => {
  for (const change of [{ revision: 8 }, { componentSha256: 'f'.repeat(64) }, { logical: 'other' },
    { value: '42' }, { value: 2147483648 }, { extra: true }]) {
    const calls = fixture(t, { '/evaluate': () => json({ token,
      observation: { ...observation(selection), ...change } }) });
    const options = await connect(capability);
    await assert.rejects(options.host.evaluate(selection, new AbortController().signal), /OBSERVATION|SHAPE/);
    await options.host.finish(null, true);
    assert.deepEqual(calls.at(-1).value, { token, cancelled: true });
  }
});

test('supervised response-selected authority and extra carrier fields reject while preserving cleanup token', async t => {
  const calls = fixture(t, { '/evaluate': () => json({ token, observation: observation(selection), authority }) });
  const options = await connect(capability);
  await assert.rejects(options.host.evaluate(selection, new AbortController().signal), /SHAPE/);
  await options.host.finish(null, true);
  assert.deepEqual(calls.at(-1).value, { token, cancelled: true });
});

test('aborted supervised response body invokes fixed cancel when no job token has been decoded', async t => {
  let cancelled = false;
  const body = new ReadableStream({ start() {}, cancel() { cancelled = true; } });
  const calls = fixture(t, { '/evaluate': () => new Response(body) });
  const options = await connect(capability); const abort = new AbortController();
  const pending = options.host.evaluate(selection, abort.signal);
  const outcome = assert.rejects(pending, /CANCELLED/);
  await new Promise(resolve => setImmediate(resolve)); abort.abort(); await outcome;
  assert.equal(cancelled, true);
  await options.host.finish(null, true);
  assert.equal(calls.at(-1).path, '/cancel'); assert.deepEqual(calls.at(-1).value, { cancelled: true });
});

test('supervised JSON reply rejects invalid job tokens noncanonical scalar numbers and byte overflow', async t => {
  for (const body of [JSON.stringify({ token: 'bad', observation: observation(selection) }),
    JSON.stringify({ token, observation: observation(selection) }).replace('"value":42', '"value":-0'),
    JSON.stringify({ token, observation: observation(selection) }).replace('"value":42', '"value":0.5'),
    ' '.repeat(2049)]) {
    fixture(t, { '/evaluate': () => new Response(body) });
    const options = await connect(capability);
    await assert.rejects(options.host.evaluate(selection, new AbortController().signal));
  }
});
