// Local HTTP and explicit event fixtures only; no actual Chromium or compiler acceptance.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EventEmitter } from 'node:events';
import { createServer, request as httpRequest } from 'node:http';
import { createConnection } from 'node:net';
import { interceptBrowser } from './browser-intercept.mjs';
import { requestFailureBranch, sha256, transportBranches, transportBranch } from './wire.mjs';
const security=['content-type','cache-control','content-security-policy','x-content-type-options',
  'referrer-policy','cross-origin-opener-policy','cross-origin-resource-policy'].map(name=>({name,value:'same-origin'}));
async function fixture(){
  const selected=createServer();await new Promise(resolve=>selected.listen(0,'127.0.0.1',resolve));
  const origin=`http://127.0.0.1:${selected.address().port}`;await new Promise(resolve=>selected.close(resolve));
  const context=new EventEmitter(),sent=[],waiters=new Map();
  const proxy=await interceptBrowser(context,origin,record=>{sent.push(record);waiters.get(sent.length)?.();});
  const wait=count=>sent.length>=count?Promise.resolve():new Promise(resolve=>waiters.set(count,resolve));
  function request(body){
    const outgoing=httpRequest(origin+'/compile',{method:'POST',agent:false});
    const result=new Promise((resolve,reject)=>{
      outgoing.once('error',reject);outgoing.once('response',response=>{
        const chunks=[];response.on('data',chunk=>chunks.push(chunk));response.once('error',reject);
        response.once('end',()=>resolve(Buffer.concat(chunks)));
      });
    });result.catch(()=>{});outgoing.end(body);return {outgoing,result};
  }
  return {context,proxy,sent,wait,request,origin};
}
const browserRequest=(f,body='{}')=>({url:()=>f.origin+'/compile',method:()=>'POST',resourceType:()=>'fetch',
  postDataBuffer:()=>Buffer.from(body)});
async function corpus(f){
  const initial=browserRequest(f);f.context.emit('request',initial);const request=f.request('{}');await f.wait(1);
  f.proxy.accept({version:1,type:'probe-paused',id:1});request.outgoing.destroy();await assert.rejects(request.result);
  f.context.emit('requestfailed',initial);await f.wait(2);f.proxy.accept({version:1,type:'aborted',id:1});
  await new Promise(resolve=>setImmediate(resolve));await f.proxy.beginCorpus();assert.equal(f.proxy.abortProof().length,1);
}

test('requestfailed metadata becomes an exact fixed category or UNKNOWN without raw content',()=>{
  for(const code of ['ERR_ABORTED','ERR_FAILED','ERR_BLOCKED_BY_RESPONSE','ERR_CONTENT_LENGTH_MISMATCH']){
    const branch=requestFailureBranch({failure:()=>({errorText:'net::'+code})});
    assert.equal(branch,'INTERCEPT_REQUEST_FAILED_'+code);assert.ok(transportBranches.includes(branch));
  }
  for(const value of [undefined,null,7,{},'private-capability','net::ERR_ABORTED https://private/#secret',
    'net::ERR_UNKNOWN','net::ERR_FAILED\nBearer private','x'.repeat(257)]){
    assert.equal(requestFailureBranch({failure:()=>({errorText:value})}),'INTERCEPT_REQUEST_FAILED_UNKNOWN');
  }
  assert.equal(requestFailureBranch({failure(){throw new Error('private URL/token');}}),'INTERCEPT_REQUEST_FAILED_UNKNOWN');
});
test('each forbidden page/context event has its own closed branch and owned listeners detach',async()=>{
  for(const name of ['worker','websocket','download','popup','serviceworker','second-page','invalidate']){
    const f=await fixture();const page=new EventEmitter();try{
      f.context.emit('page',page);
      if(name==='second-page')f.context.emit('page',new EventEmitter());
      else if(name==='serviceworker')f.context.emit(name,{url:'private'});
      else if(name==='invalidate')f.proxy.invalidate();
      else page.emit(name,{url:'private'});
      await assert.rejects(f.proxy.failures,/PLAYGROUND-UI-TRANSPORT/);
      const expected=name==='second-page'?'INTERCEPT_SECOND_PAGE':name==='serviceworker'?'INTERCEPT_SERVICE_WORKER':
        name==='invalidate'?'INTERCEPT_INVALIDATE':'INTERCEPT_PAGE_'+name.toUpperCase();
      assert.equal(f.proxy.failureBranch(),expected);
    }finally{await f.proxy.stop();}
    assert.equal(f.context.listenerCount('serviceworker'),0);
    for(const event of ['worker','websocket','download','popup'])assert.equal(page.listenerCount(event),0);
  }
});
test('a foreign failed request retains only the known network category and cannot replace initial abort proof',async()=>{
  const f=await fixture();try{
    await corpus(f);f.context.emit('requestfailed',{failure:()=>({errorText:'net::ERR_CONNECTION_RESET'})});
    await assert.rejects(f.proxy.failures,/PLAYGROUND-UI-TRANSPORT/);
    assert.equal(f.proxy.failureBranch(),'INTERCEPT_REQUEST_FAILED_ERR_CONNECTION_RESET');
    assert.equal(f.proxy.abortProof().length,1);assert.equal(f.sent.filter(record=>record.type==='abort').length,1);
  }finally{await f.proxy.stop();}
});
test('a premature corpus response close is distinct and never reuses the initial cancellation proof',async()=>{
  const f=await fixture();try{
    await corpus(f);const next=f.request('later');await f.wait(4);next.outgoing.destroy();await assert.rejects(next.result);
    await assert.rejects(f.proxy.failures,/PLAYGROUND-UI-TRANSPORT/);
    assert.equal(f.proxy.failureBranch(),'INTERCEPT_RESPONSE_CLOSE');assert.equal(f.proxy.abortProof().length,1);
    assert.equal(f.sent.filter(record=>record.type==='abort').length,1);
  }finally{await f.proxy.stop();}
});
test('6122-byte corpus reply is forwarded exactly through real local HTTP after genuine fixture abort',async()=>{
  const f=await fixture();try{
    await corpus(f);const next=f.request('later');await f.wait(4);const body=Buffer.alloc(6122);
    for(let index=0;index<body.length;index++)body[index]=index%256;
    f.proxy.accept({version:1,type:'response',id:2,status:200,headers:security,bodyBase64:body.toString('base64')});
    const actual=await next.result;assert.equal(actual.length,6122);assert.equal(sha256(actual),sha256(body));
    assert.equal(f.proxy.failureBranch(),'UNSPECIFIED');
  }finally{await f.proxy.stop();}
});
test('malformed local HTTP is attributed to client parser error without retaining parser text',async()=>{
  const f=await fixture();try{
    const target=new URL(f.origin),socket=createConnection({host:'127.0.0.1',port:Number(target.port)});
    const closed=new Promise(resolve=>socket.once('close',resolve));socket.on('error',()=>{});
    await new Promise(resolve=>socket.once('connect',resolve));socket.end('INVALID HTTP\r\n\r\n');
    await assert.rejects(f.proxy.failures,/PLAYGROUND-UI-TRANSPORT/);await closed;
    assert.equal(f.proxy.failureBranch(),'INTERCEPT_CLIENT_ERROR');assert.equal(f.sent.length,0);
  }finally{await f.proxy.stop();}
});
test('a late valid record after stop remains a fatal005 transport guard',async()=>{
  const f=await fixture();await f.proxy.stop();
  assert.throws(()=>f.proxy.accept({version:1,type:'aborted',id:1}),error=>transportBranch(error)==='INTERCEPT_005');
});
