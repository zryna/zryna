import assert from 'node:assert/strict';
import { request } from 'node:http';
import { createService } from '../../examples/playground/restricted/service.mjs';
import { sha256, utf8 } from '../../examples/playground/restricted/limits.mjs';

// Synthetic frames and host doubles exercise HTTP transport only; nothing is executed.
const source = 'export function value(): i32 { return 42; }';
const sourceRequest = { version: 1, revision: 7, source };
const witSha256 = 'd'.repeat(64);
const templateBytes = utf8('transport-only template; never imported');

function deferred() {
  let resolve; let reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}

async function frame(input, changes = {}) {
  const contents = [new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]),
    utf8('transport-only loader'), utf8('transport-only declarations')];
  const hashes = await Promise.all(contents.map(sha256));
  const metadata = { format: 'zryna.playground-compilation.v1', version: 1,
    revision: input.revision, sourcePath: 'src/main.zry', sourceSha256: await sha256(utf8(input.source)),
    sourceBytes: utf8(input.source).length, profile: 'browser-component-v1', status: 'compiled',
    report: { schema_version: 1, diagnostics: [] }, failure: null,
    identity: { revision: 'zryna.browser-component-bindings.v1',
      world: 'zryna:capability-profiles/browser@0.1.0', witSha256, componentSha256: hashes[0],
      coreSha256: hashes[0], interfaceSha256: 'e'.repeat(64), coreOffset: 0, coreBytes: 8 },
    exports: [{ logical: 'value', component: 'zryna-export-76616c7565', core: 'value', arity: 0 }],
    artifacts: ['component', 'loader', 'declarations'].map((role, index) => ({ role,
      bytes: contents[index].length, sha256: hashes[index] })), ...changes };
  const header = utf8(JSON.stringify(metadata));
  const bytes = new Uint8Array(4 + header.length + contents.reduce((sum, value) => sum + value.length, 0));
  new DataView(bytes.buffer).setUint32(0, header.length, true);
  bytes.set(header, 4);
  let offset = 4 + header.length;
  for (const content of contents) { bytes.set(content, offset); offset += content.length; }
  return { bytes, metadata };
}

async function fixture(t, overrides = {}) {
  const calls = { compile: [], evaluate: [], finish: [] };
  const host = {
    async compile(bytes, signal, token) {
      calls.compile.push({ bytes, signal, token });
      return (await frame(JSON.parse(new TextDecoder().decode(bytes)))).bytes;
    },
    async beginEvaluation(token, input) { calls.evaluate.push({ token, input }); },
    async finish(token, cancelled) { calls.finish.push({ token, cancelled }); },
    ...overrides,
  };
  const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64),
    policySha256: 'c'.repeat(64), bindingTemplateSha256: await sha256(templateBytes) };
  const service = createService({ host, authority, witSha256, templateBytes,
    resources: new Map([['/examples/playground/restricted/', { bytes: utf8('<p>Transport fixture</p>'),
      type: 'text/html; charset=utf-8' }]]) });
  const location = await service.listen();
  t.after(() => service.close());
  const capability = new URL(location.url).hash.slice(1);
  const auth = { authorization: `Bearer ${capability}`, 'sec-fetch-site': 'same-origin',
    origin: location.origin, 'content-type': 'application/json' };
  const send = (path, value = null, options = {}) => new Promise((resolve, reject) => {
    const body = options.raw ?? (value === null ? undefined : JSON.stringify(value));
    const headers = { ...auth, ...(body === undefined ? {} : { 'content-length': Buffer.byteLength(body) }),
      ...options.headers };
    for (const [key, value] of Object.entries(headers)) if (value === null) delete headers[key];
    const outgoing = request(`${location.origin}${path}`, { method: options.method ??
      (body === undefined ? 'GET' : 'POST'), headers, agent: false }, response => {
      const chunks = [];
      response.on('data', bytes => chunks.push(bytes));
      response.on('error', reject);
      response.on('end', () => resolve({ status: response.statusCode, headers: response.headers,
        bytes: new Uint8Array(Buffer.concat(chunks)),
        json() { return JSON.parse(Buffer.concat(chunks).toString('utf8')); } }));
    });
    outgoing.setTimeout(4000, () => outgoing.destroy(new Error('Test transport deadline')));
    outgoing.on('error', reject);
    outgoing.end(body);
  });
  return { calls, host, authority, service, send, auth, ...location };
}

async function expectError(reply, code, status = 400) {
  assert.equal(reply.status, status);
  assert.deepEqual(reply.json(), { policyError: code });
}

async function compile(f) {
  const reply = await f.send('/compile', sourceRequest);
  assert.equal(reply.status, 200);
  const token = reply.headers['x-zryna-job'];
  assert.match(token, /^[a-f0-9]{64}$/);
  return { reply, token };
}

async function startUpload(t, f, path, value) {
  const entered = deferred();
  f.service.server.once('request', () => entered.resolve());
  const bytes = JSON.stringify(value);
  let outgoing;
  const pending = new Promise((resolve, reject) => {
    outgoing = request(`${f.origin}${path}`, { method: 'POST', agent: false,
      headers: { ...f.auth, 'content-length': Buffer.byteLength(bytes) } }, response => {
      const chunks = [];
      response.on('data', chunk => chunks.push(chunk));
      response.on('end', () => resolve({ status: response.statusCode,
        json: () => JSON.parse(Buffer.concat(chunks).toString('utf8')) }));
    });
    outgoing.on('error', reject);
    outgoing.write(bytes.slice(0, 10));
  });
  void pending.catch(() => {});
  t.after(() => outgoing.destroy());
  await entered.promise;
  return { pending, complete: () => outgoing.end(bytes.slice(10)) };
}

export { source, sourceRequest, witSha256, templateBytes, deferred, frame, fixture,
  expectError, compile, startUpload };
