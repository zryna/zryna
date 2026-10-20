import assert from 'node:assert/strict';
import test from 'node:test';
import {mkdtempSync,writeFileSync,rmSync,realpathSync} from 'node:fs';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {runWorker,expectedHandshake,verifyAnalysisFrame,compareAnalysisFrames} from './worker.mjs';
import {strictJson} from './json.mjs';

function fake(t,body){const cwd=mkdtempSync(join(tmpdir(),'zryna-native-worker-control-'));t.after(()=>rmSync(cwd,{recursive:true,force:true}));const script=join(cwd,'worker.mjs');writeFileSync(script,"import readline from 'node:readline';\nconst input=readline.createInterface({input:process.stdin});\n"+body);return {program:realpathSync(process.execPath),args:[script],cwd,protocol:2,provider:'native',files:[{path:'secret.zry',text:'original source'}],timeoutMs:3000};}
test('source is withheld after hostile handshake identity',async t=>{
 const handshake=expectedHandshake(2,'native');handshake.provider='foreign';const options=fake(t,`input.on('line',line=>{const x=JSON.parse(line);console.log(JSON.stringify({id:x.id,result:${JSON.stringify(handshake)}}));});`);
 await assert.rejects(runWorker(options),error=>{assert.equal(error.capture.sourceSent,false);assert.equal(error.capture.frames.length,1);return true;});
});
test('actual two-phase exchange retains whole original response and closes native worker',async t=>{
 const handshake=expectedHandshake(2,'native'),snapshot={schema_version:2,files:[],diagnostics:[]};const options=fake(t,`input.on('line',line=>{const x=JSON.parse(line);console.log(JSON.stringify({id:x.id,result:x.id===1?${JSON.stringify(handshake)}:${JSON.stringify(snapshot)}}));});`);const capture=await runWorker(options);assert.equal(capture.exitCode,0);assert.equal(capture.sourceSent,true);assert.equal(capture.ownerExitObserved,true);assert.equal(capture.descendantCleanupClaimed,false);assert.deepEqual(capture.frames[1].result,snapshot);assert.equal(capture.stdout.toString().split('\n').length,3);
});
test('extra candidate frame cannot be accepted as complete observation',async t=>{
 const handshake=expectedHandshake(2,'native');const options=fake(t,`input.on('line',line=>{const x=JSON.parse(line);console.log(JSON.stringify({id:x.id,result:x.id===1?${JSON.stringify(handshake)}:{schema_version:2,files:[],diagnostics:[]}}));if(x.id===2)console.log(JSON.stringify({id:3,result:{}}));});`);await assert.rejects(runWorker(options),/extra worker response/);
});
test('silent worker reaches finite failure rather than skipped pass',async t=>{
 const options=fake(t,"input.on('line',()=>{});");options.timeoutMs=50;await assert.rejects(runWorker(options),error=>{assert.equal(error.capture.sourceSent,false);return /deadline/.test(error.message);});
});
test('coherent duplicate response fields and invalid UTF8 cannot disappear during comparison',()=>{
 assert.throws(()=>strictJson(Buffer.from('{"id":99,"id":1,"result":{}}'),100),/duplicate/);
 assert.throws(()=>strictJson(Buffer.from('{"result":{"span":1,"sp\\u0061n":2}}'),100),/duplicate/);
 assert.throws(()=>strictJson(Buffer.from([0xc0,0x80]),10));
 assert.deepEqual(strictJson(Buffer.from('{"value":"a,b{\\\"x\\\":1}","nested":[{"id":1},{"id":2}]}'),100),{value:'a,b{"x":1}',nested:[{id:1},{id:2}]});
});

test('closed source rejection is retained as an actual correlated second frame',async t=>{
 const handshake=expectedHandshake(2,'native'),error={code:'ZRYNA-F2002',message:'original unsupported source'};
 const options=fake(t,`input.on('line',line=>{const x=JSON.parse(line);console.log(JSON.stringify(x.id===1?{id:1,result:${JSON.stringify(handshake)}}:{id:2,error:${JSON.stringify(error)}}));});`);
 const capture=await runWorker(options);assert.equal(capture.sourceSent,true);assert.deepEqual(capture.frames[1],{id:2,error});assert.equal(capture.exitCode,0);
});
test('malformed, mixed, uncorrelated and oversized rejection frames fail closed',()=>{
 const valid={id:2,error:{code:'ZRYNA-F2002',message:'original rejection'}};
 for(const mutate of [f=>f.id=3,f=>f.error.extra=true,f=>f.result={},f=>f.error.code=2002,f=>f.error.message='',f=>f.error.code='x'.repeat(129),f=>f.error.message='x'.repeat(16385),f=>f.error.message='\ud800',f=>f.extra=1]){const frame=structuredClone(valid);mutate(frame);assert.throws(()=>verifyAnalysisFrame(frame));}
 assert.throws(()=>verifyAnalysisFrame({id:2,result:[]}));
});
test('a rejection followed by an extra error frame is never a complete worker result',async t=>{
 const handshake=expectedHandshake(2,'native');const options=fake(t,`input.on('line',line=>{const x=JSON.parse(line);if(x.id===1)console.log(JSON.stringify({id:1,result:${JSON.stringify(handshake)}}));else{console.log(JSON.stringify({id:2,error:{code:'ZRYNA-F2002',message:'source rejected'}}));console.log(JSON.stringify({id:2,error:{code:'ZRYNA-F2002',message:'second rejection'}}));}});`);
 await assert.rejects(runWorker(options),/extra worker response/);
});
test('full rejection comparisons reject code, message and success-versus-error divergence',()=>{
 const frame={id:2,error:{code:'ZRYNA-F2002',message:'original source rejection'}};
 compareAnalysisFrames(frame,structuredClone(frame));
 for(const changed of [{id:2,error:{code:'ZRYNA-F1003',message:frame.error.message}},{id:2,error:{code:frame.error.code,message:'different source rejection'}},{id:2,result:{schema_version:2,files:[],diagnostics:[]}}])assert.throws(()=>compareAnalysisFrames(frame,changed));
});
