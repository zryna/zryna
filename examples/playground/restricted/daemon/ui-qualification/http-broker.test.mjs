// Actual product HTTP service and real sockets; no browser/compiler/isolation acceptance claim.
import assert from 'node:assert/strict';
import test from 'node:test';
import { createService } from '../../service.mjs';
import { createHttpBroker } from './http-broker.mjs';
import { sha256 } from './wire.mjs';

async function fixture(run) {
  const template = Buffer.from('inert HTTP-only template'); let hostCalls = 0;
  const host = { compile() { hostCalls++; throw new Error('never admit fixture compilation'); },
    evaluate() { hostCalls++; throw new Error('never admit fixture evaluation'); }, async finish() {} };
  const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64),
    policySha256: 'c'.repeat(64), bindingTemplateSha256: sha256(template) };
  const html = Buffer.from('<p>Actual fixed resource bytes, transport fixture only.</p>');
  const service = createService({ host, authority, witSha256: 'd'.repeat(64), templateBytes: template,
    evaluationMode: 'supervised-host', resources: new Map([
      ['/examples/playground/restricted/', { bytes: html, type: 'text/html' }],
    ]) });
  const location = await service.listen(); const capability = new URL(location.url).hash.slice(1);
  const events = [];
  const broker = createHttpBroker({ origin: location.origin, capability, holdCompileProbe: true,
    onEvent: record => events.push(record) });
  const record = (id, route, headers = [], body = '') => ({ version: 1, type: 'request', id, route,
    method: ['compile', 'evaluate', 'finish', 'cancel'].includes(route) ? 'POST' : 'GET', headers,
    bodyBase64: Buffer.from(body).toString('base64') });
  const auth = [{ name: 'authorization', value: `Bearer ${capability}` }, { name: 'sec-fetch-site', value: 'same-origin' }];
  try { await run({ broker, record, auth, events, location, html, hostCalls: () => hostCalls }); }
  finally { await broker.close(); await service.close(); }
}
test('real service status/security/body bytes survive and absent browser authentication is not repaired', async () => {
  await fixture(async ({ broker, record, auth, html, hostCalls }) => {
    const page = await broker.request(record(1, 'index'));
    assert.equal(page.status, 200); assert.deepEqual(Buffer.from(page.bodyBase64, 'base64'), html);
    assert.ok(page.headers.find(entry => entry.name === 'content-security-policy').value.includes("default-src 'none'"));
    const missing = await broker.request(record(2, 'session'));
    assert.equal(missing.status, 400);
    assert.deepEqual(JSON.parse(Buffer.from(missing.bodyBase64, 'base64')), { policyError: 'PLAYGROUND-CAPABILITY' });
    const actual = await broker.request(record(3, 'session', auth));
    assert.equal(actual.status, 200);
    const session = JSON.parse(Buffer.from(actual.bodyBase64, 'base64'));
    assert.equal(session.version, 2); assert.equal(session.evaluationMode, 'supervised-host');
    assert.equal(hostCalls(), 0);
    const audit = JSON.stringify(broker.audit());
    assert.ok(!audit.includes('Bearer')); assert.ok(!audit.includes('authorization'));
  });
});
test('held first compile is canceled with no host effects; a later real service request is reusable', async () => {
  await fixture(async ({ broker, record, auth, events, location, hostCalls }) => {
    const headers = [...auth, { name: 'origin', value: location.origin }, { name: 'content-type', value: 'application/json' }];
    const pending = broker.request(record(1, 'compile', headers,
      JSON.stringify({ version: 1, revision: 1, source: 'export function zero(): i32 { return 0; }\n' })));
    assert.deepEqual(events, [{ version: 1, type: 'probe-paused', id: 1 }]);
    assert.equal(hostCalls(), 0); assert.equal(broker.abort(1), null);
    assert.deepEqual(await pending, { version: 1, type: 'aborted', id: 1 });
    assert.deepEqual(broker.audit(), [{ id: 1, route: 'compile', aborted: true, dispatched: false }]);
    const response = await broker.request(record(2, 'cancel', headers, '{"cancelled":true}'));
    assert.equal(response.status, 200); assert.equal(hostCalls(), 0);
    assert.throws(() => broker.abort(1), /UI-TRANSPORT/);
    assert.equal((await broker.request(record(3, 'session', auth))).status, 200);
  });
});
test('response-versus-abort race acknowledges a known completed ID once and rejects unknown ownership', async () => {
  await fixture(async ({ broker, record }) => {
    await broker.request(record(1, 'index'));
    assert.deepEqual(broker.abort(1), { version: 1, type: 'aborted', id: 1 });
    assert.throws(() => broker.abort(1), /UI-TRANSPORT/);
    assert.throws(() => broker.abort(2), /UI-TRANSPORT/);
    assert.throws(() => broker.request(record(3, 'index')), /UI-TRANSPORT/);
  });
});
test('concurrent broker close shares owned cleanup and settles a held request without dispatch', async () => {
  await fixture(async ({ broker, record, hostCalls }) => {
    const pending = broker.request(record(1, 'compile', [], '{}'));
    const rejected = assert.rejects(pending, /UI-STOPPED/);
    assert.deepEqual(broker.activeIds(), [1]);
    const first = broker.close(); const second = broker.close();
    assert.equal(first, second);
    await Promise.all([first, second, rejected]);
    assert.equal(hostCalls(), 0); assert.deepEqual(broker.activeIds(), []);
    assert.throws(() => broker.request(record(2, 'index')), /UI-TRANSPORT/);
  });
});
test('rapid cancellation retains physical ClientRequest ownership until close before dispatching replacements', async () => {
  await fixture(async ({ broker, record }) => {
    const first = Array.from({ length: 4 }, (_, index) => broker.request(record(index + 1, 'index')));
    assert.equal(broker.ownedSockets(), 4);
    for (let id = 1; id <= 4; id++) broker.abort(id);
    const replacements = Array.from({ length: 32 }, (_, index) => broker.request(record(index + 5, 'index')));
    assert.equal(broker.ownedSockets(), 4, 'destroyed sockets still consume a slot until actual close');
    assert.throws(() => broker.request(record(37, 'index')), /UI-TRANSPORT/, '32 queued bound survives closed-job socket ownership');
    assert.ok((await Promise.all(first)).every(value => value.type === 'aborted'));
    assert.ok((await Promise.all(replacements)).every(value => value.status === 200));
    assert.ok(broker.ownedSockets() <= 4);
  });
});
