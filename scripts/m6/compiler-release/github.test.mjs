import test from 'node:test';import assert from 'node:assert/strict';
import { github,readResponse } from './github.mjs';
const response=(value,status=200,headers={})=>new Response(typeof value==='string'?value:JSON.stringify(value),{status,headers});
test('GitHub fixed asset redirect never forwards credential to storage',async()=>{
  const calls=[],api=github({token:'fixture-token-never-real',fetchImpl:async(url,options)=>{
    calls.push({url,options});return calls.length===1?response('',302,{location:'https://release-assets.githubusercontent.com/fixture'}):response('bytes');
  }});
  assert.equal((await api.asset(123)).toString(),'bytes');
  assert.equal(calls[0].options.headers.Authorization,'Bearer fixture-token-never-real');
  assert.equal(calls[1].options.headers.Authorization,undefined);
  assert.equal(api.captures.length,1);
});
for(const [name,url]of [['foreign','https://example.invalid/asset'],['http','http://release-assets.githubusercontent.com/x'],
  ['credential','https://user@release-assets.githubusercontent.com/x'],['port','https://release-assets.githubusercontent.com:4433/x']])
  test(`asset acquisition rejects ${name} redirect`,async()=>{
    const api=github({token:'fixture',fetchImpl:async()=>response('',302,{location:url})});await assert.rejects(api.asset(123));
  });
test('fixed repository API rejects foreign repository and invalid ID before request',async()=>{
  let calls=0;const api=github({token:'fixture',fetchImpl:async()=>{calls++;return response({});}});
  await assert.rejects(api.request('/repos/other/repo/releases'));
  assert.throws(()=>api.asset(0));assert.equal(calls,0);
});
for(const [name,res,maximum]of [
  ['encoding',()=>response('abc',200,{'content-encoding':'gzip'}),10],
  ['declared oversize',()=>response('abc',200,{'content-length':'100'}),10],
  ['stream oversize',()=>response('abc'),2],
  ['truncated declaration',()=>response('abc',200,{'content-length':'4'}),10],
  ['invalid declaration',()=>response('abc',200,{'content-length':'03'}),10],
])test(`bounded API bytes rejects ${name}`,()=>assert.rejects(readResponse(res(),maximum)));
test('API read raw bytes retain exact status/digest without credential recording',async()=>{
  const api=github({token:'fixture-secret',fetchImpl:async()=>response({id:1})});
  assert.deepEqual(await api.request('/repos/zryna/zryna/releases/1'),{id:1});
  assert.equal(JSON.stringify(api.captures).includes('fixture-secret'),false);
  assert.equal(api.captures[0].data.toString(),'{"id":1}');
});
