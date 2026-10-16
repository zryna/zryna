// Local protocol/HTTP and explicit fake browser fixtures only; never actual UI qualification.
import assert from 'node:assert/strict';
import test from 'node:test';
import { PassThrough } from 'node:stream';
import { EventEmitter } from 'node:events';
import { createServer, request as httpRequest } from 'node:http';
import { runUiDriver } from './inner.mjs';
import { createFrameWriter, createFrameDecoder } from './framing.mjs';
import { planBytes } from './plan.mjs';
import { sha256, transportBranch } from './wire.mjs';
import { runInitialBrowserProbe, browserFailure } from './browser-probe.mjs';
async function precedence(order, cleanupFails=false, streamBranch='FRAMING_002') {
  const reservation=createServer();await new Promise(resolve=>reservation.listen(0,'127.0.0.1',resolve));
  const origin=`http://127.0.0.1:${reservation.address().port}`;await new Promise(resolve=>reservation.close(resolve));
  const input=new PassThrough(),output=new PassThrough(),writer=createFrameWriter(input),failures=[];
  let cleanupEntered,releaseCleanup,observedAbort;
  const cleaning=new Promise(resolve=>cleanupEntered=resolve),gate=new Promise(resolve=>releaseCleanup=resolve);
  const aborted=new Promise(resolve=>observedAbort=resolve);
  const browser=new EventEmitter(),context=new EventEmitter(),page=new EventEmitter(),cdp=new EventEmitter();let request;
  const originalRequest={url:()=>origin+'/compile',method:()=>'POST',resourceType:()=>'fetch',postDataBuffer:()=>Buffer.from('{}')};
  const source={async waitFor(){},async focus(){},async fill(){},async press(){
    context.emit('request',originalRequest);request=httpRequest(origin+'/compile',{method:'POST',agent:false});
    request.on('error',()=>{});request.end('{}');
  }};
  Object.assign(browser,{async newContext(){return context;},async close(){}});
  Object.assign(context,{async newPage(){context.emit('page',page);return page;},async newCDPSession(){return cdp;},
    async close(){cleanupEntered();await gate;if(cleanupFails)throw new Error('private cleanup details');}});
  cdp.send=async()=>({nodes:[]});
  Object.assign(page,{setDefaultTimeout(){},async goto(){},async waitForFunction(){},
    async evaluate(){return {pathname:'/examples/playground/restricted/',hashRemoved:true,title:'Zryna source playground',
      status:'ready',role:'status',live:'polite'};},
    getByRole(role){
      if(role==='textbox')return source;
      if(role==='button')return {async click(){request.destroy();context.emit('requestfailed',originalRequest);}};
      throw Error('unexpected fixture role');
    }});
  const decoder=createFrameDecoder(record=>{
    if(record.type==='request')writer.write({version:1,type:'probe-paused',id:record.id}).catch(()=>{});
    else if(record.type==='failure')failures.push(record);
    else {assert.equal(record.type,'abort');observedAbort();}
  });output.on('data',chunk=>decoder.push(chunk));
  const run=runUiDriver(input,output,async()=>({async launch(){return browser;}}),sha256(planBytes()));run.catch(()=>{});
  await writer.write({version:1,type:'start',runId:'a'.repeat(32),origin,
    launchPath:'/examples/playground/restricted/',capability:'b'.repeat(64),planSha256:sha256(planBytes())});
  if(order==='browser-first'){
    await cleaning;
    // Actual fixed1000ms abort-proof timeout initiated cleanup; latevalidreply still rejects005.
    await writer.write({version:1,type:'aborted',id:1});await new Promise(resolve=>setImmediate(resolve));
  }else{
    await aborted;await new Promise(resolve=>setImmediate(resolve));
    if(streamBranch==='FRAMING_002')input.write(Buffer.from([255,255,255,127]));
    else await writer.write({version:1,type:'unknown'});
    await cleaning;
  }
  releaseCleanup();let thrown;try{await run;}catch(error){thrown=error;}
  decoder.end();assert.equal(failures.length,1);await writer.stop();input.destroy();output.destroy();
  return {failure:failures[0],thrown};
}
test('browser primary and phase survive late005 during cleanup while cleanup failures remain fatal',{timeout:5000},async()=>{
  for(const cleanupFails of [false,true]){
    const {failure,thrown}=await precedence('browser-first',cleanupFails);
    assert.equal(failure.phase,'ABORT_PROOF');assert.equal(failure.cause,'ABORT_UNPROVED');
    assert.equal(failure.branch,'UNSPECIFIED');assert.equal(thrown.message,'PLAYGROUND-UI-ABORT-UNPROVED');
    assert.equal(failure.cleanupFailed,cleanupFails);assert.deepEqual(failure.cleanup,{intercept:false,context:cleanupFails,browser:false});
    assert.ok(!/private|capability|http:/.test(JSON.stringify(failure)));
  }
});
test('an earlier genuine stream failure keeps priority over subsequent browser cancellation',{timeout:4000},async()=>{
  for(const branch of ['FRAMING_002','INTERCEPT_007']){
    const {failure,thrown}=await precedence('stream-first',false,branch);
    assert.equal(failure.phase,'ABORT_PROOF');assert.equal(failure.cause,'TRANSPORT');
    assert.equal(failure.branch,branch);assert.equal(transportBranch(thrown),branch);
    assert.equal(failure.cleanupFailed,false);assert.ok(!JSON.stringify(failure).includes('STOPPED'));
  }
});
test('a primary-observation callback cannot replace the original failure or interrupt owned cleanup',async()=>{
  const error=new Error('private context details');error.name='TimeoutError';let observed=0,completed=0,closed=false;
  const browser={on(){},async newContext(){throw error;},async close(){closed=true;}};
  const configuration={version:1,type:'start',runId:'a'.repeat(32),origin:'http://127.0.0.1:12345',
    launchPath:'/examples/playground/restricted/',capability:'b'.repeat(64),planSha256:sha256(planBytes())};
  await assert.rejects(runInitialBrowserProbe({chromium:{async launch(){return browser;}},configuration,
    send(){assert.fail();},register(){assert.fail();},onFailure(original){observed++;assert.equal(original,error);throw Error('private observer details');},
    onCleanup(){completed++;throw Error('private completion details');}}),
  original=>original===error);
  assert.equal(observed,1);assert.equal(completed,1);assert.equal(closed,true);assert.equal(browserFailure(error).cause,'TIMEOUT');
});
