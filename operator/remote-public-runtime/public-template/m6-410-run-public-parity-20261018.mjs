// Actual original launcher -> service -> fixed native daemon compiler/browser -> recorder.
// No URL, capability, authorization header, or operation token is retained.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,openSync,closeSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
import {createSourceLauncher} from '/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/m6-410-public-publication-state-20261018.mjs';
import {createParityRecorder} from '/workspace/zryna/examples/playground/restricted/daemon/cli-parity/recorder.mjs';
import {createParityPlan,fixedCorpus} from '/workspace/zryna/examples/playground/restricted/daemon/cli-parity/plan.mjs';
import {decodeCompilation} from '/workspace/zryna/examples/playground/restricted/protocol.mjs';
const prep='/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';
const root='/workspace/scratch/M6_410_ACTUAL_PUBLISHED_HTTP_PARITY_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';mkdirSync(root,{mode:0o700});
const resultFd=openSync(root+'/RESULT.json','wx',0o400);let outcome={status:'failed'},launcher;
const sha=b=>createHash('sha256').update(b).digest('hex');
const put=(name,b)=>writeFileSync(root+'/'+name,b,{flag:'wx',mode:0o400});
const json=(name,v)=>put(name,JSON.stringify(v,null,2)+'\n');
const git=args=>execFileSync('git',args,{cwd:'/workspace/zryna',encoding:'utf8'}).trim();
const integration={commit:'b3be654c452ea51f2c136a75d2eb89995eb4bd66',tree:'3fa38d7b62b06a64ac5385d610be164f9b1961b8'};
const sourceProof=()=>{assert.equal(git(['rev-parse','HEAD']),integration.commit);assert.equal(git(['rev-parse','HEAD^{tree}']),integration.tree);assert.equal(git(['status','--porcelain']),'');return integration};
const stages=[];
try{
 assert.equal(process.version,'v22.22.1');json('SOURCE_BEFORE.json',sourceProof());
 const reviewed=JSON.parse(readFileSync(prep+'/reviewed-launcher.public.json'));
 const approval=JSON.parse(readFileSync(prep+'/approval.json'));
 assert.equal(sha(readFileSync(prep+'/approval.json')),reviewed.approval.sha256);
 const authority={compilerSha256:reviewed.compiler.sha256,toolkitSha256:reviewed.compositionSha256,policySha256:approval.policy_sha256,bindingTemplateSha256:reviewed.template.sha256};
 json('INTENT.json',{integration,reviewedLauncherSha256:sha(readFileSync(prep+'/reviewed-launcher.public.json')),authority,
  scriptSha256:sha(readFileSync(import.meta.filename)),scope:'Actual nine compiler frames and ten contained Chrome observations compared against19 retained independent actual CLI captures. Actual authenticated published compiler via retained original opaque capability; final23evidence remainsseparate.'});
 const recorder=createParityRecorder({authority,witSha256:reviewed.witSha256});
 const selected=JSON.parse(readFileSync('/workspace/scratch/M6_410_CLI_COMPARISON_SELECTION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN1/SELECTED19.json'));assert.deepEqual(selected.integration,integration);assert.equal(selected.rows.length,19);
 for(const row of selected.rows){
  const stdout=readFileSync(row.captureRoot+'/'+row.operationId+'-cli.stdout'),stderr=readFileSync(row.captureRoot+'/'+row.operationId+'-cli.stderr');
  assert.equal(sha(stdout),row.stdoutSha256);assert.equal(sha(stderr),row.stderrSha256);
  recorder.cli(row.operationId,{exitCode:row.exitCode,stdout,stderr});
 }
 launcher=await createSourceLauncher(reviewed);assert.equal(launcher.qualificationOnly,false);
 const started=await launcher.launch(),url=new URL(launcher.takeLaunchUrl());assert.equal(url.origin,started.origin);
 const capability=url.hash.slice(1);assert(/^[a-f0-9]{64}$/.test(capability));
 const headers={authorization:'Bearer '+capability,origin:url.origin,'sec-fetch-site':'same-origin','content-type':'application/json'};
 async function request(path,body,maximum,timeout){
  const response=await fetch(url.origin+path,{method:body===undefined?'GET':'POST',headers,body:body===undefined?undefined:JSON.stringify(body),redirect:'error',signal:AbortSignal.timeout(timeout)});
  const chunks=[];let count=0;
  for await(const block of response.body){count+=block.length;assert(count<=maximum);chunks.push(block)}
  const data=Buffer.concat(chunks);assert.equal(response.status,200,JSON.stringify({path,status:response.status,error:count<512?data.toString():'bounded'}));
  return {response,data};
 }
 async function finish(token){assert(/^[a-f0-9]{64}$/.test(token));const done=await request('/finish',{token,cancelled:false},1024,2500);assert.deepEqual(JSON.parse(done.data),{finished:true});}
 const session=JSON.parse((await request('/session',undefined,98304,2500)).data);assert.deepEqual(session.authority,authority);assert.equal(session.witSha256,reviewed.witSha256);assert.equal(session.evaluationMode,'supervised-host');
 json('SESSION_IDENTITY.json',{version:session.version,evaluationMode:session.evaluationMode,authority:session.authority,witSha256:session.witSha256,templateSha256:sha(Buffer.from(session.template,'base64'))});
 const plan=createParityPlan();
 for(const [index,entry]of fixedCorpus.entries()){
  const revision=index+1,startedAt=new Date().toISOString(),sourceRequest={version:1,revision,source:entry.source};
  const captured=await request('/compile',sourceRequest,1277956,12500);
  const token=captured.response.headers.get('x-zryna-job');
  const receipt=JSON.parse(Buffer.from(captured.response.headers.get('x-zryna-receipt'),'base64'));
  await finish(token);
  put(entry.id+'.frame',captured.data);json(entry.id+'.request.json',sourceRequest);json(entry.id+'.receipt.json',receipt);
  await recorder.compilation(entry.id,{revision,frameBytes:new Uint8Array(captured.data),receipt});
  const decoded=await decodeCompilation(new Uint8Array(captured.data),{revision,source:entry.source,witSha256:reviewed.witSha256});
  stages.push({id:'build-'+entry.id,startedAt,finishedAt:new Date().toISOString(),status:decoded.metadata.status,frameSha256:sha(captured.data),finishAcknowledged:true});
  for(const operation of plan.operations.filter(o=>o.corpusId===entry.id&&o.kind==='run')){
   const startedAt=new Date().toISOString();
   const selection={revision,sourceSha256:decoded.metadata.sourceSha256,componentSha256:decoded.metadata.identity.componentSha256,logical:operation.logical,args:operation.args};
   const evaluated=JSON.parse((await request('/evaluate',selection,2048,7500)).data);await finish(evaluated.token);
   recorder.observation(operation.id,{selection,observation:evaluated.observation});
   json(operation.id+'.observation.json',{selection,observation:evaluated.observation});
   stages.push({id:operation.id,startedAt,finishedAt:new Date().toISOString(),observedValue:evaluated.observation.value,finishAcknowledged:true});
  }
 }
 const comparison=recorder.finish();assert.equal(comparison.comparedOperations,19);json('COMPARISON.json',comparison);
 await launcher.stop();launcher=null;json('SOURCE_AFTER.json',sourceProof());
 outcome={status:'actual-nineteen-cli-http-browser-comparison-matched',integration,authority,stages,comparisonSha256:sha(readFileSync(root+'/COMPARISON.json')),
  launcherStopped:true,fullQualification:false,scope:'Original native daemon owner success and paired service finishes observed. Comparison is not final runtime/publication attestation; separate owner/kernel/source/publication review required.'};
 console.log(JSON.stringify({status:outcome.status,operations:stages.length}));
}catch(error){outcome.failure={message:String(error.message).slice(0,1000),hostFailure:error.hostFailure??null};throw error;}
finally{
 if(launcher){try{await launcher.stop();outcome.failureCleanupConfirmed=true}catch(error){outcome.cleanupFailure=String(error.message).slice(0,500);outcome.recovery=launcher.recovery();}}
 writeFileSync(resultFd,JSON.stringify(outcome,null,2)+'\n');closeSync(resultFd);
}
