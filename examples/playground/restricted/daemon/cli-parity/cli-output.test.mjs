// Synthetic serde-shaped parser controls. They are never recorded as actual CLI observations.
import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeCliOutput } from './cli-output.mjs';
import { fixedCorpus } from './plan.mjs';

function output(value, exitCode = 0) {
  return { stdout: Buffer.from(JSON.stringify(value, null, 2) + '\n'), stderr: Buffer.alloc(0), exitCode };
}
function run(value = 42) {
  return { version: 1, command: 'run', ok: true, manifest: '.zryna/out/parity.run/zryna-manifest-v1.json',
    diagnostics: [], results: [{ target: 'javascript', outcome: { kind: 'returned', value: { type: 'i32', value } } }] };
}
test('release version1 output parser requires a typed i32 scalar, exact command and manifest', () => {
  const parsed = decodeCliOutput('run-addition-0', output(run()));
  assert.equal(parsed.value, 42); assert.equal(parsed.status, 'compiled');
  assert.equal(parsed.capture.exitCode, 0); assert.equal(parsed.capture.stderrBytes, 0);
  assert.equal(decodeCliOutput('run-addition-1', output(run(-2147483648))).value, -2147483648);
  for (const mutate of [value => { value.results[0].target = 'native'; }, value => { value.command = 'build'; },
    value => { value.manifest = '/tmp/other.json'; }, value => { value.results[0].outcome.value.type = 'bool'; },
    value => { value.results[0].outcome.value.value = 43; }, value => { value.results.push(value.results[0]); },
    value => { value.extra = true; }]) {
    const value = run(); mutate(value);
    assert.throws(() => decodeCliOutput('run-addition-0', output(value)), /CLI-PARITY/);
  }
  assert.throws(() => decodeCliOutput('run-addition-0', output(run(), 6)), /CLI-PARITY/);
  const stderr = output(run()); stderr.stderr = Buffer.from('unconfirmed cleanup');
  assert.throws(() => decodeCliOutput('run-addition-0', stderr), /CLI-PARITY/);
  assert.throws(() => decodeCliOutput('run-addition-0', { ...output(run()), cleanupConfirmed: true }), /CLI-PARITY/);
});
test('source rejection requires exit3 and authoritative exact file-zero UTF8 span rather than an invocation/setup failure', () => {
  const source = fixedCorpus.find(value => value.id === 'any-rejected').source; const start = source.indexOf('any');
  const value = { version: 1, command: 'build', ok: false, manifest: null, results: [], diagnostics: [{
    code: 'ZRYNA-M1004', severity: 'error', primary: { kind: 'source', span: { file: 0, start, end: start + 3 } },
    message: 'Synthetic parser fixture only', guidance: 'Never an actual execution result',
  }] };
  const parsed = decodeCliOutput('build-any-rejected', output(value, 3));
  assert.equal(parsed.diagnostics[0].byte_start, start); assert.equal(parsed.diagnostics[0].path, 'src/main.zry');
  for (const code of [0, 1, 2, 4, 5, 6, 70]) assert.throws(() => decodeCliOutput('build-any-rejected', output(value, code)), /CLI-PARITY/);
  value.diagnostics[0].primary.span.file = 1;
  assert.throws(() => decodeCliOutput('build-any-rejected', output(value, 3)), /CLI-PARITY/);
});
test('duplicate keys, oversized streams, stdout garbage and unknown operation IDs never become observations', () => {
  for (const text of ['{"version":1,"version":1}', JSON.stringify(run()) + '\n{}', 'x'.repeat(65537)]) {
    assert.throws(() => decodeCliOutput('run-addition-0', { stdout: Buffer.from(text), stderr: Buffer.alloc(0), exitCode: 0 }));
  }
  assert.throws(() => decodeCliOutput('shell', output(run())), /CLI-PARITY/);
});

