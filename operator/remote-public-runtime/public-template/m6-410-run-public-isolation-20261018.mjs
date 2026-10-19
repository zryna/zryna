// Two separately owned original source sessions and actual contained Chrome scalar runs.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,openSync,closeSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {createSourceLauncher} from '/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/m6-410-public-publication-state-20261018.mjs';
import {decodeCompilation} from '/workspace/zryna/examples/playground/restricted/protocol.mjs';
const root='/workspace/scratch/M6_410_ACTUAL_PUBLISHED_SESSION_ISOLATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';mkdirSync(root,{mode:0o700});
const fd=openSync(root+'/RESULT.json','wx',0o400);let result={status:'failed'};const owned=[];
const sha=b=>createHash('sha256').update(b).digest('hex');
const save=(name,v)=>writeFileSync(root+'/'+name,Buffer.isBuffer(v)?v:JSON.stringify(v,null,2)+'\n',{flag:'wx',mode:0o400});
try{
 assert.equal(process.version,'v22.22.1');
 const reviewed=JSON.parse(readFileSync('/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/reviewed-launcher.public.json'));
 save('INTENT.json',{source:{commit:reviewed.source.commit,tree:reviewed.source.tree},compositionSha256:reviewed.compositionSha256,
  scriptSha256:sha(readFileSync(import.meta.filename)),scope:'Two independent original source-launcher sessions. Cross-capability and cross-source rejection, followed by original contained Chrome42/7 executions. Operations are sequential. Does not claim two concurrent visible UI browser hosts.'});
 async function open(name){
  const launcher=await createSourceLauncher(reviewed);owned.push(launcher);await launcher.launch();const url=new URL(launcher.takeLaunchUrl());
  return {name,launcher,url,capability:url.hash.slice(1)};
 }
 const a=await open('A'),b=await open('B');assert.notEqual(a.url.origin,b.url.origin);assert.notEqual(a.capability,b.capability);
 async function query(session,path,body,expected=200,capability=session.capability){
  const response=await fetch(session.url.origin+path,{method:body===undefined?'GET':'POST',
   headers:{authorization:'Bearer '+capability,origin:session.url.origin,'sec-fetch-site':'same-origin','content-type':'application/json'},
   body:body===undefined?undefined:JSON.stringify(body),redirect:'error',signal:AbortSignal.timeout(12500)});
  const chunks=[];let size=0;for await(const block of response.body){size+=block.length;assert(size<=1277956);chunks.push(block)}
  const raw=Buffer.concat(chunks);assert.equal(response.status,expected,session.name+path);
  return {response,raw};
 }
 async function finish(session,token){const done=await query(session,'/finish',{token,cancelled:false});assert.deepEqual(JSON.parse(done.raw),{finished:true});}
 for(const [receiver,sender]of [[a,b],[b,a]]){
  const rejected=JSON.parse((await query(receiver,'/session',undefined,400,sender.capability)).raw);
  assert.equal(rejected.policyError,'PLAYGROUND-CAPABILITY');save(receiver.name+'-FOREIGN_CAPABILITY.json',{status:400,policyError:rejected.policyError});
 }
 async function compile(session,revision,value){
  const source='export function value(): i32 { return '+value+'; }\n';const captured=await query(session,'/compile',{version:1,revision,source});
  const token=captured.response.headers.get('x-zryna-job');await finish(session,token);
  const decoded=await decodeCompilation(new Uint8Array(captured.raw),{revision,source,witSha256:reviewed.witSha256});assert.equal(decoded.metadata.status,'compiled');
  save(session.name+'-'+revision+'.frame',captured.raw);save(session.name+'-'+revision+'.REQUEST.json',{version:1,revision,source});
  return {revision,sourceSha256:decoded.metadata.sourceSha256,componentSha256:decoded.metadata.identity.componentSha256,logical:'value',args:[]};
 }
 const selectedA=await compile(a,1,42),selectedB=await compile(b,1,7);
 assert.notEqual(selectedA.sourceSha256,selectedB.sourceSha256);assert.notEqual(selectedA.componentSha256,selectedB.componentSha256);
 const cross=JSON.parse((await query(b,'/evaluate',selectedA,400)).raw);assert.equal(cross.policyError,'PLAYGROUND-STALE');
 save('CROSS_SOURCE.json',{status:400,policyError:cross.policyError,sourceIdentitiesDistinct:true,componentIdentitiesDistinct:true});
 async function evaluate(session,selection,expected){
  const actual=JSON.parse((await query(session,'/evaluate',selection)).raw);await finish(session,actual.token);assert.equal(actual.observation.value,expected);
  save(session.name+'-OBSERVATION.json',{selection,observation:actual.observation,finishAcknowledged:true});
 }
 await evaluate(a,selectedA,42);
 const recoveredB=await compile(b,2,7);await evaluate(b,recoveredB,7);
 for(const launcher of owned)await launcher.stop();owned.length=0;
 result={status:'actual-source-session-and-contained-browser-isolation-pass',sessions:2,crossCapabilityRejections:2,crossSourceRejections:1,actualChromeValues:[42,7],allLaunchersStopped:true,fullQualification:false,
  scope:'Independent source-session and fresh contained browser evaluation observations. No claim of parallel visible UI hosts or complete browser/publication qualification.'};
 console.log(JSON.stringify(result));
}catch(error){result.failure={message:String(error.message).slice(0,1000),hostFailure:error.hostFailure??null};throw error;}
finally{
 const failures=[];for(const launcher of owned){try{await launcher.stop()}catch(error){failures.push({message:String(error.message).slice(0,500),recovery:launcher.recovery()})}}
 if(failures.length)result.cleanupFailures=failures;writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd);
}
