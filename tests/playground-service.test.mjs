import assert from 'node:assert/strict';
import { test } from 'node:test';
import { sha256, utf8 } from '../examples/playground/restricted/limits.mjs';
import { source, sourceRequest, witSha256, templateBytes, deferred, frame, fixture,
  expectError, compile, startUpload } from './playground-fixtures/service.mjs';

test('external evaluation expiry returns a closed teardown acknowledgement and fresh admission', async t => {
  const f = await fixture(t);
  const first = await compile(f);
  await f.send('/finish', { token: first.token, cancelled: false });
  const artifact = await frame(sourceRequest);
  const reply = await f.send('/evaluate', { revision: sourceRequest.revision,
    sourceSha256: artifact.metadata.sourceSha256, componentSha256: artifact.metadata.identity.componentSha256 });
  assert.equal(reply.status, 200);
  f.host.finish = async () => true;
  const finished = await f.send('/finish', { token: reply.json().token, cancelled: false });
  assert.deepEqual(finished.json(), { finished: true, expired: true });
  f.host.finish = async () => false;
  const next = await compile(f);
  assert.equal(next.reply.status, 200);
  await f.send('/finish', { token: next.token, cancelled: false });
});

test('loopback resources are read-only and every response has restrictive browser headers', async t => {
  const f = await fixture(t);
  const reply = await f.send('/examples/playground/restricted/', null, { headers: {
    authorization: null, 'sec-fetch-site': null, origin: null } });
  assert.equal(reply.status, 200);
  assert.equal(reply.headers['cache-control'], 'no-store');
  assert.equal(reply.headers['x-content-type-options'], 'nosniff');
  assert.equal(reply.headers['referrer-policy'], 'no-referrer');
  assert.equal(reply.headers['cross-origin-resource-policy'], 'same-origin');
  assert.match(reply.headers['content-security-policy'], /default-src 'none'/);
  assert.match(reply.headers['content-security-policy'], /frame-ancestors 'none'/);
  await expectError(await f.send('/examples/playground/restricted/', {}), 'PLAYGROUND-ROUTE');
  for (const path of ['/package.json', '/run', '/exec', '/%2e%2e/package.json',
    '/compile?target=native', '/compile#x']) {
    assert.equal((await f.send(path, {})).status, 400);
  }
  assert.equal(f.calls.compile.length, 0);
  assert.equal(f.calls.evaluate.length, 0);
});

test('Host, Bearer, fetch-site and POST Origin checks precede every host operation', async t => {
  const f = await fixture(t);
  for (const headers of [{ host: 'localhost:1' }, { authorization: null },
    { authorization: `Bearer ${'0'.repeat(64)}` }, { 'sec-fetch-site': null },
    { 'sec-fetch-site': 'cross-site' }, { origin: null }, { origin: 'https://foreign.invalid' }]) {
    const reply = await f.send('/compile', sourceRequest, { headers });
    assert.equal(reply.status, 400);
  }
  await expectError(await f.send('/session', null, { headers: { authorization: null } }), 'PLAYGROUND-CAPABILITY');
  const session = await f.send('/session');
  assert.equal(session.status, 200);
  assert.deepEqual(session.json(), { version: 1, authority: f.authority, witSha256,
    template: Buffer.from(templateBytes).toString('base64') });
  assert.deepEqual(f.calls, { compile: [], evaluate: [], finish: [] });
});

test('closed request validation rejects generic commands and malformed source before provider use', async t => {
  const f = await fixture(t);
  const malformed = ['{}', '[]', '{"version":1,"revision":true,"source":""}',
    '{"version":1,"revision":1,"source":"","sou\\u0072ce":"other"}',
    '{"version":1,"revision":1,"source":"\\ud800"}',
    JSON.stringify({ ...sourceRequest, source: 'x'.repeat(4097) }),
    ...['argv', 'target', 'path', 'command', 'authority', 'package'].map(key =>
      JSON.stringify({ ...sourceRequest, [key]: 'unavailable' }))];
  for (const raw of malformed) assert.equal((await f.send('/compile', null, { raw })).status, 400);
  for (const headers of [{ 'content-type': 'text/plain' }, { 'content-encoding': 'gzip' }]) {
    assert.equal((await f.send('/compile', sourceRequest, { headers })).status, 400);
  }
  assert.equal(f.calls.compile.length, 0);
  assert.equal(f.calls.evaluate.length, 0);
  assert.equal((await compile(f)).reply.status, 200);
});

