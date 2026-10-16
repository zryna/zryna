// Deferred header fixtures only: no actual browser, socket, compiler or qualification claim.
import assert from 'node:assert/strict';
import test from 'node:test';
import { interceptBrowser } from './browser-intercept.mjs';
import { bounds } from './wire.mjs';

const origin = 'http://127.0.0.1:12345';
const deferred = () => { let resolve; const promise = new Promise(value => { resolve = value; }); return { promise, resolve }; };
async function fixture() {
  let handler; const sent = []; const waiters = new Map();
  const context = { on() {}, off() {}, async route(_, value) { handler = value; }, async unroute() {} };
  const intercept = await interceptBrowser(context, origin, value => {
    sent.push(value); waiters.get(sent.length)?.();
  });
  const waitFor = size => sent.length >= size ? Promise.resolve() : new Promise(resolve => waiters.set(size, resolve));
  function request(path = '/examples/playground/restricted/') {
    const headers = deferred(); const aborted = [];
    const value = { url: () => origin + path, method: () => 'GET', resourceType: () => 'document',
      postDataBuffer: () => null, allHeaders: () => headers.promise };
    const operation = handler({ request: () => value, async abort(reason) { aborted.push(reason); },
      async fulfill() { assert.fail('fixture has no successful browser response'); } });
    return { headers, aborted, operation };
  }
  return { intercept, sent, request, waitFor };
}
test('concurrent header reads reserve unique consecutive IDs in actual send order', async () => {
  const f = await fixture();
  const first = f.request(); const second = f.request('/examples/playground/restricted/style.css');
  second.headers.resolve({}); await f.waitFor(1);
  assert.equal(f.sent[0].id, 1); assert.equal(f.sent[0].route, 'asset:style.css');
  first.headers.resolve({}); await f.waitFor(2);
  assert.deepEqual(f.sent.map(value => value.id), [1, 2]);
  assert.equal(f.sent[1].route, 'index');
  await f.intercept.stop(); await Promise.all([first.operation, second.operation]);
});
test('simultaneous header completions cannot overbook the pending bound', async () => {
  const f = await fixture(); const limit = bounds.live + bounds.queued;
  const requests = Array.from({ length: limit + 1 }, () => f.request());
  for (const request of requests) request.headers.resolve({});
  await f.waitFor(limit); await requests.at(-1).operation;
  assert.equal(f.sent.length, limit);
  assert.deepEqual(f.sent.map(value => value.id), Array.from({ length: limit }, (_, index) => index + 1));
  assert.deepEqual(requests.at(-1).aborted, ['failed']);
  await assert.rejects(f.intercept.failures, /UI-TRANSPORT/);
  await f.intercept.stop(); await Promise.all(requests.map(value => value.operation));
});
test('stop while headers are pending fences subsequent ID reservation and send', async () => {
  const f = await fixture(); const request = f.request();
  await f.intercept.stop(); request.headers.resolve({}); await request.operation;
  assert.equal(f.sent.length, 0); assert.deepEqual(request.aborted, ['failed']);
});
test('the first route validator branch is retained before abort can trigger a different navigation error', async () => {
  const f = await fixture();const request = f.request('/unapproved-route');
  assert.match(f.intercept.failureBranch(), /^ROUTES_[0-9]{3}$/);
  await request.operation;await assert.rejects(f.intercept.failures, /UI-TRANSPORT/);
  assert.equal(f.sent.length, 0);await f.intercept.stop();
});
