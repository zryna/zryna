// Pure Request-object fixtures. Actual HTTP/CDP/consumer behavior needs the separate operator cohort.
import test from 'node:test';import assert from 'node:assert/strict';
import { createCompletionLedger } from './completion-ledger.mjs';
import { validateCompletionFailure, validateCompletionLedger } from './completion-record.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';
import { sha256 } from './wire.mjs';
const origin = 'http://127.0.0.1:12345';
function fixture() {
  const ledger = createCompletionLedger(origin);let id = 10, unique = 0;
  function api(route, { localFirst = false, terminal = 'FINISHED', write = true, body = undefined } = {}) {
    body ??= route === 'session' ? Buffer.alloc(0) : Buffer.from(JSON.stringify({ fixture: ++unique }));
    const request = { url: () => origin + '/' + route, method: () => route === 'session' ? 'GET' : 'POST',
      resourceType: () => 'fetch', postDataBuffer: () => route === 'session' ? null : body,
      redirectedFrom: () => null, redirectedTo: () => null };
    const job = { id: ++id, name: route };
    if (localFirst) ledger.localBody(job, request.method(), body);
    ledger.browserStart(request);if (!localFirst) ledger.localBody(job, request.method(), body);
    if (route === 'compile' && terminal === 'INITIAL') {
      ledger.browserTerminal(request, 'ERR_ABORTED');return { request, job, body };
    }
    ledger.browserResponse({ request: () => request, status: () => 200 });
    ledger.localReply(job.id, 200, Buffer.from('actual fixture response'));
    if (write) ledger.localWriteComplete(job.id);
    if (terminal) ledger.browserTerminal(request, terminal);
    return { request, job, body };
  }
  function all(options = {}) {
    let fence = ledger.begin('STARTUP');api('session', options);ledger.publish(fence);
    fence = ledger.begin('INITIAL_ABORT');const held = api('compile', { terminal: 'INITIAL' });ledger.publish(fence);
    ledger.proveInitialAbort({ id: held.job.id, paused: true, acknowledged: true, httpResponseClosed: true,
      event: 'requestfailed', requestBytes: held.body.length, requestSha256: sha256(held.body) });
    fence = ledger.begin('INITIAL_CANCEL');api('cancel', options);ledger.publish(fence);
    let scalar = 0;
    for (const [index, row] of qualificationCases.entries()) {
      fence = ledger.begin('COMPILE', index);api('compile', options);api('finish', options);ledger.publish(fence);
      for (const _ of scalarCases(row)) {
        fence = ledger.begin('EVALUATE', index, scalar++);api('evaluate', options);api('finish', options);ledger.publish(fence);
      }
    }
  }
  return { ledger, api, all };
}
test('either arrival order joins45 unique objects/ids and21 ownfinish with reconciled provisional anchors', () => {
  for (const localFirst of [true, false]) {
    const f = fixture();f.all({ localFirst, terminal: 'ERR_ABORTED' });const value = f.ledger.seal(true);
    assert.equal(value.rows.length, 45);assert.equal(value.rows.filter(row => row.finishId !== null).length, 21);
    assert.equal(value.firstProvisional.id, value.rows[0].id);assert.equal(value.firstProvisional.ordinal, 1);
    assert.notEqual(value.rows[0].ordinal, value.rows[0].id);validateCompletionLedger(value);
    assert.throws(() => f.ledger.begin('STARTUP'));assert.throws(() => f.ledger.seal(true));
  }
});
test('unmatched event anchor remainshistorical but finalfailure reconciles ONLY that exact object join', () => {
  const f = fixture(), fence = f.ledger.begin('STARTUP');
  const request = { url: () => origin + '/session', method: () => 'GET', resourceType: () => 'fetch',
    postDataBuffer: () => null, redirectedFrom: () => null, redirectedTo: () => null };
  f.ledger.browserStart(request);f.ledger.browserResponse({ request: () => request, status: () => 200 });
  f.ledger.browserTerminal(request, 'ERR_ABORTED');const before = f.ledger.failure();assert.equal(before.firstProvisional.id, null);
  f.ledger.localBody({ id: 17, name: 'session' }, 'GET', Buffer.alloc(0));f.ledger.localReply(17, 200, Buffer.from('body'));f.ledger.localWriteComplete(17);
  f.ledger.publish(fence);const after = f.ledger.failure();assert.equal(after.firstProvisional.id, 17);
  assert.equal(before.firstProvisional.id, null);assert.equal(before.firstProvisional.sequence, after.firstProvisional.sequence);
});
test('repeated tuple/object/localid reject forever even afterpublication', () => {
  for (const mode of ['object', 'tuple', 'local']) {
    const f = fixture();const fence = f.ledger.begin('STARTUP');const row = f.api('session');f.ledger.publish(fence);
    f.ledger.begin('INITIAL_CANCEL');
    if (mode === 'object') assert.throws(() => f.ledger.browserStart(row.request));
    else if (mode === 'local') assert.throws(() => f.ledger.localBody({ id: row.job.id, name: 'cancel' }, 'POST', Buffer.from('different')));
    else assert.throws(() => f.api('session'));
  }
});
test('POSTmissingbody, redirects, wrong resource and bodycap refuse before joining', () => {
  for (const change of [r => { r.postDataBuffer = () => null; }, r => { r.redirectedFrom = () => ({}); },
    r => { r.redirectedTo = () => ({}); }, r => { r.resourceType = () => 'document'; }, r => { r.postDataBuffer = () => Buffer.alloc(1025); }]) {
    const f = fixture();f.ledger.begin('INITIAL_CANCEL');const r = { url: () => origin + '/cancel', method: () => 'POST', resourceType: () => 'fetch',
      postDataBuffer: () => Buffer.from('{}'), redirectedFrom: () => null, redirectedTo: () => null };change(r);assert.throws(() => f.ledger.browserStart(r));
  }
});
test('candidate needs its same Request200 already observed; unknownobject cannot become candidate', () => {
  const f = fixture();f.ledger.begin('STARTUP');const r = { url: () => origin + '/session', method: () => 'GET', resourceType: () => 'fetch',
    postDataBuffer: () => null, redirectedFrom: () => null, redirectedTo: () => null };
  f.ledger.browserStart(r);assert.equal(f.ledger.browserTerminal(r, 'ERR_ABORTED'), false);assert.equal(f.ledger.browserTerminal({}, 'ERR_ABORTED'), false);
  assert.equal(f.ledger.failure().firstProvisional, null);
});
test('fencewrongorder, ambiguity, overlap, invalidinput request and stalehandle fail immediately', () => {
  const f = fixture();assert.throws(() => f.api('session'));const fence = f.ledger.begin('STARTUP');assert.throws(() => f.ledger.begin('COMPILE', 0));
  f.api('cancel');assert.throws(() => f.ledger.publish(fence));
  const g = fixture();const a = g.ledger.begin('STARTUP');g.api('session');g.ledger.publish(a);assert.throws(() => g.ledger.publish(a));
  assert.throws(() => g.api('evaluate'));
});
test('missing terminal/write and unsettledoriginaloperations FAIL synchronously withoutdrain', () => {
  for (const options of [{ terminal: null }, { write: false }, {}]) {
    const f = fixture();f.all(options);assert.throws(() => f.ledger.seal(Object.keys(options).length > 0));
    const partial = f.ledger.failure();assert.equal(partial.sealed, false);validateCompletionFailure(partial);
  }
});
test('originalabort exact two-signal proof cannot borrowanother request or skip its proof', () => {
  const f = fixture(), fence = f.ledger.begin('INITIAL_ABORT'), held = f.api('compile', { terminal: 'INITIAL' });f.ledger.publish(fence);
  assert.throws(() => f.ledger.proveInitialAbort({ id: held.job.id + 1 }));assert.throws(() => f.ledger.seal(true));
});
test('failureprivacy/caps reject firstextra and maximumarrays fit4096bytes', () => {
  const f = fixture();f.all({ terminal: 'ERR_ABORTED' });const partial = f.ledger.failure();assert.equal(JSON.stringify(partial).includes('fixture response'), false);
  partial.unresolvedOrdinals = Array.from({ length: 256 }, (_, i) => i + 1);partial.unmatchedLocalIds = [...partial.unresolvedOrdinals];
  partial.rowCount = 256;partial.candidateCount = 256;validateCompletionFailure(partial);assert.ok(Buffer.byteLength(JSON.stringify(partial)) <= 4096);
  assert.throws(() => validateCompletionFailure({ ...partial, raw: 'token' }));assert.throws(() => validateCompletionFailure({ ...partial, rowCount: 257 }));
  assert.throws(() => validateCompletionFailure({ ...partial, fullLedgerBytes: 2097153 }));
});