test('retained legacy raw diagnostic orders project into structured-v1 without rewriting raw captures', async () => {
  const { readFileSync } = await import('node:fs');
  for (const [name, count] of [['bool-gated',3], ['parse-rejected',4]]) {
    const raw = readFileSync(new URL('./fixtures/legacy-'+name+'.json',import.meta.url));
    const value = JSON.parse(raw); const before = Buffer.from(raw);
    const projected = decodeCliOutput('build-'+name,{exitCode:3,stdout:raw,stderr:Buffer.alloc(0)});
    const reversed = structuredClone(value);reversed.diagnostics.reverse();
    assert.deepEqual(decodeCliOutput('build-'+name,output(reversed,3)).diagnostics,projected.diagnostics);
    assert.equal(projected.diagnostics.length,count);assert.deepEqual(raw,before);
    if(name==='bool-gated') assert.match(projected.diagnostics[0].message,/unsupported result type/);
    else assert.deepEqual(projected.diagnostics.map(d=>d.byte_start),[23,29,38,40]);
  }
});
test('structured projection follows Rust Option, span, severity, code and UTF8 order including non-BMP text', async () => {
  const { readFileSync } = await import('node:fs');
  const value=JSON.parse(readFileSync(new URL('./fixtures/legacy-bool-gated.json',import.meta.url)));
  const global=structuredClone(value.diagnostics[0]);
  const messages=['\u{10000}','\ue000','a'];
  value.diagnostics=messages.map(message=>({...structuredClone(global),message}));
  value.diagnostics.push({...structuredClone(global),message:'a',guidance:'\u{10000}'});
  value.diagnostics.push({...structuredClone(global),message:'a',guidance:'\ue000'});
  const projected=decodeCliOutput('build-bool-gated',output(value,3));
  assert.deepEqual(projected.diagnostics.map(d=>d.message),['a','a','a','\ue000','\u{10000}']);
  assert.deepEqual(projected.diagnostics.filter(d=>d.message==='a').map(d=>d.guidance),[global.guidance,'\ue000','\u{10000}']);
  value.diagnostics.push({...structuredClone(global),message:'z',primary:{kind:'source',span:{file:0,start:0,end:1}}});
  value.diagnostics.push({...structuredClone(global),severity:'warning',message:'0'});
  const ordered=decodeCliOutput('build-bool-gated',output(value,3)).diagnostics;
  assert.equal(ordered.at(-1).byte_start,0);assert.equal(ordered.at(-2).severity,'warning');
  value.diagnostics.push({...structuredClone(global),code:'ZRYNA-A0001',message:'z'});
  value.diagnostics.push({...structuredClone(global),message:'z',primary:{kind:'source',span:{file:0,start:0,end:0}}});
  value.diagnostics.push({...structuredClone(global),message:'z',primary:{kind:'source',span:{file:0,start:1,end:1}}});
  const keyed=decodeCliOutput('build-bool-gated',output(value,3)).diagnostics;
  assert.equal(keyed[0].code,'ZRYNA-A0001');
  assert.deepEqual(keyed.filter(d=>d.byte_start!==null).map(d=>[d.byte_start,d.byte_end]),[[0,0],[0,1],[1,1]]);
});
test('projection preserves duplicate counts and changed valid spans while rejecting invalid spans before ordering', async () => {
  const { readFileSync } = await import('node:fs');
  const value=JSON.parse(readFileSync(new URL('./fixtures/legacy-parse-rejected.json',import.meta.url)));
  const baseline=decodeCliOutput('build-parse-rejected',output(value,3)).diagnostics;
  value.diagnostics.push(structuredClone(value.diagnostics[0]));
  assert.equal(decodeCliOutput('build-parse-rejected',output(value,3)).diagnostics.length,baseline.length+1);
  value.diagnostics.pop();value.diagnostics[0].primary.span.start=28;
  const changed=decodeCliOutput('build-parse-rejected',output(value,3)).diagnostics;
  assert.notDeepEqual(changed,baseline);assert(changed.some(d=>d.byte_start===28));
  value.diagnostics[0].primary.span.end=99999;
  assert.throws(()=>decodeCliOutput('build-parse-rejected',output(value,3)),/CLI-PARITY/);
});
