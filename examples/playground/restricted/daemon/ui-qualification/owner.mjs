// Distinct trusted test owner. Never a guest executor or a generic Engine client.
import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { captureOwnerTools } from './owner-tools.mjs';
import { createOwnerControl } from './owner-control.mjs';
import { createHostSession } from './host-session.mjs';
import { createFrameDecoder,createFrameWriter } from './framing.mjs';
import { createEngineDemux } from './engine-stream.mjs';
import { planBytes } from './plan.mjs';
import { fail,sha256 } from './wire.mjs';
import { createOwnerCompletion } from './owner-completion.mjs';

const admitted=new WeakSet();
export async function qualifyInitialUi({tools,approval,launcher,resourcePins,signal}) {
  const started=performance.now(),selected=structuredClone(approval);
  const deadlineNs=(process.hrtime.bigint()+300000000000n).toString();
  if(selected.ui_plan_sha256!==sha256(planBytes()) || admitted.has(launcher))fail();
  admitted.add(launcher);
  const captured=captureOwnerTools(tools);
  let child,session,receipt=null,readyRecord=null,reservedRecord=null,streamEnded=false,closedStatus=null;
  let rejectReady,resolveReady,resolveClosed,resolveChildClose,rejectFailure;
  const ready=new Promise((resolve,reject)=>{resolveReady=resolve;rejectReady=reject;});ready.catch(()=>{});
  const closed=new Promise(resolve=>{resolveClosed=resolve;});
  const childClosed=new Promise(resolve=>{resolveChildClose=resolve;});
  const failure=new Promise((_,reject)=>{rejectFailure=reject;});failure.catch(()=>{});
  let primary=null,decoder,demux,writer,timer,stderrBytes=0;
  const broken=error=>{primary??=error;rejectReady(error);rejectFailure(error);};
  const guarded=promise=>Promise.race([promise,failure]);
  const aborted=()=>broken(new Error('PLAYGROUND-UI-OWNER-ABORTED'));
  let completion;
  try {
    child=spawn(captured.interpreter,['-I','-S','-B','-u',captured.entry],{cwd:captured.root,
      env:{LANG:'C.UTF-8',HOME:'/nonexistent',TZ:'UTC'},stdio:['pipe','pipe','pipe','pipe'],shell:false});
    writer=createFrameWriter(child.stdin);
    completion=createOwnerCompletion({child,closed,childClosed,captured,launcher,
      session:()=>session,receipt:()=>receipt,readyRecord:()=>readyRecord,reservedRecord:()=>reservedRecord});
    const control=createOwnerControl({onReserved(value){reservedRecord=value;},onReady(value){
      const pin=selected.material;
      if(value.audit.rootSha256!==pin.root_sha256 || value.audit.bytes!==pin.bytes || value.audit.files!==pin.files)fail();
      readyRecord=value;resolveReady(value);
    },onClosed(value){
      if(readyRecord && (value.nonce!==readyRecord.nonce||value.containerId!==readyRecord.containerId))fail();
      receipt=value;resolveClosed(value);
      if(!readyRecord)broken(new Error('PLAYGROUND-UI-OWNER-AUDIT'));
    }});
    child.stdio[3].on('data',chunk=>{try{control.push(chunk);}catch(error){broken(error);}});
    child.stdio[3].on('end',()=>{try{control.end();}catch(error){broken(error);}});
    child.stdio[3].on('error',broken);child.stdin.on('error',broken);child.on('error',broken);
    child.stderr.on('data',chunk=>{stderrBytes+=chunk.length;if(stderrBytes>16384)broken(new Error('PLAYGROUND-UI-OWNER-STDERR'));});
    child.stderr.on('error',broken);child.stdout.on('error',broken);
    demux=createEngineDemux(chunk=>{if(!decoder)fail();decoder.push(chunk);},()=>{});
    child.stdout.on('data',chunk=>{try{demux.push(chunk);}catch(error){broken(error);}});
    child.stdout.on('end',()=>{
      try{demux.end();if(decoder)decoder.end();session?.eof();streamEnded=true;}catch(error){broken(error);}
    });
    child.once('close',(code,termination)=>{
      closedStatus={code,termination};resolveChildClose(closedStatus);
      if(code!==0 || termination || !receipt)broken(new Error('PLAYGROUND-UI-OWNER-CLOSE'));
    });
    timer=setTimeout(()=>broken(new Error('PLAYGROUND-UI-OWNER-DEADLINE')),Math.max(1,300000-(performance.now()-started)));
    signal?.addEventListener('abort',aborted,{once:true});if(signal?.aborted)aborted();
    if(primary)throw primary;
    const initial={version:1,approval:selected,deadlineNs};
    await guarded(new Promise((resolve,reject)=>child.stdin.write(Buffer.from(JSON.stringify(initial)+'\n'),error=>error?reject(error):resolve())));
    await guarded(ready);
    await guarded(launcher.launch());const url=new URL(launcher.takeLaunchUrl());
    session=createHostSession({configuration:{version:1,type:'start',runId:randomBytes(16).toString('hex'),
      origin:url.origin,launchPath:url.pathname,capability:url.hash.slice(1),planSha256:selected.ui_plan_sha256},
      send:record=>writer.write(record),resourcePins});
    decoder=createFrameDecoder(record=>session.accept(record));await guarded(session.begin());
    const observation=await guarded(Promise.race([session.result,closed.then(async()=>{
      // The control pipe may close first; drain application stdout before discarding a terminal failure frame.
      await childClosed;throw new Error('PLAYGROUND-UI-EARLY-OWNER-CLOSE');
    })]));
    const outcome=await completion.stop();
    if(primary)throw primary;
    if(!streamEnded || receipt.failure || !receipt.kernelReapProven || receipt.wait?.StatusCode!==0 || closedStatus?.code!==0)fail();
    return Object.freeze({...observation,containerCleanupConfirmed:true,owner:outcome,fullQualification:false});
  }catch(error){
    primary??=error;
    try{session?.failureEvidence(primary);}catch{/* Diagnostics must never prevent owned cleanup. */}
    let cleanup=null;try{if(completion)await completion.stop();else if(!child)captured.cleanup();}catch(error){cleanup=error;}
    const failure=new AggregateError([primary,...(cleanup?[cleanup]:[])],'PLAYGROUND-UI-QUALIFICATION-FAILED');
    failure.recovery=completion?.recovery()??{helperSnapshot:captured.root,ownershipUnconfirmed:Boolean(child)};
    try{failure.transport=session?.failureEvidence(primary)??null;}catch{failure.transport=null;}
    throw failure;
  }finally{clearTimeout(timer);signal?.removeEventListener('abort',aborted);}
}
