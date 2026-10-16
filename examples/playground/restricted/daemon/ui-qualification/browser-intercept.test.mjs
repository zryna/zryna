// Real local TCP/HTTP fixtures and explicit browser-event fixtures only; no actual browser acceptance.
import assert from 'node:assert/strict';import test from 'node:test';
import { EventEmitter } from 'node:events';import { createServer, request as httpRequest } from 'node:http';
import { interceptBrowser } from './browser-intercept.mjs';import { bounds, sha256 } from './wire.mjs';
const headers = ['content-type','cache-control','content-security-policy','x-content-type-options','referrer-policy','cross-origin-opener-policy','cross-origin-resource-policy'].map(name=>({name,value:name==='content-type'?'text/plain':name==='content-security-policy'?"default-src 'none'":'same-origin'}));
async function unusedOrigin() { const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));const origin=`http://127.0.0.1:${server.address().port}`;await new Promise(resolve=>server.close(resolve));return origin; }
async function fixture(sendOverride) {
 const origin=await unusedOrigin(),context=new EventEmitter(),sent=[],waiters=new Map();
 const proxy=await interceptBrowser(context,origin,record=>{sent.push(record);waiters.get(sent.length)?.();return sendOverride?.(record);});
 function request(path='/examples/playground/restricted/',method='GET',body='',extra={}) {
  const request=httpRequest(origin+path,{method,agent:false,...extra});
  const result=new Promise((resolve,reject)=>{request.once('error',reject);request.once('response',response=>{const chunks=[];response.on('data',v=>chunks.push(v));response.on('error',reject);response.on('end',()=>resolve({status:response.statusCode,headers:response.headers,body:Buffer.concat(chunks)}));});});result.catch(()=>{});
  if(body!==null)request.end(body);return{request,result};
 }
 return{origin,context,proxy,sent,request,wait:size=>sent.length>=size?Promise.resolve():new Promise(resolve=>waiters.set(size,resolve))};
}
const reply=(id,body='actual bounded reply',selected=headers)=>({version:1,type:'response',id,status:200,headers:selected,bodyBase64:Buffer.from(body).toString('base64')});
function browserCompile(f,body='{}') { return {url:()=>f.origin+'/compile',method:()=>'POST',resourceType:()=>'fetch',postDataBuffer:()=>Buffer.from(body)}; }

