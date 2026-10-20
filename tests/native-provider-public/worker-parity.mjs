import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
import {canonical,readPinned,sha256} from './registry.mjs';
import {runWorker} from './worker.mjs';

export async function runWorkerParity({registry,root,cli,node,output}){
 mkdirSync(output,{mode:0o700});const results=[];
 for(const [index,item]of registry.sources.entries()){
  const file={path:item.source.path,text:readPinned(root,item.source).toString('utf8')},captures={};
  for(const provider of ['native','bootstrap']){
   const program=provider==='native'?cli:node;
   const args=provider==='native'?['__native-frontend','--protocol',String(item.protocol)]:[join(root,'adapters/typescript-6/src',item.protocol===2?'worker.mjs':`worker-v${item.protocol}.mjs`)];
   const stem=String(index).padStart(3,'0')+'-'+provider;
   try{captures[provider]=await runWorker({program,args,cwd:root,protocol:item.protocol,provider,files:[file]});}
   catch(error){if(error.capture)persist(stem,error.capture);results.push({id:item.id,status:'failed',provider,error:String(error.message)});break;}
   persist(stem,captures[provider]);
  }
  if(captures.native&&captures.bootstrap){
   try{assert.deepEqual(captures.native.frames[1].result,captures.bootstrap.frames[1].result,'whole raw snapshot, including exact ordered diagnostics and byte spans');results.push({id:item.id,status:'passed',protocol:item.protocol,snapshotSha256:sha256(canonical(captures.native.frames[1].result)),syntaxVerification:item.syntaxVerification,source:item.source});}
   catch(error){results.push({id:item.id,status:'failed',error:String(error.message)});}
  }
 }
 assert.equal(results.length,114,'all frozen source cases attempted');
 const failed=results.filter(r=>r.status==='failed').length,result={status:failed?'failed':'passed',executed:results.length,failed,skipped:0,results,scope:'Whole worker candidate comparisons only. Original syntax/IR verifier and CLI/installed/runtime/full-CI lanes remain separate required obligations.'};
 writeFileSync(join(output,'RESULT.json'),canonical(result),{flag:'wx',mode:0o600});return result;
 function persist(stem,capture){
  writeFileSync(join(output,stem+'.stdout'),capture.stdout,{flag:'wx',mode:0o600});writeFileSync(join(output,stem+'.stderr'),capture.stderr,{flag:'wx',mode:0o600});
  const {stdout,stderr,...command}=capture;writeFileSync(join(output,stem+'.json'),canonical({...command,stdout:{bytes:stdout.length,sha256:sha256(stdout)},stderr:{bytes:stderr.length,sha256:sha256(stderr)}}),{flag:'wx',mode:0o600});
 }
}
