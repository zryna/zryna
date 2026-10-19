// Actual published guide source, compile/reject/restore, real contained browser observations.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,openSync,closeSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createSourceLauncher} from '/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/m6-410-public-publication-state-20261018.mjs';
import {decodeCompilation} from '/workspace/zryna/examples/playground/restricted/protocol.mjs';
const root='/workspace/scratch/M6_410_ACTUAL_PUBLISHED_DOCUMENTATION_EXERCISE_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';mkdirSync(root,{mode:0o700});
const fd=openSync(root+'/RESULT.json','wx',0o400),sha=b=>createHash('sha256').update(b).digest('hex');
const save=(name,b)=>writeFileSync(root+'/'+name,Buffer.isBuffer(b)?b:JSON.stringify(b,null,2)+'\n',{flag:'wx',mode:0o400});
const prep='/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2',reviewed=JSON.parse(readFileSync(prep+'/reviewed-launcher.public.json'));
const path='spec/tooling/M6_REVIEWED_SOURCE_V2.md',guide=readFileSync('/workspace/zryna/'+path);assert(guide.equals(execFileSync('git',['show',reviewed.source.commit+':'+path],{cwd:'/workspace/zryna',maxBuffer:262144})));
const text=guide.toString('utf8').split('### Editable-source exercise\n')[1],match=/```zryna\n([\s\S]*?)```/.exec(text);assert(match);const source=match[1];assert.equal(source,'export function add(a: i32, b: i32): i32 {\n  return a + b;\n}\n');
save('GUIDE_INPUT.json',{sourcePath:path,sourceCommit:reviewed.source.commit,guideSha256:sha(guide),source,sourceSha256:sha(Buffer.from(source)),profile:'browser-component-v1',editedInvalidSource:source.replace('return a + b;','return missing;'),scriptSha256:sha(readFileSync(import.meta.filename))});
let result={status:'failed',startedAt:new Date().toISOString()},launcher;
try{
 launcher=await createSourceLauncher(reviewed);assert.equal(launcher.qualificationOnly,false);await launcher.launch();const url=new URL(launcher.takeLaunchUrl());
 const headers={authorization:'Bearer '+url.hash.slice(1),origin:url.origin,'sec-fetch-site':'same-origin','content-type':'application/json'};
 const request=async(path,body,maximum)=>{const r=await fetch(url.origin+path,{method:'POST',headers,body:JSON.stringify(body),redirect:'error',signal:AbortSignal.timeout(12500)});assert.equal(r.status,200);let count=0;const blocks=[];for await(const b of r.body){count+=b.length;assert(count<=maximum);blocks.push(b)}return {response:r,body:Buffer.concat(blocks)}};
 const finish=async token=>{const r=await request('/finish',{token,cancelled:false},1024);assert.deepEqual(JSON.parse(r.body),{finished:true})};
 const observations=[];
 for(const [offset,current]of [source,source.replace('return a + b;','return missing;'),source].entries()){
  const revision=1001+offset,r=await request('/compile',{version:1,revision,source:current},1277956);await finish(r.response.headers.get('x-zryna-job'));save('step-'+offset+'.frame',r.body);
  const decoded=await decodeCompilation(new Uint8Array(r.body),{revision,source:current,witSha256:reviewed.witSha256});
  if(offset===1){assert.equal(decoded.metadata.status,'rejected');assert(decoded.metadata.report.diagnostics.length>0);observations.push({revision,status:'rejected',diagnostics:decoded.metadata.report.diagnostics});continue}
  assert.equal(decoded.metadata.status,'compiled');const selection={revision,sourceSha256:decoded.metadata.sourceSha256,componentSha256:decoded.metadata.identity.componentSha256,logical:'add',args:[20,22]};
  const evaluated=JSON.parse((await request('/evaluate',selection,2048)).body);assert.equal(evaluated.observation.value,42);await finish(evaluated.token);observations.push({revision,status:'compiled',selection,observation:evaluated.observation});
 }
 await launcher.stop();launcher=null;save('OBSERVATIONS.json',observations);result={...result,status:'actual-published-guide-source-reject-restore-and-browser42-pass',finishedAt:new Date().toISOString(),sourceCommit:reviewed.source.commit,guideSha256:sha(guide),compositionSha256:reviewed.compositionSha256,compileCount:3,evaluationCount:2,restoredValue:42,normalNativeCleanupConfirmed:true};
 console.log(JSON.stringify({status:result.status}));
}catch(error){result.failure=String(error.message).slice(0,1500);throw error}
finally{if(launcher)await launcher.stop();writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd)}
