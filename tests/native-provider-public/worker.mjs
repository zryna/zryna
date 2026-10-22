import assert from 'node:assert/strict';
import {spawn} from 'node:child_process';
import {resolve} from 'node:path';
import {canonical,exact} from './registry.mjs';
import {strictJson} from './json.mjs';

const MAX_FRAME=16*1024*1024,MAX_STDERR=65536;
export function expectedHandshake(protocol,provider){
 const capabilities={module_resolution:false,semantic_diagnostics:false};
 if(protocol>=3)capabilities.control_flow_v1=true;
 if(protocol===4)capabilities.data_ownership_syntax_v1=true;
 return {provider:provider==='native'?'zryna-native':'typescript-6',provider_version:provider==='native'?'0.2.3':'6.0.3',protocol_version:protocol,capabilities};
}
// A source rejection is an ordinary observable result, not verified syntax authority.
export function verifyAnalysisFrame(frame) {
 exact(frame,Object.hasOwn(frame,'error')?['id','error']:['id','result'],'closed worker analysis frame');
 assert.equal(frame.id,2,'original correlated analysis id');
 if(Object.hasOwn(frame,'error')) {
  exact(frame.error,['code','message'],'closed provider rejection');
  for(const [field,limit]of [['code',128],['message',16384]]) {
   const value=frame.error[field];
   assert(typeof value==='string'&&value.isWellFormed()&&Buffer.byteLength(value)>0&&Buffer.byteLength(value)<=limit,'bounded well-formed provider '+field);
  }
 } else assert(frame.result&&typeof frame.result==='object'&&!Array.isArray(frame.result),'candidate document object');
 return frame;
}
export function compareAnalysisFrames(native,bootstrap) {
 verifyAnalysisFrame(native);verifyAnalysisFrame(bootstrap);
 assert.deepEqual(native,bootstrap,'whole correlated analysis frame, including ordered candidate spans/diagnostics or exact provider rejection');
}
export async function runWorker({program,args,cwd,protocol,provider,files,timeoutMs=30000}){
 assert([2,3,4].includes(protocol)&&['native','bootstrap'].includes(provider));
 assert(program===resolve(program)&&Number.isSafeInteger(timeoutMs)&&timeoutMs>0&&timeoutMs<=30000);
 const handshake=canonical({id:1,method:'handshake'}),analyze=canonical({id:2,method:'analyze',params:{schema_version:protocol,files}});
 assert(analyze.length<=72*1024*1024,'original request bound');
 const env={...process.env};delete env.NODE_OPTIONS;delete env.NODE_PATH;delete env.ZRYNA_TEST_REQUEST_BYTES;
 const startedAt=new Date().toISOString(),stdout=[],stderr=[],frames=[];let pending=Buffer.alloc(0),outputBytes=0,errorBytes=0,sourceSent=false,failure;
 return await new Promise((accept,reject)=>{
  const child=spawn(program,args,{cwd,env,stdio:['pipe','pipe','pipe'],windowsHide:true});
  const stop=error=>{failure??=error;child.stdin.destroy();child.kill();};
  const deadline=setTimeout(()=>stop(new Error('worker deadline exceeded')),timeoutMs);
  child.once('error',stop);child.stdin.on('error',error=>{if(!failure)stop(error);});
  child.stderr.on('data',bytes=>{errorBytes+=bytes.length;if(errorBytes>MAX_STDERR){stop(new Error('worker stderr bound'));return;}stderr.push(bytes);});
  child.stdout.on('data',bytes=>{
   outputBytes+=bytes.length;if(outputBytes>2*MAX_FRAME){stop(new Error('worker output bound'));return;}stdout.push(bytes);pending=Buffer.concat([pending,bytes]);
   for(;;){const end=pending.indexOf(10);if(end<0){if(pending.length>MAX_FRAME)stop(new Error('worker frame bound'));return;}const line=pending.subarray(0,end);pending=pending.subarray(end+1);
    try{
     assert(line.length>0&&line.length<=MAX_FRAME,'bounded nonempty frame');const frame=strictJson(line,MAX_FRAME);frames.push(frame);
     if(frames.length===1){exact(frame,['id','result'],'exact worker handshake success frame');assert.equal(frame.id,1);assert.deepEqual(frame.result,expectedHandshake(protocol,provider),'exact admitted handshake');sourceSent=true;child.stdin.write(analyze);}
     else if(frames.length===2){verifyAnalysisFrame(frame);child.stdin.end();}
     else throw new Error('extra worker response');
    }catch(error){stop(error);}
   }
  });
  child.once('close',(exitCode,signal)=>{
   clearTimeout(deadline);const result={program,args,cwd,protocol,provider,startedAt,finishedAt:new Date().toISOString(),exitCode,signal,sourceSent,stdout:Buffer.concat(stdout),stderr:Buffer.concat(stderr),frames,ownerExitObserved:Number.isSafeInteger(child.pid)&&child.pid>0,descendantCleanupClaimed:false};
   try{assert(!failure,failure?.message);assert.equal(exitCode,0,'actual worker exited successfully');assert.equal(signal,null);assert.equal(pending.length,0,'complete final framing');assert.equal(frames.length,2,'complete handshake and analysis');assert.equal(errorBytes,0,'worker stderr empty');accept(result);}catch(error){error.capture=result;reject(error);}
  });
  child.stdin.write(handshake);
 });
}
