// Actual fixed original browser qualification owner; secret transport stays in memory.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,openSync,closeSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createSourceLauncher} from '/workspace/scratch/M6_410_PUBLIC_COORDINATOR_OPERATORS_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2/m6-410-public-publication-state-20261018.mjs';
import {qualifyInitialUi} from '/workspace/zryna/examples/playground/restricted/daemon/ui-qualification/owner.mjs';
import {routes} from '/workspace/zryna/examples/playground/restricted/daemon/ui-qualification/routes.mjs';
const root='/workspace/scratch/M6_410_ACTUAL_PUBLISHED_UI_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';mkdirSync(root,{mode:0o700});
const fd=openSync(root+'/RESULT.json','wx',0o400);let result={status:'failed'},launcher;
const prep='/workspace/scratch/M6_410_PUBLIC_LAUNCHER_PREPARATION_b3be654c452ea51f2c136a75d2eb89995eb4bd66_RUN2';
const sha=b=>createHash('sha256').update(b).digest('hex');
const save=(name,value)=>writeFileSync(root+'/'+name,JSON.stringify(value,null,2)+'\n',{flag:'wx',mode:0o400});
try{
 assert.equal(process.version,'v22.22.1');
 const reviewed=JSON.parse(readFileSync(prep+'/reviewed-launcher.public.json'));
 const approval=JSON.parse(readFileSync(prep+'/ui-approval.json'));
 const tools=JSON.parse(readFileSync('/workspace/scratch/M6_410_HOST_TOOLS_20261018_RUN2/POLICY_CAPTURED_CLOSURE.json'));
 const resourcePins=new Map();
 for(const [name,route]of Object.entries(routes))if(name.startsWith('asset:')||name==='index'){
  const path='examples/playground/restricted/'+(name==='index'?'index.html':name.slice(6));
  const bytes=readFileSync('/workspace/zryna/'+path);
  assert(bytes.equals(execFileSync('git',['show','b3be654c452ea51f2c136a75d2eb89995eb4bd66:'+path],{cwd:'/workspace/zryna'})));
  resourcePins.set(route.path,{bytes:bytes.length,sha256:sha(bytes)});
 }
 save('INTENT.json',{integration:{commit:reviewed.source.commit,tree:reviewed.source.tree},
  compositionSha256:reviewed.compositionSha256,uiApprovalSha256:sha(readFileSync(prep+'/ui-approval.json')),
  scriptSha256:sha(readFileSync(import.meta.filename)),sourceResourcePins:Object.fromEntries(resourcePins),
  scope:'Original trusted300second real UI owner; original compiler10s/evaluator5s/cleanup2s remain unchanged. No capability or raw headers retained; separate resource/session-isolation/full qualification gates remain.'});
 launcher=await createSourceLauncher(reviewed);assert.equal(launcher.qualificationOnly,false);
 const observation=await qualifyInitialUi({tools,approval,launcher,resourcePins});
 save('OBSERVATION.json',observation);launcher=null;
 assert.equal(observation.containerCleanupConfirmed,true);assert.equal(observation.fullQualification,false);
 result={status:'actual-initial-abort-corpus-ax-ui-pass',observationSha256:sha(readFileSync(root+'/OBSERVATION.json')),
  fullQualification:false,scope:'Original real UI cumulative probe only. It does not replace separate resource, watchdog, paired-session isolation or compiler publication evidence.'};
 console.log(JSON.stringify(result));
}catch(error){
 result.failure={message:String(error.message).slice(0,500),recovery:error.recovery??null,transport:error.transport??null};
 throw error;
}finally{
 if(launcher){try{await launcher.stop();result.launcherStopped=true}catch(error){result.cleanupFailure=String(error.message).slice(0,500);result.recovery=launcher.recovery();}}
 writeFileSync(fd,JSON.stringify(result,null,2)+'\n');closeSync(fd);
}
