// Synthetic owning-decoder/DOM fixtures only; no compiler/browser acceptance.
import { bindingRevision, world } from '../../limits.mjs';
import { qualificationCases, scalarCases } from './browser-oracles.mjs';
import { createCorpusTrace } from './corpus-trace.mjs';
import { diagnosticChildren } from './corpus-observation.mjs';
import { sha256 } from './wire.mjs';
export const json = value => Buffer.from(JSON.stringify(value));
export const authority = { compilerSha256: 'a'.repeat(64), toolkitSha256: 'b'.repeat(64), policySha256: 'c'.repeat(64), bindingTemplateSha256: sha256(Buffer.from('inert')) };
export const token = 'e'.repeat(64), witSha256 = 'd'.repeat(64);
const sequences = new WeakMap(), activeTokens = new WeakMap();
export function nextId(trace) { const id = (sequences.get(trace) ?? 0) + 1; sequences.set(trace, id); return id; }
export async function observe(trace, name, request, body, headers = []) {
  await trace.observe({ id: nextId(trace), name, request: json(request), body: json(body), headers, status: 200 });
  if (name === 'evaluate') activeTokens.set(trace, body.token);
}
export async function begin() {
  const trace = createCorpusTrace();await observe(trace, 'session', null, { version: 2, evaluationMode: 'supervised-host', authority, witSha256, template: Buffer.from('inert').toString('base64') });return trace;
}
export function compilation(row, revision) {
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
  exports.sort((left, right) => row.source.indexOf('function ' + left.logical + '(') - row.source.indexOf('function ' + right.logical + '('));
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
  return { metadata, body, headers: [{ name: 'x-zryna-job', value: sha256(Buffer.from('synthetic-compile-' + revision)) }, { name: 'x-zryna-receipt', value: json(receipt).toString('base64') }] };
}
export async function capture(trace, row, revision, mutate = value => value) {
  const frame = mutate(compilation(row,revision));
  await trace.observe({ id: nextId(trace), name: 'compile', request: json({version:1,revision,source:row.source}), ...frame, status: 200 });activeTokens.set(trace, frame.headers.find(row => row.name === 'x-zryna-job').value);return frame;
}
export const finish = trace => observe(trace, 'finish', { token: activeTokens.get(trace), cancelled: false }, { finished: true });
export async function completeFixture() {
  const trace = await begin();nextId(trace);await observe(trace, 'cancel', { cancelled: true }, { finished: true });
  for (const [index,row] of qualificationCases.entries()) {
    const {metadata} = await capture(trace,row,index+2);await finish(trace);
    for (const value of scalarCases(row)) {
      const input = { revision: metadata.revision, sourceSha256: metadata.sourceSha256, componentSha256: metadata.identity.componentSha256, logical: value.logical, args: value.args };
      await observe(trace, 'evaluate', input, { token: sha256(Buffer.from('synthetic-evaluation-' + input.revision + '-' + input.logical + '-' + input.args.join(','))), observation: { revision: input.revision, logical: input.logical, componentSha256: input.componentSha256, value: value.value } });await finish(trace);
    }
  }
  return trace.complete();
}
export function domFixture(actual) {
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
      controls:{compileDisabled:false,cancelDisabled:true,selectedExport:(compiled.exports.find(entry=>entry.logical===row.export)??compiled.exports[0])?.logical??''},
      observations:scalarCases(row).map(v=>({...v,text:`${v.logical}(${v.args.join(', ')}) = ${v.value} (signed i32)`,identity:`Source revision ${compiled.revision}; source SHA-256 ${compiled.sourceSha256}; observed component SHA-256 ${compiled.identity.componentSha256}`,
        selection:{source:row.source,logical:v.logical,args:v.args.map(String),status:'Evaluation completed.',compileDisabled:false,cancelDisabled:true,runDisabled:false}}))};}),
    keyboard:{tabFromSource:'compile'},invalidArgument:{focusedId:'argument-0',invalid:'true',error:'Enter a whole decimal number from -2147483648 to 2147483647.',observation:'Check the highlighted argument values before running.'},
    revisionInvalidation:{runDisabled:true,exports:0,identity:'',observation:'No observation.'},sourceBudget:{bytes:'4098 / 4096 UTF-8 bytes',invalid:'true',error:'Source exceeds 4096 UTF-8 bytes. Remove 2 bytes before compiling.',compileDisabled:true,runDisabled:true},
    recovered:{invalid:'false',error:'',compileDisabled:false,runDisabled:true,identity:''},unicodeNavigation:{diagnosticIndex:0,focusedId:'source',start:bytes.subarray(0,record.byte_start).toString().length,end:bytes.subarray(0,record.byte_end).toString().length},
    argumentAX:[argument,{...argument,name:'Argument 2 (i32)'}],invalidAX:[argument],diagnosticAX:[source,status],
    inertSource:{sourceExact:true,imgElements:0,scriptElementsAfter:1,markerAbsent:true,scriptElementsBefore:1},
    sessionIsolation:'unrun',arbitraryInertMarkup:presentationFixture(),audibleAssistiveTechnology:'unrun'};
}

// Same explicitly synthetic four-branch shape as the presentation validator controls.
const text = '<img src=x onerror="globalThis.__zrynaPresentationProbe=1"><script>globalThis.__zrynaPresentationProbe=1</script>';
function presentationFixture() {
  const counts = () => ({ script: 1, img: 0, iframe: 0, svg: 0, object: 0, embed: 0, eventAttributes: 0 });
  const rows = ['diagnostic', 'unavailable', 'error', 'identity'].map(id => ({
    id, revision: 2, source: text, sourceInvalid: 'false',
    status: id === 'diagnostic' ? 'Compilation rejected. Review the compiler diagnostics.' :
      id === 'unavailable' ? `Compiler unavailable: PLAYGROUND-UNAVAILABLE — ${text}` :
      id === 'error' ? `Compilation failed: ${text}` : 'Compiled. Choose an export and run it.',
    identity: id === 'identity' ? `Source revision 2; source SHA-256 ${text}; component SHA-256 ${text}` : '',
    diagnostics: id === 'diagnostic' ? [
      { tag: 'P', text: `error: PRESENTATION-FIXTURE — ${text}` },
      { tag: 'BUTTON', text: `${text}:1:1–1:1 (UTF-8 bytes 0–0)` },
      { tag: 'P', text },
    ] : [],
    before: counts(), after: counts(), detachedDocument: true, markerAbsent: true,
  }));
  return { format: 'zryna.ui-inert-presentation-negative.v1', version: 1,
    fixture: 'SYNTHETIC_PRESENTATION_NEGATIVE', actualCompilerResult: false,
    renderer: 'ORIGINAL_MOUNT_PLAYGROUND', rows, liveDocumentUnchanged: true };
}
