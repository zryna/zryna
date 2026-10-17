// Protocol-negative tests only. No browser, compiler or qualification success is simulated.
import assert from 'node:assert/strict';
import test from 'node:test';
import { emptyBrowserProcess } from './wire.mjs';
import { createHostSession, validateFailureEvidence } from './host-session.mjs';
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
    transport: 'operator-stdio-browser-loopback-http-v1', abort: { id: 1, route: 'compile', paused: true,
      event: 'requestfailed', acknowledged: true, httpResponseClosed: true, requestBytes: 2, requestSha256: sha256(Buffer.from('{}')) }, initial: {}, state: {}, scripts: [],
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
test('corpus transition and evaluator admission cannot precede the genuine held-abort acknowledgement',async()=>{
 for(const record of [{version:1,type:'corpus-begin',id:1},{version:1,type:'request',id:1,route:'evaluate',method:'POST',headers:[],bodyBase64:'e30='}]){
  const{session}=fixture();await session.begin();assert.throws(()=>session.accept(record),/UI-TRANSPORT/);
  await assert.rejects(session.result,/UI-TRANSPORT/);assert.equal(session.audit().length,0);await session.stop();
 }
 const{session}=fixture();await session.begin();session.accept({version:1,type:'request',id:1,route:'compile',method:'POST',headers:[],bodyBase64:'e30='});
 assert.throws(()=>session.accept({version:1,type:'request',id:2,route:'compile',method:'POST',headers:[],bodyBase64:'e30='}),/UI-TRANSPORT/);
 await assert.rejects(session.result,/UI-TRANSPORT/);assert.equal(session.audit().length,0);await session.stop();
});
test('closed inner failure preserves fixed phase/cause without admitting arbitrary strings or another run', async () => {
  const selected = fixture();await selected.session.begin();
  selected.session.accept({ version: 1, type: 'failure', runId: selected.configuration.runId,
    phase: 'INITIAL_STATE', cause: 'INITIAL_HASH', cleanupFailed: false, cleanup: { intercept: false, context: false, browser: false }, lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false }, process: emptyBrowserProcess(), branch: 'UNSPECIFIED' });
  await assert.rejects(selected.session.result, error => {
    assert.equal(error.message, 'PLAYGROUND-UI-INNER-INITIAL-STATE-INITIAL-HASH');
    assert.match(error.message, /^PLAYGROUND-[A-Z0-9-]{1,100}$/);
    return true;
  });
  assert.equal(selected.session.audit().length, 0);await selected.session.stop();
  for (const extra of [{ runId: 'f'.repeat(32) }, { phase: 'private URL/#capability' },
    { lifecycle: { browserDisconnected: 1, pageClosed: false, pageCrashed: false, contextClosed: false } },
    { lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false, raw: 'secret' } },
    { process: { ...emptyBrowserProcess(), raw: 'private' } }, { process: null }, { lifecycle: null }, { phase: 'CDP_SETUP' }, { cause: 'Bearer secret' }, { cleanupFailed: 0 }, { raw: 'private stderr' },
    { branch: 'private branch' }, { cleanup: { intercept: true, context: false, browser: false } }]) {
    const invalid = fixture();await invalid.session.begin();
    assert.throws(() => invalid.session.accept({ version: 1, type: 'failure', runId: invalid.configuration.runId,
      phase: 'NAVIGATE', cause: 'TRANSPORT', cleanupFailed: false, cleanup: { intercept: false, context: false, browser: false }, lifecycle: { browserDisconnected: false, pageClosed: false, pageCrashed: false, contextClosed: false }, process: emptyBrowserProcess(), branch: 'UNSPECIFIED', ...extra }), /UI-TRANSPORT/);
    await assert.rejects(invalid.session.result, /UI-TRANSPORT/);await invalid.session.stop();
  }
});
test('first failing transport validator survives teardown and accepts no error-message branch spoof', async () => {
  const selected = fixture();await selected.session.begin();
  assert.throws(() => selected.session.accept({ version: 1, type: 'request', id: 2,
    route: 'index', method: 'GET', headers: [], bodyBase64: '' }), /UI-TRANSPORT/);
  await assert.rejects(selected.session.result, /UI-TRANSPORT/);
  assert.equal(selected.session.failureEvidence().firstBranch, 'BROKER_004');
  await selected.session.stop();assert.equal(selected.session.failureEvidence().firstBranch, 'BROKER_004');
  const unmarked = fixture();
  assert.equal(unmarked.session.failureEvidence(new Error('BROKER_004 private URL/#capability')).firstBranch, 'UNSPECIFIED');
  await unmarked.session.stop();
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
    const evidence = session.failureEvidence();
    assert.equal(evidence.audit.length, 1);assert.equal(evidence.audit[0].route, 'index');
    assert.equal(evidence.audit[0].status, 200);assert.equal(evidence.audit[0].responseSha256, sha256(Buffer.from('x')));
    assert.ok(!JSON.stringify(evidence).includes(configuration.capability));
    assert.ok(!JSON.stringify(evidence).includes('securityHeaders'));
    assert.throws(() => validateFailureEvidence({ ...evidence, firstBranch: 'private URL/#capability' }), /UI-TRANSPORT/);
    assert.throws(() => validateFailureEvidence({ ...evidence, audit: [{ ...evidence.audit[0], headers: 'Bearer private' }] }), /UI-TRANSPORT/);
    evidence.audit[0].route = 'private URL';assert.equal(session.failureEvidence().audit[0].route, 'index');
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
