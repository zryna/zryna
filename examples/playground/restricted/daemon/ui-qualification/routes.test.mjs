import assert from 'node:assert/strict';
import test from 'node:test';
import { requestRecord, responseHeaders, routeFor, routes } from './routes.mjs';

const origin = 'http://127.0.0.1:12345'; const capability = 'a'.repeat(64);
test('fixed route mapping rejects targets, escaping, worker execution and method changes', () => {
  assert.equal(routeFor(origin + '/examples/playground/restricted/main.mjs', 'GET', origin), 'asset:main.mjs');
  for (const path of ['/compile?x=1', '/compile#x', '/%63ompile', '/examples/playground/restricted/worker.mjs',
    '/examples/playground/restricted/../restricted/main.mjs', '/unknown']) {
    assert.throws(() => routeFor(origin + path, 'GET', origin), /UI-TRANSPORT/);
  }
  assert.throws(() => routeFor('http://127.0.0.1:12346/session', 'GET', origin), /UI-TRANSPORT/);
  assert.throws(() => routeFor(origin + '/compile', 'GET', origin), /UI-TRANSPORT/);
  assert.ok(Object.isFrozen(routes.compile));
});
test('only captured real browser headers are admitted; absent authentication remains absent', () => {
  const value = { version: 1, type: 'request', id: 1, route: 'session', method: 'GET', headers: [], bodyBase64: '' };
  assert.equal(requestRecord(value, origin, capability).headers.authorization, undefined);
  for (const headers of [[{ name: 'host', value: 'evil' }], [{ name: 'authorization', value: 'Bearer wrong' }],
    [{ name: 'origin', value: 'http://outside.invalid' }], [{ name: 'content-type', value: 'application/json\r\nX: y' }]]) {
    assert.throws(() => requestRecord({ ...value, headers }, origin, capability), /UI-TRANSPORT/);
  }
  assert.throws(() => requestRecord({ ...value, bodyBase64: 'eA==' }, origin, capability), /UI-TRANSPORT/);
  assert.throws(() => requestRecord({ ...value, path: '/other' }, origin, capability), /UI-TRANSPORT/);
});
test('actual response headers must contain every security header with no duplicates or additions', () => {
  const actual = ['Content-Type', 'application/json', 'Cache-Control', 'no-store', 'Content-Security-Policy', "default-src 'none'",
    'X-Content-Type-Options', 'nosniff', 'Referrer-Policy', 'no-referrer', 'Cross-Origin-Opener-Policy', 'same-origin',
    'Cross-Origin-Resource-Policy', 'same-origin', 'Date', 'Thu, 15 Oct 2026 00:00:00 GMT'];
  assert.equal(responseHeaders(actual).length, 7);
  assert.throws(() => responseHeaders(actual.slice(2)), /UI-TRANSPORT/);
  assert.throws(() => responseHeaders([...actual, 'content-type', 'text/plain']), /UI-TRANSPORT/);
  assert.throws(() => responseHeaders([...actual, 'Location', 'http://outside.invalid']), /UI-TRANSPORT/);
});
