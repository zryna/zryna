// Local HTTP and explicit event fixtures only; no actual Chromium or compiler acceptance.
import assert from 'node:assert/strict';
import test from 'node:test';
import { EventEmitter } from 'node:events';
import { createServer, request as httpRequest } from 'node:http';
import { createConnection } from 'node:net';
import { interceptBrowser } from './browser-intercept.mjs';
import { browserFailure } from './browser-probe.mjs';
import { routes } from './routes.mjs';
import { requestFailureBranch, observeBrowserRequests, requestFailureObservation } from './browser-diagnostics.mjs';
import { sha256, transportBranches, transportBranch, validateRequestObservation, innerFailure, emptyBrowserProcess } from './wire.mjs';
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
const sessionRequest=()=>({url:()=> 'http://127.0.0.1:12345/session',method:()=> 'GET',resourceType:()=> 'fetch'});
test('request identity is exact object, bounded ordinal and immutable response/finish snapshot',()=>{
  const context=new EventEmitter(),observer=observeBrowserRequests(context,'http://127.0.0.1:12345');
  const first=sessionRequest(),second=sessionRequest(),unseen=sessionRequest();context.emit('request',first);context.emit('request',second);
  context.emit('response',{request:()=>second,status:()=>200});context.emit('requestfinished',second);
  const firstError=observer.failed(first,new Error('private metadata')),secondError=observer.failed(second,new Error('private metadata'));
  const one=requestFailureObservation(firstError),two=requestFailureObservation(secondError);
  assert.deepEqual(one,{observed:true,ordinal:1,route:'session',method:'GET',resourceType:'fetch',responseSeen:false,responseStatus:null,requestFinished:false});
  assert.deepEqual(two,{...one,ordinal:2,responseSeen:true,responseStatus:200,requestFinished:true});
  context.emit('response',{request:()=>first,status:()=>200});context.emit('requestfinished',first);
  assert.deepEqual(requestFailureObservation(firstError),one,'failure snapshot precedes later observations');
  const unknown=requestFailureObservation(observer.failed(unseen,new Error()));
  assert.deepEqual(unknown,{observed:false,ordinal:null,route:null,method:null,resourceType:null,responseSeen:false,responseStatus:null,requestFinished:false});
  for(const value of [one,two,unknown])validateRequestObservation(value);
  assert.equal(requestFailureObservation(new Error('unrelated guard')),null);
  observer.stop();for(const name of ['request','response','requestfinished'])assert.equal(context.listenerCount(name),0);
});
test('diagnostic request count never exceeds256 and forbidden/unknown metadata gains no identity',()=>{
  const context=new EventEmitter(),observer=observeBrowserRequests(context,'http://127.0.0.1:12345');let last;
  for(let index=0;index<256;index++){last=sessionRequest();context.emit('request',last);}
  assert.equal(requestFailureObservation(observer.failed(last,new Error())).ordinal,256);
  const extra=sessionRequest();context.emit('request',extra);assert.equal(requestFailureObservation(observer.failed(extra,new Error())).observed,false);
  observer.stop();
  const other=new EventEmitter(),selected=observeBrowserRequests(other,'http://127.0.0.1:12345');
  for(const request of [{...sessionRequest(),url:()=> 'http://private/#capability'},
    {...sessionRequest(),resourceType:()=> 'unknown'}, {url(){throw Error('private');}}]){
    other.emit('request',request);assert.equal(requestFailureObservation(selected.failed(request,new Error())).observed,false);
  }selected.stop();
});
test('request attribution rejects forged nonnetwork causes and accepts old optional-absence records',()=>{
  const base={version:1,type:'failure',runId:'a'.repeat(32),phase:'SOURCE_READY',cause:'TRANSPORT',cleanupFailed:false,
    cleanup:{intercept:false,context:false,browser:false},lifecycle:{browserDisconnected:false,pageClosed:false,pageCrashed:false,contextClosed:false},
    process:emptyBrowserProcess(),branch:'INTERCEPT_REQUEST_FAILED_ERR_ABORTED'};
  const observed={observed:true,ordinal:1,route:'session',method:'GET',resourceType:'fetch',responseSeen:true,responseStatus:200,requestFinished:false};
  innerFailure(base,base.runId);innerFailure({...base,requestObservation:observed},base.runId);
  for(const [route,configuration] of Object.entries(routes))if(route!=='asset:worker.mjs')validateRequestObservation({...observed,route,method:configuration.method});
  for(const changed of [{branch:'INTERCEPT_005'},{cause:'SCRIPT_SOURCE'},{requestObservation:{...observed,ordinal:true}},
    {requestObservation:{...observed,responseStatus:true}},{requestObservation:{...observed,method:'POST'}},
    {requestObservation:{...observed,url:'private'}},{requestObservation:{...observed,observed:false}}]){
    assert.throws(()=>innerFailure({...base,requestObservation:observed,...changed},base.runId));
  }
});
test('the actual first transport Error receives only its own exact Request snapshot',async()=>{
  for(const earlierGuard of [false,true]){
    const f=await fixture();let first;try{
      if(earlierGuard)f.context.emit('request',{url:()=> f.origin+'/unapproved',method:()=> 'GET',resourceType:()=> 'fetch'});
      const request={url:()=> f.origin+'/session',method:()=> 'GET',resourceType:()=> 'fetch',failure:()=>({errorText:'net::ERR_ABORTED'})};
      f.context.emit('request',request);f.context.emit('response',{request:()=>request,status:()=>200});f.context.emit('requestfailed',request);
      await assert.rejects(f.proxy.failures,error=>{first=error;return true;});
      const result=browserFailure(first);
      if(earlierGuard){assert.equal(result.branch,'ROUTES_005');assert.equal(result.requestObservation,null);}
      else{
        assert.equal(result.branch,'INTERCEPT_REQUEST_FAILED_ERR_ABORTED');
        assert.deepEqual(result.requestObservation,{observed:true,ordinal:1,route:'session',method:'GET',resourceType:'fetch',
          responseSeen:true,responseStatus:200,requestFinished:false});
        f.context.emit('requestfinished',request);assert.equal(browserFailure(first).requestObservation.requestFinished,false);
      }
      assert.ok(!/private|capability|http:/.test(JSON.stringify(result)));
    }finally{await f.proxy.stop();}
  }
});