test('earliest unresolved follows actual terminal sequence rather than Request start ordinal', () => {
  const f = fixture();f.ledger.begin('COMPILE', 0);
  const first = f.api('compile', { terminal: null }), second = f.api('finish', { terminal: null });
  f.ledger.browserTerminal(second.request, 'ERR_ABORTED');f.ledger.browserTerminal(first.request, 'ERR_ABORTED');
  const value = f.ledger.failure();assert.equal(value.firstProvisional.ordinal, 2);assert.equal(value.earliestUnresolved.ordinal, 2);
  assert.throws(() => validateCompletionFailure({ ...value, firstProvisional: null }));
  assert.throws(() => validateCompletionFailure({ ...value, unresolvedOrdinals: [3] }));
});
test('256 entry histories are bounded and the first extra Request or localid fails synchronously', () => {
  const f = fixture(), g = fixture();f.ledger.begin('INITIAL_CANCEL');g.ledger.begin('INITIAL_CANCEL');
  const request = index => ({url:()=>origin+'/cancel',method:()=> 'POST',resourceType:()=> 'fetch',
    postDataBuffer:()=>Buffer.from(String(index)),redirectedFrom:()=>null,redirectedTo:()=>null});
  for (let index = 1; index <= 256; index++) {
    f.ledger.browserStart(request(index));g.ledger.localBody({id:index,name:'cancel'},'POST',Buffer.from(String(index)));
  }
  assert.throws(()=>f.ledger.browserStart(request(257)));assert.throws(()=>g.ledger.localBody({id:257,name:'cancel'},'POST',Buffer.from('257')));
});
test('worst permitted45-row values remaininside49152bytes and existing canonicalframe decoder', async () => {
  const {decode}=await import('./wire.mjs');const f=fixture();f.all({terminal:'ERR_ABORTED'});const value=f.ledger.seal(true);
  const inputs={session:0,compile:32768,evaluate:8192,finish:1024,cancel:1024}, outputs={session:98304,compile:1277956,evaluate:2048,finish:1024,cancel:1024};
  for(const row of value.rows) {row.requestBytes=inputs[row.route];if(row.response)row.response.bytes=outputs[row.route];}
  // Request hashes stay unique; only bounded numeric field widths change here.
  validateCompletionLedger(value);const bytes=Buffer.from(JSON.stringify(value));assert.ok(bytes.length<=49152);assert.deepEqual(decode(bytes),value);
  assert.throws(()=>validateCompletionLedger({...value,failureEvidenceTruncated:true}));
});
test('request body is read once and the1024-event observation limit rejects the firstextra', () => {
  const f=fixture();f.ledger.begin('INITIAL_CANCEL');let reads=0;
  const request={url:()=>origin+'/cancel',method:()=> 'POST',resourceType:()=> 'fetch',postDataBuffer:()=>{reads++;return Buffer.from('{}');},redirectedFrom:()=>null,redirectedTo:()=>null};
  f.ledger.browserStart(request);assert.equal(reads,1);assert.throws(()=>f.ledger.browserStart(request));assert.equal(reads,1);
  const g=fixture();g.ledger.begin('STARTUP');
  for(let i=0;i<1023;i++)g.ledger.browserStart({url:()=>origin+'/examples/playground/restricted/style.css',method:()=> 'GET'});
  assert.throws(()=>g.ledger.browserStart({url:()=>origin+'/examples/playground/restricted/style.css',method:()=> 'GET'}));
});
