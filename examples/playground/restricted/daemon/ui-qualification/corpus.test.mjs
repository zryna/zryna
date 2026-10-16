// Pure validator/decoder fixtures. These synthetic frames establish no compiler or browser acceptance.
import assert from 'node:assert/strict';import test from 'node:test';
import { corpus } from '../../corpus.mjs';
import { bindingRevision, world } from '../../limits.mjs';
import { browserCorpus, qualificationCases, scalarCases } from './browser-oracles.mjs';
import { createCorpusTrace } from './corpus-trace.mjs';
import { inspectCorpus, diagnosticChildren } from './corpus-observation.mjs';
import { sha256 } from './wire.mjs';

const json = value => Buffer.from(JSON.stringify(value));
const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64), policySha256: 'c'.repeat(64), bindingTemplateSha256: sha256(Buffer.from('inert')) };
const token = 'e'.repeat(64), witSha256 = 'd'.repeat(64);
const observe = (trace, name, request, body, headers = []) => trace.observe({ name, request: json(request), body: json(body), headers, status: 200 });
async function begin() {
  const trace = createCorpusTrace();await observe(trace, 'session', null, { version: 2, evaluationMode: 'supervised-host', authority, witSha256, template: Buffer.from('inert').toString('base64') });return trace;
}
function compilation(row, revision) {
  const success = !!row.cases, component = Buffer.from([0,97,115,109,13,0,1,0]);
  const artifacts = success ? [component, Buffer.from('inert'), Buffer.from('inert')] : [];
  const bytes = Buffer.from(row.source), start = bytes.indexOf('absent');
  const prefix = start < 0 ? '' : bytes.subarray(0,start).toString();
  const line = prefix.split('\n').length, column = [...prefix.split('\n').at(-1)].length + 1;
  const diagnostics = success ? [] : [{ code: row.code ?? 'TS2304', severity: 'error', path: start < 0 ? null : 'src/main.zry',
    byte_start: start < 0 ? null : start, byte_end: start < 0 ? null : start + 6, line_start: start < 0 ? null : line,
    line_end: start < 0 ? null : line, column_start: start < 0 ? null : column, column_end: start < 0 ? null : column + 6,
    message: 'inert synthetic diagnostic <img src=x>', guidance: '' }];
  const exports = [...new Map(scalarCases(row).map(value => [value.logical, value])).values()].map(value => ({
    logical: value.logical, component: 'zryna-export-' + Buffer.from(value.logical).toString('hex'), core: value.logical, arity: value.args.length }));
  const metadata = { format: 'zryna.playground-compilation.v1', version: 1, revision, sourcePath: 'src/main.zry',
    sourceSha256: sha256(bytes), sourceBytes: bytes.length, profile: 'browser-component-v1', status: success ? 'compiled' : 'rejected',
    report: { schema_version: 1, diagnostics }, failure: null, exports,
    identity: success ? { revision: bindingRevision, world, witSha256, componentSha256: sha256(component),
      coreSha256: sha256(component), interfaceSha256: 'f'.repeat(64), coreOffset: 0, coreBytes: component.length } : null,
    artifacts: artifacts.map((data,index) => ({ role: ['component','loader','declarations'][index], bytes: data.length, sha256: sha256(data) })) };
  const header = json(metadata), size = Buffer.alloc(4);size.writeUInt32LE(header.length);
  const body = Buffer.concat([size, header, ...artifacts]);
  const receipt = { revision, sourceSha256: metadata.sourceSha256, frameSha256: sha256(body),
    compilerSha256: authority.compilerSha256, toolkitSha256: authority.toolkitSha256, policySha256: authority.policySha256 };
  return { metadata, body, headers: [{ name: 'x-zryna-job', value: token }, { name: 'x-zryna-receipt', value: json(receipt).toString('base64') }] };
}
async function capture(trace, row, revision, mutate = value => value) {
  const frame = mutate(compilation(row,revision));
  await trace.observe({ name: 'compile', request: json({version:1,revision,source:row.source}), ...frame, status: 200 });return frame;
}
const finish = trace => observe(trace, 'finish', { token, cancelled: false }, { finished: true });
async function completeFixture() {
  const trace = await begin();
  for (const [index,row] of qualificationCases.entries()) {
    const {metadata} = await capture(trace,row,index+2);await finish(trace);
    for (const value of scalarCases(row)) {
      const input = { revision: metadata.revision, sourceSha256: metadata.sourceSha256, componentSha256: metadata.identity.componentSha256, logical: value.logical, args: value.args };
      await observe(trace, 'evaluate', input, { token, observation: { revision: input.revision, logical: input.logical, componentSha256: input.componentSha256, value: value.value } });await finish(trace);
    }
  }
  return trace.complete();
}
function domFixture(actual) {
  const property = (name,value) => ({ name,value });
  const source = {role:'textbox',name:'Source',ignored:false,properties:[property('focused',true)]};
  const status = {role:'status',name:'',ignored:false,properties:[property('live','polite')]};
  const argument = {role:'textbox',name:'Argument 1 (i32)',ignored:false,properties:[property('focused',true),property('invalid','true')]};
  const unicode=qualificationCases.findIndex(row=>row.id==='unicode-navigation');
  const record=actual.compilations[unicode].report.diagnostics[0], bytes=Buffer.from(qualificationCases[unicode].source);
  return { format:'zryna.ui-corpus-observation.v1',version:1,rows:qualificationCases.map((row,index)=>{
    const compiled=actual.compilations[index];return {id:row.id,source:row.source,status:row.cases?'Compiled. Choose an export and run it.':'Compilation rejected. Review the compiler diagnostics.',
      identity:row.cases?`Source revision ${compiled.revision}; source SHA-256 ${compiled.sourceSha256}; component SHA-256 ${compiled.identity.componentSha256}`:'',runDisabled:!row.cases,
      exports:compiled.exports.map(v=>({value:v.logical,text:`${v.logical} (${v.arity} arguments)`})),diagnostics:compiled.report.diagnostics.map(diagnosticChildren),unexpectedDiagnosticElements:0,
      observations:scalarCases(row).map(v=>({...v,text:`${v.logical}(${v.args.join(', ')}) = ${v.value} (signed i32)`,identity:`Source revision ${compiled.revision}; source SHA-256 ${compiled.sourceSha256}; observed component SHA-256 ${compiled.identity.componentSha256}`}))};}),
    keyboard:{tabFromSource:'compile'},invalidArgument:{focusedId:'argument-0',invalid:'true',error:'Enter a whole decimal number from -2147483648 to 2147483647.',observation:'Check the highlighted argument values before running.'},
    revisionInvalidation:{runDisabled:true,exports:0,identity:'',observation:'No observation.'},sourceBudget:{bytes:'4098 / 4096 UTF-8 bytes',invalid:'true',error:'Source exceeds 4096 UTF-8 bytes. Remove 2 bytes before compiling.',compileDisabled:true,runDisabled:true},
    recovered:{invalid:'false',error:'',compileDisabled:false,runDisabled:true,identity:''},unicodeNavigation:{diagnosticIndex:0,focusedId:'source',start:bytes.subarray(0,record.byte_start).toString().length,end:bytes.subarray(0,record.byte_end).toString().length},
    argumentAX:[argument,{...argument,name:'Argument 2 (i32)'}],invalidAX:[argument],diagnosticAX:[source,status],
    inertSource:{sourceExact:true,imgElements:0,scriptElementsAfter:1,markerAbsent:true,scriptElementsBefore:1},
    sessionIsolation:'unrun',arbitraryInertMarkup:'unrun',audibleAssistiveTechnology:'unrun'};
}
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
