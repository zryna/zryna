// Sanitized separate owner FD; never receives the browser capability or raw HTTP frames.
import { exact, fail, hash } from './wire.mjs';
export function validateOwnerReceipt(value) {
  const required=['format','version','qualificationOnly','name','nonce','containerId','audit','kernel','inspect',
    'wait','removed','dockerRemoved','kernelReapProven','failure','cleanupFailure','ownershipUnconfirmed'];
  const optional=['imageBefore','volumeBefore','volumeAfterAudit','volumeAfterCleanup','imageAfter','channelTerminal','reap',
    'initialIdentity','kernelIdentityFailure'];
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
