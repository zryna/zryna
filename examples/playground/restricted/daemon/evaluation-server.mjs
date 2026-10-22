// A private network-namespace loopback server with a finite, eagerly captured resource map.
import { createServer } from 'node:http';
import { readImageFile } from './evaluation-input.mjs';
import { fail } from '../limits.mjs';

const modules = ['worker', 'protocol', 'limits', 'json', 'report', 'executable'];
const policy = "default-src 'none'; script-src 'self' blob: 'wasm-unsafe-eval'; " +
  "worker-src 'self'; connect-src 'self'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'";

export async function evaluationServer(payload, templateBytes) {
  const resources = new Map([
    ['/', { type: 'text/html; charset=utf-8', bytes: Buffer.from('<!doctype html><title>Isolated scalar evaluation</title>') }],
    ['/payload', { type: 'application/json', bytes: Buffer.from(JSON.stringify(payload)) }],
    ['/template', { type: 'application/octet-stream', bytes: templateBytes }],
  ]);
  for (const name of modules) {
    resources.set(`/${name}.mjs`, { type: 'text/javascript; charset=utf-8',
      bytes: await readImageFile(new URL(`../${name}.mjs`, import.meta.url), 65536) });
  }
  let origin;
  const server = createServer((request, response) => {
    const resource = resources.get(request.url);
    if (request.method !== 'GET' || !resource || request.headers.host !== new URL(origin).host) {
      response.writeHead(404); response.end(); return;
    }
    response.writeHead(200, { 'content-type': resource.type, 'content-length': resource.bytes.length,
      'content-security-policy': policy, 'cache-control': 'no-store',
      'x-content-type-options': 'nosniff', 'referrer-policy': 'no-referrer',
      'cross-origin-opener-policy': 'same-origin', 'cross-origin-resource-policy': 'same-origin' });
    response.end(resource.bytes);
  });
  server.maxConnections = 4;
  server.headersTimeout = 1000;
  server.requestTimeout = 1000;
  server.timeout = 1000;
  server.keepAliveTimeout = 100;
  try {
    await new Promise((resolve, reject) => {
      server.once('error', reject);
      server.listen(0, '127.0.0.1', resolve);
    });
    origin = `http://127.0.0.1:${server.address().port}`;
  } catch { server.closeAllConnections(); server.close(); fail('EVALUATION-LISTENER'); }
  return {
    origin,
    async close() {
      server.closeAllConnections();
      await new Promise((resolve, reject) => server.close(error => error ? reject(error) : resolve()));
    },
  };
}
