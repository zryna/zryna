// Pure validator/decoder fixtures. These synthetic frames establish no compiler or browser acceptance.
import assert from 'node:assert/strict';import test from 'node:test';
import { corpus } from '../../corpus.mjs';
import { browserCorpus, qualificationCases, scalarCases } from './browser-oracles.mjs';
import { inspectCorpus } from './corpus-observation.mjs';
import { createCorpusTrace } from './corpus-trace.mjs';
import { sha256 } from './wire.mjs';

import { authority, token, witSha256, begin, observe, compilation, capture, finish, completeFixture, domFixture } from './corpus-fixture.mjs';
test('fixed browser corpus equals every original row and retains all ten independent scalar oracles',()=>{
  assert.deepEqual(browserCorpus,corpus);assert.equal(qualificationCases.flatMap(scalarCases).length,10);
});
test('actual-byte decoder/receipt boundary rejects foreign source, frame mutation, authority replacement and missing cleanup',async()=>{
  const wrong=await begin();await assert.rejects(capture(wrong,{...qualificationCases[0],source:'foreign source'},2));
  for(const mutate of [f=>({...f,body:Buffer.concat([f.body,Buffer.from('x')])}),f=>({...f,headers:f.headers.map(v=>v.name==='x-zryna-receipt'?{...v,value:json({}).toString('base64')}:v)}),f=>({...f,headers:[]}),
    ()=>compilation({...qualificationCases[0],extra:{export:'unexpected',args:[],value:0}},2)]) {
    await assert.rejects(capture(await begin(),qualificationCases[0],2,mutate));
  }
  const active=await begin();await capture(active,qualificationCases[0],2);assert.throws(()=>active.complete());
  await assert.rejects(observe(active,'finish',{token:'a'.repeat(64),cancelled:false},{finished:true}));
});
test('authenticated selection rejects wrong scalar and the next source before all original oracles',async()=>{
  const trace=await begin(),{metadata}=await capture(trace,qualificationCases[0],2);await finish(trace);
  await assert.rejects(capture(trace,qualificationCases[1],3));
  const input={revision:2,sourceSha256:metadata.sourceSha256,componentSha256:metadata.identity.componentSha256,logical:'add',args:[20,22]};
  await assert.rejects(observe(trace,'evaluate',input,{token,observation:{revision:2,logical:'add',componentSha256:input.componentSha256,value:41}}));
});
test('DOM projection must match authenticated reports, Unicode byte-to-UTF16 selection and AX focus',async()=>{
  const actual=await completeFixture(),dom=domFixture(actual);assert.deepEqual(actual.authority,authority);assert.equal(actual.witSha256,witSha256);assert.deepEqual(inspectCorpus(dom,actual),dom);
  for(const mutate of [d=>{d.rows[4].diagnostics[0][0].tag='IMG';},d=>{d.rows[0].observations[0].value=41;},d=>{d.unicodeNavigation.start=actual.compilations[9].report.diagnostics[0].byte_start;},d=>{d.invalidAX[0].properties=[];},d=>{d.inertSource.markerAbsent=false;},d=>{d.inertSource.scriptElementsAfter=2;},d=>{d.rows[0].identity='';},d=>{d.sessionIsolation='passed';},d=>{d.raw='secret';}]){
    const changed=structuredClone(dom);mutate(changed);assert.throws(()=>inspectCorpus(changed,actual));
  }
  assert.ok(!JSON.stringify(actual).includes(token));assert.ok(!JSON.stringify(actual).includes('Bearer'));
});
test('trace owns exact broker IDs and exports21 finish links without live job tokens', async () => {
  const actual = await completeFixture();
  const operations = [...actual.compilations, ...actual.evaluations];
  assert.equal(new Set(operations.map(row => row.requestId)).size, 21);
  assert.equal(new Set(operations.map(row => row.finishId)).size, 21);
  assert.ok(operations.every(row => row.requestId < row.finishId));
  assert.equal(actual.sessionRequestId, 1);assert.equal(actual.cancelRequestId, 3);
  const serialized = JSON.stringify(actual);
  for (const row of actual.compilations) {
    const frame = compilation(qualificationCases.find(value => value.id === row.id), row.revision);
    assert.ok(!serialized.includes(frame.headers.find(value => value.name === 'x-zryna-job').value));
  }
  for (const row of actual.evaluations) assert.ok(!serialized.includes(sha256(Buffer.from(
    'synthetic-evaluation-' + row.revision + '-' + row.logical + '-' + row.args.join(',')))));
  const trace = createCorpusTrace();
  const record = { id: 1, name: 'cancel', request: Buffer.from('{"cancelled":true}'),
    body: Buffer.from('{"finished":true}'), headers: [], status: 200 };
  await assert.rejects(trace.observe({ ...record, id: undefined }));
  await assert.rejects(trace.observe(record), 'cancel before authenticated session refuses');
  await assert.rejects(trace.observe(record), 'same observed id cannot be reused after refusal');
});
test('trace refuses expired acknowledgements, reused accepted IDs and invalid id carriers', async () => {
  const trace = await begin(), frame = await capture(trace, qualificationCases[0], 2);
  const ownToken = frame.headers.find(row => row.name === 'x-zryna-job').value;
  await assert.rejects(observe(trace, 'finish', { token: ownToken, cancelled: false }, { finished: true, expired: true }));
  assert.equal(trace.snapshot().cleanupAcknowledged, false);
  for (const id of [1, 0, true, 257]) await assert.rejects(trace.observe({ id, name: 'finish',
    request: Buffer.from(JSON.stringify({ token: ownToken, cancelled: false })), body: Buffer.from('{"finished":true}'), status: 200 }));
});
