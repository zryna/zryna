// Local real TCP/HTTP byte fixtures only; no Chromium or product qualification.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EventEmitter } from 'node:events';
import { createServer } from 'node:http';
import { createConnection } from 'node:net';
import { interceptBrowser } from './browser-intercept.mjs';
import { loopbackResponseHeaders, responseHeaders, responseRecord, routeFor } from './routes.mjs';
import { transportBranch } from './wire.mjs';
const selected = ['content-type','cache-control','content-security-policy','x-content-type-options','referrer-policy','cross-origin-opener-policy','cross-origin-resource-policy'].map(name => ({name,value:name === 'content-type' ? 'application/octet-stream' : name === 'content-security-policy' ? "default-src 'none'" : 'same-origin'}));
function record(status, body) { return {version:1,type:'response',id:1,status,headers:selected,bodyBase64:body.toString('base64')}; }
async function fixture() {
 const reserve=createServer();await new Promise(resolve=>reserve.listen(0,'127.0.0.1',resolve));const port=reserve.address().port;
 await new Promise(resolve=>reserve.close(resolve));let received;const ready=new Promise(resolve=>{received=resolve;});
 const context=new EventEmitter(),proxy=await interceptBrowser(context,`http://127.0.0.1:${port}`,message=>received(message));
 const socket=createConnection({host:'127.0.0.1',port});let count=0;const chunks=[];
 const raw=new Promise((resolve,reject)=>{socket.on('error',reject);socket.on('data',chunk=>{count+=chunk.length;if(count>140000){socket.destroy();reject(new Error('fixture byte bound'));}else chunks.push(chunk);});socket.on('close',()=>resolve(Buffer.concat(chunks)));});raw.catch(()=>{});
 socket.setTimeout(2000,()=>socket.destroy(new Error('fixture timeout')));
 socket.end(`GET /session HTTP/1.1\r\nHost: 127.0.0.1:${port}\r\nConnection: close\r\n\r\n`);
 return {context,proxy,ready,raw,stop:async()=>{socket.destroy();await proxy.stop();}};
}
function parse(raw) {
 const offset=raw.indexOf('\r\n\r\n');assert.ok(offset>=0);const lines=raw.subarray(0,offset).toString('ascii').split('\r\n');
 const headers=Object.create(null);for(const line of lines.slice(1)){const i=line.indexOf(':');assert.ok(i>0);const name=line.slice(0,i).toLowerCase();assert.equal(Object.hasOwn(headers,name),false);headers[name]=line.slice(i+1).trim();}
 return {status:Number(lines[0].split(' ')[1]),headers,body:raw.subarray(offset+4)};
}
test('complete validated binary and multibyte responses have exact byte framing without chunks',async()=>{
 for(const body of [Buffer.from('{"finished":true}'),Buffer.from('é漢🙂'),Buffer.from(Array.from({length:6122},(_,i)=>i%256)),Buffer.alloc(0)]){
  const f=await fixture();try{assert.equal((await f.ready).route,'session');f.proxy.accept(record(200,body));const actual=parse(await f.raw);
   assert.equal(actual.status,200);assert.equal(actual.headers['content-length'],String(body.length));assert.equal(actual.headers['transfer-encoding'],undefined);assert.equal(actual.headers.connection,'close');assert.deepEqual(actual.body,body);
   for(const {name,value} of selected)assert.equal(actual.headers[name],value);
  }finally{await f.stop();}
 }
});
test('204 is empty without content-length or transfer-encoding; 205 has explicit length zero',async()=>{
 for(const status of [204,205]){const f=await fixture();try{await f.ready;f.proxy.accept(record(status,Buffer.alloc(0)));const actual=parse(await f.raw);
  assert.equal(actual.status,status);assert.equal(actual.headers['content-length'],status===204?undefined:'0');assert.equal(actual.headers['transfer-encoding'],undefined);assert.equal(actual.body.length,0);
 }finally{await f.stop();}}
});
test('nonempty 204/205 bodies fail before any local response headers or bytes',async()=>{
 for(const status of [204,205]){const f=await fixture();try{await f.ready;f.proxy.accept(record(status,Buffer.from('forbidden')));
  const error=await f.proxy.failures.catch(error=>error);assert.equal(transportBranch(error),'ROUTES_012');assert.equal((await f.raw).length,0);
 }finally{await f.stop();}}
});
test('transport lengths remain untrusted, security requirements and unreachable HEAD/304 stay enforced',()=>{
 const raw=selected.flatMap(({name,value})=>[name,value]);const stripped=responseHeaders([...raw,'Content-Length','999','Transfer-Encoding','chunked']);
 assert.deepEqual(stripped,selected);const value=record(200,Buffer.from('é'));
 const validated=responseRecord({...value,headers:stripped},'session');assert.equal(loopbackResponseHeaders(200,validated)['content-length'],'2');
 assert.throws(()=>responseRecord({...value,headers:[...selected,{name:'content-length',value:'999'}]},'session'),/UI-TRANSPORT/);
 assert.throws(()=>responseRecord({...value,headers:selected.slice(1)},'session'),/UI-TRANSPORT/);
 assert.throws(()=>responseRecord({...value,status:304},'session'),/UI-TRANSPORT/);
 assert.throws(()=>routeFor('http://127.0.0.1:12345/session','HEAD','http://127.0.0.1:12345'),/UI-TRANSPORT/);
});
