// Sanitized separate owner FD; never receives the browser capability or raw HTTP frames.
import { exact, fail, hash } from './wire.mjs';
export function validateOwnerReceipt(value) {
  const required=['format','version','qualificationOnly','name','nonce','containerId','audit','kernel','inspect',
    'wait','removed','dockerRemoved','kernelReapProven','failure','cleanupFailure','ownershipUnconfirmed'];
  const optional=['imageBefore','volumeBefore','volumeAfterAudit','volumeAfterCleanup','imageAfter','channelTerminal','reap',
    'initialIdentity','kernelIdentityFailure','procAudit','acquisition'];
  if(!value || typeof value!=='object' || Array.isArray(value) ||
    !required.every(key=>Object.hasOwn(value,key)) || Object.keys(value).some(key=>!required.includes(key)&&!optional.includes(key)))fail();
  if(value.format!=='zryna.playground-ui-owner-observation.v1' || value.version!==1 || value.qualificationOnly!==true ||
    !/^[a-f0-9]{32}$/.test(value.nonce) || value.name!=='zryna410-ui-'+value.nonce ||
    value.containerId!==null&&!hash(value.containerId) || !Array.isArray(value.kernel) || value.kernel.length>2 ||
    !Array.isArray(value.inspect) || value.inspect.length>32 || typeof value.removed!=='boolean' ||
    typeof value.ownershipUnconfirmed!=='boolean' || typeof value.dockerRemoved!=='boolean' ||
    typeof value.kernelReapProven!=='boolean')fail();
  for(const key of ['failure','cleanupFailure'])if(value[key]!==null &&
    (typeof value[key]!=='string'||!/^[A-Za-z0-9 ._-]{1,128}$/.test(value[key])))fail();
  if(value.wait!==null){
    if(!value.wait || typeof value.wait!=='object' || Array.isArray(value.wait) ||
      Object.keys(value.wait).some(key=>!['StatusCode','Error'].includes(key)) ||
      !Number.isInteger(value.wait.StatusCode) || value.wait.StatusCode<0 || value.wait.StatusCode>255 || value.wait.Error)fail();
  }
  if(value.removed && (value.ownershipUnconfirmed || value.cleanupFailure!==null))fail();
  if(value.removed && !(value.reap?.noCreateAttempt===true && value.containerId===null) &&
    (!value.dockerRemoved || !(value.kernelReapProven || value.reap?.notStarted===true)))fail();
  if(Object.hasOwn(value,'acquisition'))validateAcquisitions(value.acquisition);
  return value;
}
export function createOwnerControl({onReserved=()=>{},onReady,onClosed}) {
  let buffer=Buffer.alloc(0),total=0,ready=false,closed=false,reserved=null;
  return Object.freeze({
    push(chunk){
      if(closed || !(chunk instanceof Uint8Array) || (total+=chunk.length)>2097152)fail();
      buffer=Buffer.concat([buffer,chunk]);
      let index;
      while((index=buffer.indexOf(10))>=0){
        const text=new TextDecoder('utf-8',{fatal:true,ignoreBOM:true}).decode(buffer.subarray(0,index));
        const value=JSON.parse(text);if(JSON.stringify(value)!==text)fail();buffer=buffer.subarray(index+1);
        if(value.type==='reserved'){
          exact(value,['type','nonce','name']);
          if(reserved || !/^[a-f0-9]{32}$/.test(value.nonce) || value.name!=='zryna410-ui-'+value.nonce)fail();
          reserved=value;onReserved(value);
        }else if(value.type==='ready'){
          exact(value,['type','nonce','containerId','audit']);exact(value.audit,['version','rootSha256','files','bytes']);
          if(!reserved || ready || !hash(value.containerId) || value.nonce!==reserved.nonce || value.audit.version!==1 ||
            value.audit.files!==419 || !hash(value.audit.rootSha256) || !Number.isSafeInteger(value.audit.bytes))fail();
          ready=true;onReady(value);
        }else if(value.type==='closed'){
          exact(value,['type','receipt']);validateOwnerReceipt(value.receipt);
          if(!reserved || value.receipt.nonce!==reserved.nonce || value.receipt.name!==reserved.name)fail();
          if(buffer.length)fail();
          closed=true;onClosed(value.receipt);
        }else fail();
      }
    },
    end(){if(buffer.length || !closed)fail();},
  });
}