test('escaped UTF-8 source admits the byte limit and rejects the first extra request byte', async t => {
  const f = await fixture(t);
  const input = { ...sourceRequest, source: '\0'.repeat(4096) };
  const raw = JSON.stringify(input).padEnd(32768, ' ');
  const reply = await f.send('/compile', null, { raw });
  assert.equal(reply.status, 200);
  assert.equal(f.calls.compile.length, 1);
  await f.send('/finish', { token: reply.headers['x-zryna-job'], cancelled: false });
  assert.equal((await f.send('/compile', null, { raw: raw + ' ' })).status, 400);
  assert.equal(f.calls.compile.length, 1);
});

test('cancellation during request upload cannot invoke the provider after acknowledged cleanup', async t => {
  const f = await fixture(t);
  const upload = await startUpload(t, f, '/compile', sourceRequest);
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  assert.deepEqual((await f.send('/cancel', { cancelled: true })).json(), { finished: true });
  upload.complete();
  await expectError(await upload.pending, 'PLAYGROUND-CANCELLED');
  assert.equal(f.calls.compile.length, 0, 'an upload from a released job cannot start the provider');
});

test('cancelled evaluation upload cannot arm the supervisor after its job was released', async t => {
  const f = await fixture(t); const { token } = await compile(f);
  await f.send('/finish', { token, cancelled: false });
  const metadata = (await frame(sourceRequest)).metadata;
  const upload = await startUpload(t, f, '/evaluate', { revision: 7,
    sourceSha256: metadata.sourceSha256, componentSha256: metadata.identity.componentSha256 });
  assert.deepEqual((await f.send('/cancel', { cancelled: true })).json(), { finished: true });
  upload.complete();
  await expectError(await upload.pending, 'PLAYGROUND-CANCELLED');
  assert.equal(f.calls.evaluate.length, 0, 'released evaluation must not arm another supervisor');
});

test('job and receipt bind exact source revision, returned frame and launcher authority', async t => {
  const f = await fixture(t);
  const { reply, token } = await compile(f);
  const receipt = JSON.parse(Buffer.from(reply.headers['x-zryna-receipt'], 'base64').toString('utf8'));
  assert.deepEqual(receipt, { revision: 7, sourceSha256: await sha256(utf8(source)),
    frameSha256: await sha256(reply.bytes), compilerSha256: f.authority.compilerSha256,
    toolkitSha256: f.authority.toolkitSha256, policySha256: f.authority.policySha256 });
  assert.equal(f.calls.compile[0].token, token);
  assert.deepEqual(JSON.parse(new TextDecoder().decode(f.calls.compile[0].bytes)), sourceRequest);
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  await expectError(await f.send('/finish', { token: '0'.repeat(64), cancelled: false }), 'PLAYGROUND-JOB');
  assert.equal(f.calls.finish.length, 0);
  assert.deepEqual((await f.send('/finish', { token, cancelled: false })).json(), { finished: true });
  assert.deepEqual(f.calls.finish, [{ token, cancelled: false }]);
});

test('mismatched provider revision and source digest cannot issue a usable compilation receipt', async t => {
  for (const changes of [{ revision: 8 }, { sourceSha256: 'f'.repeat(64) }, { sourceBytes: 1 }]) {
    const f = await fixture(t, { async compile() { return (await frame(sourceRequest, changes)).bytes; } });
    const reply = await f.send('/compile', sourceRequest);
    await expectError(reply, 'PLAYGROUND-CORRELATION');
    assert.equal(reply.headers['x-zryna-job'], undefined);
    assert.equal(reply.headers['x-zryna-receipt'], undefined);
    assert.equal(f.calls.finish.length, 1);
    assert.equal(f.calls.finish[0].cancelled, true);
  }
});

