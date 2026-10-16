// Protocol-negative tests only. No browser, compiler or qualification success is simulated.
import assert from 'node:assert/strict';
import test from 'node:test';
import { createHostSession } from './host-session.mjs';
import { routes } from './routes.mjs';
import { sha256 } from './wire.mjs';
import { createService } from '../../service.mjs';

function fixture(send = () => {}) {
  const configuration = { version: 1, type: 'start', runId: '1'.repeat(32), origin: 'http://127.0.0.1:12345',
    launchPath: '/examples/playground/restricted/', capability: 'a'.repeat(64), planSha256: 'b'.repeat(64) };
  const resourcePins = new Map(Object.entries(routes).filter(([name]) => name.startsWith('asset:') || name === 'index')
    .map(([, route]) => [route.path, { bytes: 1, sha256: sha256(Buffer.from('x')) }]));
  return { configuration, session: createHostSession({ configuration, send, resourcePins }) };
}
test('source-only host session rejects a fabricated passing record without actual transport observations', async () => {
  const sent = []; const { configuration, session } = fixture(record => sent.push(record));
  await session.begin(); assert.equal(sent.length, 1);
  assert.throws(() => session.begin(), /UI-TRANSPORT/);
  const record = { version: 1, type: 'result', runId: configuration.runId, phase: 'initial-abort-static-ax',
    transport: 'operator-stdio-http-interception-v1', abort: { id: 1, route: 'compile', paused: true,
      event: 'requestfailed', acknowledged: true }, initial: {}, state: {}, scripts: [],
    axBefore: [], axAfter: [], axUpdates: [], corpus: 'unrun', audibleAssistiveTechnology: 'unrun', browserClosed: true };
  assert.throws(() => session.accept(record), /UI-TRANSPORT/);
  await assert.rejects(session.result, /UI-TRANSPORT/);
  assert.equal(session.audit().length, 0); await session.stop();
});
test('EOF and deliberate stop settle the incomplete session and never establish container cleanup', async () => {
  const first = fixture().session;
  assert.throws(() => first.accept({ version: 1, type: 'abort', id: 1 }), /UI-TRANSPORT/);
  await assert.rejects(first.result, /UI-TRANSPORT/); await first.stop();
  const second = fixture().session; await second.begin(); second.eof();
  await assert.rejects(second.result, /UI-TRANSPORT/); await second.stop();
  const third = fixture().session; await third.begin(); await third.stop();
  await assert.rejects(third.result, /UI-STOPPED/);
  assert.ok(third.recovery());
});
test('stalled attached-stdin transmission cannot hold HTTP or outer cleanup beyond two seconds', { timeout: 5000 }, async () => {
  const templateBytes = Buffer.from('inert no-host-operation test');
  const service = createService({ evaluationMode: 'supervised-host', host: {
    compile() { assert.fail('no compiler admission'); }, evaluate() { assert.fail('no evaluator admission'); }, async finish() {},
  }, authority: { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64), policySha256: 'c'.repeat(64),
    bindingTemplateSha256: sha256(templateBytes) }, witSha256: 'd'.repeat(64), templateBytes,
  resources: new Map([['/examples/playground/restricted/', { type: 'text/html', bytes: Buffer.from('x') }]]) });
  const location = await service.listen();
  let transmitted;
  const writing = new Promise(resolve => { transmitted = resolve; });
  const selected = fixture(); await selected.session.stop();
  const configuration = { ...selected.configuration, origin: location.origin, capability: new URL(location.url).hash.slice(1) };
  const resourcePins = new Map(Object.entries(routes).filter(([name]) => name.startsWith('asset:') || name === 'index')
    .map(([, route]) => [route.path, { bytes: 1, sha256: sha256(Buffer.from('x')) }]));
  const session = createHostSession({ configuration, resourcePins, send(record) {
    if (record.type === 'response') { transmitted(); return new Promise(() => {}); }
  } });
  try {
    await session.begin();
    session.accept({ version: 1, type: 'request', id: 1, route: 'index', method: 'GET', headers: [], bodyBase64: '' });
    await writing;
    const before = performance.now(); const stop = session.stop();
    assert.equal(stop, session.stop());
    await assert.rejects(stop, /UI-CLEANUP/);
    assert.ok(performance.now() - before < 3000);
    assert.equal(session.recovery().message, 'PLAYGROUND-UI-CLEANUP');
    assert.equal(session.recovery().errors.length, 2, 'primary stop and cleanup failure both retained');
  } finally { await service.close(); }
});
test('every probe-event transmission is owned by stop and a scheduled start cannot dispatch after stop', { timeout: 5000 }, async () => {
  let called = 0;
  const fenced = fixture(() => { called++; }).session;
  const beginning = fenced.begin(); const stopped = fenced.stop();
  await Promise.all([beginning, stopped]);
  assert.equal(called, 0, 'microtask-scheduled start is fenced before the send callback');
  let paused;
  const writing = new Promise(resolve => { paused = resolve; });
  const selected = fixture(record => {
    if (record.type === 'probe-paused') { paused(); return new Promise(() => {}); }
  });
  await selected.session.begin();
  selected.session.accept({ version: 1, type: 'request', id: 1, route: 'compile', method: 'POST',
    headers: [], bodyBase64: Buffer.from('{}').toString('base64') });
  await writing;
  const before = performance.now();
  await assert.rejects(selected.session.stop(), /UI-CLEANUP/);
  assert.ok(performance.now() - before < 3000);
  assert.equal(selected.session.recovery().message, 'PLAYGROUND-UI-CLEANUP');
});
