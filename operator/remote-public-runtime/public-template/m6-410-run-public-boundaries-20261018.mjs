// Fixed real HTTP boundary cohort; original launcher/native owner, no invented compiler replies.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,openSync,closeSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {performance} from 'node:perf_hooks';
import {createSourceLauncher} from '/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/m6-410-public-publication-state-20261018.mjs';
import {decodeCompilation} from '/workspace/zryna/examples/playground/restricted/protocol.mjs';
import {verifyReceipt} from '/workspace/zryna/examples/playground/restricted/executable.mjs';
const prep='/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';
const root='/workspace/scratch/M6_410_ACTUAL_PUBLISHED_HTTP_BOUNDARIES_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';mkdirSync(root,{mode:0o700});
const fd=openSync(root+'/RESULT.json','wx',0o400);let result={status:'failed'},launcher;
const sha=b=>createHash('sha256').update(b).digest('hex');
const save=(name,v)=>writeFileSync(root+'/'+name,Buffer.isBuffer(v)?v:JSON.stringify(v,null,2)+'\n',{flag:'wx',mode:0o400});
const rows=[];
try{
 assert.equal(process.version,'v22.22.1');
 const reviewed=JSON.parse(readFileSync(prep+'/reviewed-launcher.public.json'));
 const approval=JSON.parse(readFileSync(prep+'/approval.json'));
 const authority={compilerSha256:reviewed.compiler.sha256,toolkitSha256:reviewed.compositionSha256,policySha256:approval.policy_sha256,bindingTemplateSha256:reviewed.template.sha256};
 save('INTENT.json',{source:{commit:reviewed.source.commit,tree:reviewed.source.tree},authority,scriptSha256:sha(readFileSync(import.meta.filename)),
  scope:'Actual finite HTTP rejections and inclusive real compilations, reservation ownership, completion-lease expiry and actual Chrome scalar recovery. Expiry here is the2s completion lease, not a5s evaluator watchdog proof.'});
 launcher=await createSourceLauncher(reviewed);assert.equal(launcher.qualificationOnly,false);await launcher.launch();
 const url=new URL(launcher.takeLaunchUrl()),capability=url.hash.slice(1);
 const headers={authorization:'Bearer '+capability,origin:url.origin,'sec-fetch-site':'same-origin','content-type':'application/json'};
 async function bytes(response,maximum){let total=0;const chunks=[];for await(const block of response.body){total+=block.length;assert(total<=maximum);chunks.push(block)}return Buffer.concat(chunks);}
 async function fetchRaw(path,body,changes={}){return fetch(url.origin+path,{method:body===undefined?'GET':'POST',headers:{...headers,...changes},body,redirect:'error',signal:AbortSignal.timeout(12500)});}
 async function rejection(id,path,body,status=400,changes={}){
  const start=performance.now(),response=await fetchRaw(path,body,changes),raw=await bytes(response,1024);assert.equal(response.status,status,id);
  const value=JSON.parse(raw);assert.equal(typeof value.policyError,'string');assert(/^PLAYGROUND-[A-Z0-9-]+$/.test(value.policyError));assert(!value.policyError.startsWith('PLAYGROUND-HOST'),id);
  const row={id,status:response.status,policyError:value.policyError,bodyBytes:body===undefined?0:Buffer.byteLength(body),elapsedMs:performance.now()-start};assert(row.elapsedMs<5000);rows.push(row);save(id+'.json',row);return row;
 }
 async function finish(token){const r=await fetchRaw('/finish',JSON.stringify({token,cancelled:false}));assert.equal(r.status,200);assert.deepEqual(JSON.parse(await bytes(r,1024)),{finished:true});}
 async function compilation(id,revision,source,rawBody,hold=false){
  const start=performance.now(),response=await fetchRaw('/compile',rawBody??JSON.stringify({version:1,revision,source}));assert.equal(response.status,200,id);
  const frame=await bytes(response,1277956),token=response.headers.get('x-zryna-job');assert(/^[a-f0-9]{64}$/.test(token));
  const receipt=JSON.parse(Buffer.from(response.headers.get('x-zryna-receipt'),'base64'));
  if(!hold)await finish(token);
  const decoded=await decodeCompilation(new Uint8Array(frame),{revision,source,witSha256:reviewed.witSha256});assert.equal(decoded.metadata.status,'compiled');verifyReceipt(receipt,decoded.metadata,authority);assert.equal(receipt.frameSha256,sha(frame));
  save(id+'.frame',frame);save(id+'.receipt.json',receipt);
  const row={id,revision,sourceBytes:Buffer.byteLength(source),requestBytes:Buffer.byteLength(rawBody??JSON.stringify({version:1,revision,source})),frameBytes:frame.length,frameSha256:sha(frame),elapsedMs:performance.now()-start,finishAcknowledged:!hold};rows.push(row);
  return {metadata:decoded.metadata,token,frame,row};
 }
 const source='export function value(): i32 { return 42; }\n';
 const sourceRequest=revision=>JSON.stringify({version:1,revision,source});
 await rejection('wrong-capability','/session',undefined,400,{authorization:'Bearer '+(capability[0]==='0'?'1':'0')+capability.slice(1)});
 await rejection('foreign-site','/session',undefined,400,{'sec-fetch-site':'cross-site'});
 await rejection('foreign-origin','/compile',sourceRequest(1),400,{origin:'http://127.0.0.1:9'});
 await rejection('invalid-utf8','/compile',Buffer.from([255]));
 await rejection('duplicate-revision','/compile','{"version":1,"revision":1,"revision":2,"source":'+JSON.stringify(source)+'}');
 await rejection('noninteger-token','/compile','{"version":1,"revision":1.0,"source":'+JSON.stringify(source)+'}');
 await rejection('zero-revision','/compile',JSON.stringify({version:1,revision:0,source}));
 await rejection('unknown-path-selector','/compile',JSON.stringify({version:1,revision:1,source,path:'/etc/passwd'}));
 await rejection('source-first-extra','/compile',JSON.stringify({version:1,revision:1,source:source+' '.repeat(4097-Buffer.byteLength(source))}));
 await rejection('source-utf8-first-extra','/compile',JSON.stringify({version:1,revision:1,source:'é'.repeat(2049)}));
 const raw=sourceRequest(202);await rejection('request-first-extra','/compile',raw+' '.repeat(32769-Buffer.byteLength(raw)));
 const exactSource=source+' '.repeat(4096-Buffer.byteLength(source));const exact=await compilation('source-exact',201,exactSource);
 await assert.rejects(decodeCompilation(new Uint8Array(exact.frame),{revision:202,source:exactSource,witSha256:reviewed.witSha256}),/PLAYGROUND-CORRELATION/);rows.push({id:'actual-frame-wrong-revision',status:'rejected'});
 assert.throws(()=>verifyReceipt(JSON.parse(readFileSync(root+'/source-exact.receipt.json')),exact.metadata,{...authority,compilerSha256:'0'.repeat(64)}),/PLAYGROUND-/);rows.push({id:'actual-receipt-wrong-authority',status:'rejected'});
 await compilation('request-exact',202,source,raw+' '.repeat(32768-Buffer.byteLength(raw)));
 const reservation=await compilation('reservation-held',203,source,undefined,true);
 await rejection('second-admission','/compile',sourceRequest(204),409);
 await finish(reservation.token);reservation.row.finishAcknowledged=true;
 const expired=await compilation('completion-lease-expiry',205,source,undefined,true);
 await new Promise(resolve=>setTimeout(resolve,2300));
 await rejection('expired-finish','/finish',JSON.stringify({token:expired.token,cancelled:false}));
 // Tokens are deliberately excluded from all retained rows and files.
 const recovered=await compilation('recovery',206,source);
 const selection={revision:206,sourceSha256:recovered.metadata.sourceSha256,componentSha256:recovered.metadata.identity.componentSha256,logical:'value',args:[]};
 const response=await fetchRaw('/evaluate',JSON.stringify(selection));assert.equal(response.status,200);const evaluated=JSON.parse(await bytes(response,2048));await finish(evaluated.token);assert.equal(evaluated.observation.value,42);
 save('RECOVERY_OBSERVATION.json',{selection,observation:evaluated.observation});rows.push({id:'actual-chrome-recovery',value:42,finishAcknowledged:true});
 await launcher.stop();launcher=null;
 result={status:'actual-http-boundary-cohort-pass',rows,fullQualification:false,scope:'Actual carrier/budget/reservation/2s completion-lease/recovery observations only. Separate5s evaluator watchdog, kernel, session-isolation and publication obligations remain.'};
 console.log(JSON.stringify({status:result.status,rows:rows.length}));
}catch(error){result.failure={message:String(error.message).slice(0,1000),hostFailure:error.hostFailure??null};throw error;}
finally{if(launcher){try{await launcher.stop();result.launcherStopped=true}catch(error){result.cleanupFailure=String(error.message).slice(0,500);result.recovery=launcher.recovery();}}writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd);}