test('evaluation accepts only the exact latest revision and hashes after compile cleanup', async t => {
  const f = await fixture(t); const { token } = await compile(f);
  const expected = (await frame(sourceRequest)).metadata;
  const value = { revision: 7, sourceSha256: expected.sourceSha256,
    componentSha256: expected.identity.componentSha256 };
  await expectError(await f.send('/evaluate', value), 'PLAYGROUND-BUSY', 409);
  await f.send('/finish', { token, cancelled: false });
  for (const invalid of [{ ...value, revision: 8 }, { ...value, sourceSha256: '0'.repeat(64) },
    { ...value, componentSha256: '0'.repeat(64) }, { ...value, argv: [] }]) {
    assert.equal((await f.send('/evaluate', invalid)).status, 400);
  }
  assert.equal(f.calls.evaluate.length, 0);
  const reply = await f.send('/evaluate', value);
  assert.equal(reply.status, 200);
  assert.equal(f.calls.evaluate.length, 1);
  assert.equal(f.calls.evaluate[0].token, reply.json().token);
  assert.deepEqual({ ...f.calls.evaluate[0].input }, value);
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  await f.send('/cancel', { cancelled: true });
  assert.equal(f.calls.finish.at(-1).cancelled, true);
});

test('cancellation aborts in-flight compile and reservation remains held until teardown acknowledgement', async t => {
  const entered = deferred(); const cleaned = deferred();
  const f = await fixture(t, {
    async compile(bytes, signal) {
      entered.resolve(signal);
      return await new Promise((resolve, reject) => signal.addEventListener('abort', () =>
        reject(new Error('PLAYGROUND-CANCELLED')), { once: true }));
    },
    async finish() { await cleaned.promise; },
  });
  const pending = f.send('/compile', sourceRequest);
  const signal = await entered.promise;
  const cancellation = f.send('/cancel', { cancelled: true });
  await new Promise(resolve => signal.addEventListener('abort', resolve, { once: true }));
  assert.equal(signal.aborted, true);
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  cleaned.resolve();
  assert.deepEqual((await cancellation).json(), { finished: true });
  await expectError(await pending, 'PLAYGROUND-CANCELLED');
  f.host.compile = async bytes => (await frame(JSON.parse(new TextDecoder().decode(bytes)))).bytes;
  await compile(f);
});

test('failed teardown permanently closes job admission instead of silently recovering', async t => {
  const f = await fixture(t, { async finish() { throw new Error('cleanup could not be proved'); } });
  const { token } = await compile(f);
  await expectError(await f.send('/finish', { token, cancelled: false }), 'PLAYGROUND-HOST-TEARDOWN');
  await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-HOST-TEARDOWN');
  assert.equal(f.calls.compile.length, 1);
});

test('overlapping finish acknowledgements cannot release another pending cleanup', async t => {
  const entered = deferred(); const first = deferred(); const second = deferred();
  let finishes = 0;
  const f = await fixture(t, { async finish() {
    const index = ++finishes;
    if (index === 1) { entered.resolve(); await first.promise; }
    if (index === 2) await second.promise;
  } });
  t.after(() => { first.resolve(); second.resolve(); });
  const { token } = await compile(f);
  const a = f.send('/finish', { token, cancelled: false });
  await entered.promise;
  const uploaded = deferred();
  f.service.server.once('request', incoming => incoming.once('end', () => uploaded.resolve()));
  const b = f.send('/finish', { token, cancelled: false });
  void b.catch(() => {});
  await uploaded.promise;
  await new Promise(resolve => setImmediate(resolve));
  first.resolve();
  await a;
  try {
    // Shared cleanup may settle both callers; an independent cleanup must keep admission closed.
    if (finishes > 1) await expectError(await f.send('/compile', sourceRequest), 'PLAYGROUND-BUSY', 409);
  } finally {
    second.resolve();
    await b;
  }
  assert.equal(finishes, 1, 'one job must have exactly one shared teardown operation');
});
