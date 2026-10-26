import assert from 'node:assert/strict';
import { join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { auditMaterials } from './materials.mjs';
// Resolution reads package JSON only; no vsce/identity credential constructors execute.
const { requireMsal: require, packageRoot } = auditMaterials();
let number = 0, failed = 0;
async function run(name, action) {
  ++number;
  try { await action(); console.log(`ok ${number} - ${name}`); }
  catch (error) { ++failed; console.log(`not ok ${number} - ${name}\n# ${error.name}: ${error.message.replaceAll('\n', ' ')}`); }
}
console.log('TAP version 13');
await run('deny guard blocks real HTTPS, fetch and socket before SDK calls', () => {
  for (const action of [() => require('node:https').get('https://not-contacted.invalid'),
    () => fetch('https://not-contacted.invalid'), () => require('node:net').connect(1, 'not-contacted.invalid')])
    assert.throws(action, /M418_DENIED_/);
  assert.equal(globalThis.__m418Denied.length, 3); globalThis.__m418Denied.length = 0;
});
for (const [mode, sdk] of [['cjs', require('@azure/msal-node')],
  ['esm', await import(pathToFileURL(join(packageRoot, 'dist/index.mjs')).href)]]) {
  const Cca = sdk.ConfidentialClientApplication;
  const base = Object.getPrototypeOf(Cca.prototype);
  const config = () => ({ auth: { clientId: '11111111-1111-1111-1111-111111111111',
    authority: 'https://login.microsoftonline.com/22222222-2222-2222-2222-222222222222', clientSecret: 'synthetic-only-secret' } });
  await run(`${mode} upstream instance-aware constructor rejects`, () => {
    const cfg = config(); cfg.auth.instanceAware = true;
    assert.throws(() => new Cca(cfg), error => error.errorCode === 'unsupported_cca_instance_aware');
  });
  await run(`${mode} upstream payload strips routing and attacker nonce before delegation`, async () => {
    const original = base.acquireTokenByCode;
    const request = { scopes: ['https://graph.microsoft.com/.default'], redirectUri: 'http://localhost', code: 'request-code', nonce: 'request-nonce' };
    const payload = { code: 'attacker-code', state: 'returned-state', nonce: 'attacker-nonce',
      cloud_instance_host_name: 'not-contacted.invalid', cloud_instance_name: 'attacker-cloud',
      cloud_graph_host_name: 'not-contacted.invalid', msgraph_host: 'not-contacted.invalid', client_info: 'untrusted-info' };
    let seen;
    base.acquireTokenByCode = async (...args) => { seen = args; return { marker: 'synthetic-delegation' }; };
    try { await new Cca(config()).acquireTokenByCode(request, payload);
      assert.equal(seen[0], request); assert.deepEqual(seen[1], { code: 'request-code', state: 'returned-state', nonce: 'request-nonce' });
    } finally { base.acquireTokenByCode = original; }
  });
  await run(`${mode} upstream instance_aware true false empty reject without delegation`, async () => {
    const original = base.getAuthCodeUrl; let calls = 0;
    base.getAuthCodeUrl = async () => { ++calls; return 'synthetic-parent-url'; };
    try { for (const value of ['true', 'false', '']) {
      await assert.rejects(new Cca(config()).getAuthCodeUrl({ scopes: ['scope'], extraQueryParameters: { instance_aware: value } }),
        error => error.errorCode === 'unsupported_cca_instance_aware');
    } assert.equal(calls, 0); } finally { base.getAuthCodeUrl = original; }
  });
  await run(`${mode} ordinary no-payload authCode preserves request and delegation`, async () => {
    const original = base.acquireTokenByCode; const request = { scopes: ['scope'], code: 'request-code' }; let calls = 0;
    base.acquireTokenByCode = async (actual, payload) => { ++calls; assert.equal(actual, request); assert.equal(payload, undefined); return { token: 'synthetic' }; };
    try { assert.deepEqual(await new Cca(config()).acquireTokenByCode(request), { token: 'synthetic' }); assert.equal(calls, 1); }
    finally { base.acquireTokenByCode = original; }
  });
  await run(`${mode} genuine base state mismatch remains rejected before network`, async () => {
    await assert.rejects(new Cca(config()).acquireTokenByCode({ scopes: ['scope'], code: 'request-code', state: 'expected-state' },
      { code: 'attacker-code', state: 'different-returned-state' }), error => error.errorCode === 'state_mismatch');
  });
}
assert.equal(globalThis.__m418Denied.length, 0, 'no SDK network/auth subprocess was attempted');
console.log(`1..${number}\n# pass ${number - failed}\n# fail ${failed}`); process.exitCode = failed ? 1 : 0;
