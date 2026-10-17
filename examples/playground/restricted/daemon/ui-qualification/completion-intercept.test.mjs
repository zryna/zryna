// Real loopback HTTP plus explicit Request fixtures; never real browser qualification.
import test from 'node:test';import assert from 'node:assert/strict';
import { EventEmitter } from 'node:events';import { createServer, request as httpRequest } from 'node:http';
import { interceptBrowser } from './browser-intercept.mjs';import { transportBranch } from './wire.mjs';
async function fixture() {
  const reservation = createServer();await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const origin = `http://127.0.0.1:${reservation.address().port}`;await new Promise(resolve => reservation.close(resolve));
  const context = new EventEmitter();let proxy;
  const names = ['content-type','cache-control','content-security-policy','x-content-type-options','referrer-policy','cross-origin-opener-policy','cross-origin-resource-policy'];
  proxy = await interceptBrowser(context, origin, record => {
    assert.equal(record.route, 'session');queueMicrotask(() => proxy.accept({version:1,type:'response',id:record.id,status:200,
      headers:names.map(name=>({name,value:'fixture'})),bodyBase64:Buffer.from('closed actual body').toString('base64')}));
  });
  const selected = {url:()=>origin+'/session',method:()=> 'GET',resourceType:()=> 'fetch',postDataBuffer:()=>null,
    redirectedFrom:()=>null,redirectedTo:()=>null,failure:()=>({errorText:'net::ERR_ABORTED'})};
  async function fetch() {
    context.emit('request', selected);
    return new Promise((resolve,reject)=>{const req=httpRequest(origin+'/session',{agent:false},response=>{
      context.emit('response',{request:()=>selected,status:()=>response.statusCode});response.resume();response.once('end',resolve);response.once('error',reject);
    });req.once('error',reject);req.end();});
  }
  return {context,proxy,selected,fetch};
}
test('only exact sameobject200 API ERR_ABORTED becomes provisional; laterfatal staysprimary',async()=>{
  const f=await fixture();try {
    const fence=f.proxy.beginPublication('STARTUP');await f.fetch();f.context.emit('requestfailed',f.selected);f.proxy.publish(fence);
    assert.equal(f.proxy.failureBranch(),'UNSPECIFIED');const partial=f.proxy.completionFailure();assert.equal(partial.firstProvisional.route,'session');
    assert.equal(partial.firstProvisional.status,200);assert.equal(partial.earliestUnresolved,null);
    f.proxy.invalidate();await assert.rejects(f.proxy.failures,error=>transportBranch(error)==='INTERCEPT_INVALIDATE');
    assert.throws(()=>f.proxy.seal(),error=>transportBranch(error)==='INTERCEPT_INVALIDATE');
  } finally {await f.proxy.stop();}
});
test('unknown Request and nonaborted network errors remainfatal despite actualsession200',async()=>{
  for(const known of [false,true]) {
    const f=await fixture();try {
      f.proxy.beginPublication('STARTUP');await f.fetch();
      if(known)f.selected.failure=()=>({errorText:'net::ERR_FAILED'});
      f.context.emit('requestfailed',known?f.selected:{...f.selected});
      await assert.rejects(f.proxy.failures,error=>transportBranch(error)===(known?'INTERCEPT_REQUEST_FAILED_ERR_FAILED':'INTERCEPT_REQUEST_FAILED_ERR_ABORTED'));
      assert.equal(f.proxy.completionFailure().firstProvisional,null);
    } finally {await f.proxy.stop();}
  }
});