function integer(value,minimum,maximum){return Number.isSafeInteger(value)&&value>=minimum&&value<=maximum;}
function pids(value){
  if(!Array.isArray(value)||value.length>256||value.some((pid,index)=>!integer(pid,1,4294967295)||(index&&pid<=value[index-1])))fail();
}
function group(value){
  exact(value,['processes','threads','current']);pids(value.processes);pids(value.threads);
  if(!integer(value.current,2,256)||value.current!==value.threads.length||value.processes.length<2||
    value.processes.some(pid=>!value.threads.includes(pid)))fail();
}
function epoch(value){
  exact(value,['statAbsent','statusAbsent','state','ticks','tgid']);
  if(typeof value.statAbsent!=='boolean'||typeof value.statusAbsent!=='boolean'||
    (value.statAbsent?(value.state!==null||value.ticks!==null):
      typeof value.state!=='string'||!(/^[RSDZTWtXxKPI]$/).test(value.state)||
      typeof value.ticks!=='string'||!(/^[0-9]{1,20}$/).test(value.ticks))||
    (value.statusAbsent?value.tgid!==null:!integer(value.tgid,1,4294967295)))fail();
}
function leaders(value){
  if(!Array.isArray(value)||value.length<2||value.length>256)fail();
  value.forEach((leader,index)=>{
    exact(leader,['pid','ticks','parent']);
    if(!integer(leader.pid,1,4294967295)||!integer(leader.parent,0,4294967295)||
      typeof leader.ticks!=='string'||!(/^[0-9]{1,20}$/).test(leader.ticks)||
      index&&leader.pid<=value[index-1].pid)fail();
  });
}
export function validateAcquisitions(value){
  if(!Array.isArray(value)||value.length>2)fail();
  value.forEach((phase,index)=>{
    exact(phase,['format','version','phase','status','windowMs','attempts','acceptedAttempt']);
    if(phase.format!=='zryna.ui-kernel-acquisition.v1'||phase.version!==1||phase.windowMs!==1000||
      phase.phase!==['POST_PROC_AUDIT','POST_MATERIAL_AUDIT'][index]||!['accepted','failed'].includes(phase.status)||
      !Array.isArray(phase.attempts)||phase.attempts.length>3||
      (phase.status==='accepted'?phase.acceptedAttempt!==phase.attempts.length||phase.attempts.length===0:phase.acceptedAttempt!==null))fail();
    phase.attempts.forEach((row,attempt)=>{
      exact(row,['number','status','failure','task','before','after','targetBefore','targetAfter','initStable',
        'leadersStable','visibleProcesses','unrelatedZombies','noEscapedOwnedDescendants','leaders']);
      if(row.number!==attempt+1||!['accepted','retired-task','fatal'].includes(row.status)||
        ![null,'OPEN_ENOENT_FIRST','OTHER'].includes(row.failure)||row.task!==null&&!integer(row.task,1,4294967295)||
        ['initStable','leadersStable','noEscapedOwnedDescendants'].some(key=>row[key]!==null&&typeof row[key]!=='boolean')||
        row.visibleProcesses!==null&&!integer(row.visibleProcesses,0,8192)||
        row.unrelatedZombies!==null&&!integer(row.unrelatedZombies,0,row.visibleProcesses??8192))fail();
      for(const key of ['before','after'])if(row[key]!==null)group(row[key]);
      for(const key of ['targetBefore','targetAfter'])if(row[key]!==null)epoch(row[key]);
      if(row.leaders!==null){leaders(row.leaders);if(!row.before||
        JSON.stringify(row.leaders.map(leader=>leader.pid))!==JSON.stringify(row.before.processes))fail();}
      if(row.status!=='fatal'){
        if(!row.before||!row.after||!row.leaders||row.initStable!==true||row.leadersStable!==true||
          row.noEscapedOwnedDescendants!==true||!integer(row.visibleProcesses,row.leaders.length,8192)||
          !integer(row.unrelatedZombies,0,row.visibleProcesses)||
          JSON.stringify(row.before.processes)!==JSON.stringify(row.after.processes))fail();
      }
      if(row.status==='accepted'){
        if(phase.status!=='accepted'||attempt!==phase.attempts.length-1||row.failure!==null||row.task!==null||
          row.targetBefore!==null||row.targetAfter!==null||JSON.stringify(row.before)!==JSON.stringify(row.after))fail();
      }else if(row.status==='retired-task'){
        if(row.failure!=='OPEN_ENOENT_FIRST'||row.task===null||row.before.processes.includes(row.task)||
          !row.before.threads.includes(row.task)||row.after.threads.includes(row.task)||
          row.after.threads.some(pid=>!row.before.threads.includes(pid))||row.before.current-row.after.current!==1||
          row.before.threads.filter(pid=>!row.after.threads.includes(pid)).length!==1||
          ['targetBefore','targetAfter'].some(key=>!row[key]||row[key].statAbsent!==true||row[key].statusAbsent!==true))fail();
      }else if(phase.status!=='failed'||attempt!==phase.attempts.length-1)fail();
      if(attempt&&row.leaders!==null&&JSON.stringify(row.leaders)!==JSON.stringify(phase.attempts[0].leaders))fail();
      if(index&&row.leaders!==null&&JSON.stringify(row.leaders)!==JSON.stringify(value[0].attempts.at(-1).leaders))fail();
    });
    if(phase.status==='accepted'&&phase.attempts.at(-1).status!=='accepted')fail();
    if(index&&value[0].status!=='accepted')fail();
  });
  return value;
}