test('actual loopback HTTP forwards observed headers unchanged and preserves status/security/body',async()=>{
 const f=await fixture();try{
  const first=f.request('/session');await f.wait(1);assert.deepEqual(f.sent[0].headers,[]);f.proxy.accept(reply(1));assert.equal((await first.result).status,200);
  const second=f.request('/session','GET','',{headers:{authorization:'Bearer fixture','sec-fetch-site':'same-origin',origin:f.origin}});await f.wait(2);
  assert.deepEqual(Object.fromEntries(f.sent[1].headers.map(v=>[v.name,v.value])),{authorization:'Bearer fixture','sec-fetch-site':'same-origin',origin:f.origin});
  f.proxy.accept(reply(2));const actual=await second.result;assert.equal(actual.body.toString(),'actual bounded reply');assert.equal(actual.headers['content-security-policy'],"default-src 'none'");
 }finally{await f.proxy.stop();}
});
test('asynchronous body completion reserves unique IDs in actual send order',async()=>{
 const f=await fixture();try{
  const first=f.request('/cancel','POST',null,{headers:{'content-length':'2'}});first.request.write('{');
  const second=f.request('/cancel','POST','{}');await f.wait(1);assert.equal(f.sent[0].id,1);assert.equal(Buffer.from(f.sent[0].bodyBase64,'base64').toString(),'{}');
  first.request.end('}');await f.wait(2);assert.deepEqual(f.sent.map(v=>v.id),[1,2]);f.proxy.accept(reply(1));f.proxy.accept(reply(2));await Promise.all([first.result,second.result]);
 }finally{await f.proxy.stop();}
});
test('binding the exact original port rejects collision without selecting another listener',async()=>{
 const server=createServer();await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));try{
  await assert.rejects(interceptBrowser(new EventEmitter(),`http://127.0.0.1:${server.address().port}`,()=>{}),error=>error.code==='EADDRINUSE');
 }finally{await new Promise(resolve=>server.close(resolve));}
});
test('unknown routes, duplicate selected headers and oversized bodies fail before forwarding',async()=>{
 for(const options of [f=>f.request('/unapproved-route'),f=>f.request('/session','GET','',{headers:['Host',new URL(f.origin).host,'Authorization','one','Authorization','two']}),f=>f.request('/cancel','POST','x'.repeat(1025))]){
  const f=await fixture();try{const pending=options(f);await assert.rejects(f.proxy.failures,/UI-TRANSPORT/);await assert.rejects(pending.result);assert.equal(f.sent.length,0);}finally{await f.proxy.stop();}
 }
});
test('missing security headers are never repaired and duplicate or foreign host records reject',async()=>{
 const f=await fixture();try{const request=f.request();await f.wait(1);assert.throws(()=>f.proxy.accept(reply(1,'x',[])),/UI-TRANSPORT/);assert.throws(()=>f.proxy.accept(reply(99)),/UI-TRANSPORT/);await f.proxy.stop();await assert.rejects(request.result);}finally{await f.proxy.stop();}
});
test('both actual local socket close and the matching browser requestfailed are required for abort',async()=>{
 for(const browserFirst of [true,false]){
  const f=await fixture();try{
   const browser=browserCompile(f);f.context.emit('request',browser);const request=f.request('/compile','POST','{}');await f.wait(1);f.proxy.accept({version:1,type:'probe-paused',id:1});
   if(browserFirst)f.context.emit('requestfailed',browser);else{request.request.destroy();await assert.rejects(request.result);await new Promise(resolve=>setImmediate(resolve));}
   assert.equal(f.sent.filter(v=>v.type==='abort').length,0);
   if(browserFirst){request.request.destroy();await assert.rejects(request.result);}else f.context.emit('requestfailed',browser);
   await f.wait(2);assert.deepEqual(f.sent[1],{version:1,type:'abort',id:1});assert.deepEqual(f.proxy.abortProof(),[]);
   f.proxy.accept({version:1,type:'aborted',id:1});assert.deepEqual(f.proxy.abortProof(),[{id:1,route:'compile',paused:true,event:'requestfailed',acknowledged:true,httpResponseClosed:true,requestBytes:2,requestSha256:sha256(Buffer.from('{}'))}]);
   assert.throws(()=>f.proxy.accept({version:1,type:'aborted',id:1}),/UI-TRANSPORT/);
  }finally{await f.proxy.stop();}
 }
});
test('compile hash mismatch, a second compile and foreign requestfailed cannot establish proof',async()=>{
 for(const kind of ['hash','second','foreign']){
  const f=await fixture();try{
   const browser=browserCompile(f,kind==='hash'?'different':'{}');f.context.emit('request',browser);const request=f.request('/compile','POST','{}');
   if(kind==='hash'){await assert.rejects(f.proxy.failures,/UI-TRANSPORT/);await assert.rejects(request.result);assert.equal(f.sent.length,0);}
   else{await f.wait(1);f.proxy.accept({version:1,type:'probe-paused',id:1});f.context.emit(kind==='second'?'request':'requestfailed',browserCompile(f));await assert.rejects(f.proxy.failures,/UI-TRANSPORT/);await f.proxy.stop();await assert.rejects(request.result);}
   assert.deepEqual(f.proxy.abortProof(),[]);assert.equal(f.sent.filter(v=>v.type==='abort').length,0);
  }finally{await f.proxy.stop();}
 }
});
test('a host pause can precede the browser event without exposing an unassociated cancellation proof',async()=>{
 const f=await fixture();try{
  const request=f.request('/compile','POST','{}');await f.wait(1);f.proxy.accept({version:1,type:'probe-paused',id:1});assert.equal(f.proxy.probeState(),null);
  const browser=browserCompile(f);f.context.emit('request',browser);assert.equal(f.proxy.probeState().id,1);
  request.request.destroy();await assert.rejects(request.result);f.context.emit('requestfailed',browser);await f.wait(2);
  f.proxy.accept({version:1,type:'aborted',id:1});assert.equal(f.proxy.abortProof()[0].requestSha256,sha256(Buffer.from('{}')));
 }finally{await f.proxy.stop();}
});
test('corpus admission is one-way and requires the fully drained genuine abort before later compiles',async()=>{
 const f=await fixture();try{
  assert.throws(()=>f.proxy.beginCorpus(),/UI-TRANSPORT/);
  const browser=browserCompile(f);f.context.emit('request',browser);const initial=f.request('/compile','POST','{}');await f.wait(1);
  f.proxy.accept({version:1,type:'probe-paused',id:1});assert.throws(()=>f.proxy.beginCorpus(),/UI-TRANSPORT/);
  initial.request.destroy();await assert.rejects(initial.result);f.context.emit('requestfailed',browser);await f.wait(2);
  assert.throws(()=>f.proxy.beginCorpus(),/UI-TRANSPORT/);f.proxy.accept({version:1,type:'aborted',id:1});
  await new Promise(resolve=>setImmediate(resolve));await f.proxy.beginCorpus();
  assert.deepEqual(f.sent[2],{version:1,type:'corpus-begin',id:1});assert.throws(()=>f.proxy.beginCorpus(),/UI-TRANSPORT/);
  f.context.emit('request',browserCompile(f,'later'));const next=f.request('/compile','POST','later');await f.wait(4);
  assert.equal(f.sent[3].id,2);f.proxy.accept(reply(2));assert.equal((await next.result).status,200);
  assert.equal(f.proxy.abortProof().length,1,'later successful compilation cannot replace the initial proof');
 }finally{await f.proxy.stop();}
});
test('owned pending sockets and body readers close; a stalled stdio send has one finite cleanup', {timeout:6000},async()=>{
 const f=await fixture(()=>new Promise(()=>{}));const request=f.request('/session');await f.wait(1);const start=performance.now(),stopping=f.proxy.stop();assert.equal(stopping,f.proxy.stop());await assert.rejects(stopping,/UI-CLEANUP/);assert.ok(performance.now()-start<3000);await assert.rejects(request.result);
 const rejected=httpRequest(f.origin+'/session');const result=new Promise(resolve=>rejected.once('error',resolve));rejected.end();assert.equal((await result).code,'ECONNREFUSED');
 const reader=await fixture();const body=reader.request('/cancel','POST',null,{headers:{'content-length':'10'}});body.request.write('x');await new Promise(resolve=>setImmediate(resolve));await reader.proxy.stop();await assert.rejects(body.result);assert.equal(reader.sent.length,0);
});
test('accepted sockets and pending jobs cannot exceed the existing live plus queue bound',async()=>{
 const f=await fixture();const requests=[];try{
  for(let i=0;i<bounds.live+bounds.queued;i++)requests.push(f.request('/session'));await f.wait(bounds.live+bounds.queued);
  const extra=f.request('/session');await assert.rejects(extra.result);await assert.rejects(f.proxy.failures,/UI-TRANSPORT/);assert.equal(f.sent.length,bounds.live+bounds.queued);
 }finally{await f.proxy.stop();await Promise.allSettled(requests.map(v=>v.result));}
});
