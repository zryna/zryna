// Shared monotonic cleanup allowance; neither errors nor repeated calls renew it.
import { performance } from 'node:perf_hooks';
import { fail } from './wire.mjs';
export function createOwnerCompletion({child,closed,childClosed,captured,launcher,session,receipt,readyRecord,reservedRecord=()=>null}) {
  let stopping=null,unknown=false,helperReleased=false;
  const recovery=()=>Object.freeze({owner:receipt()??{containerId:readyRecord()?.containerId??null,
    name:reservedRecord()?.name??null,nonce:reservedRecord()?.nonce??null,ownershipUnconfirmed:true},
    helperPid:child.pid??null,helperSnapshot:helperReleased?null:captured.root,
    ownershipUnconfirmed:unknown,launcher:launcher.recovery()});
  function stop(){
    if(stopping)return stopping;
    const deadlineNs=(process.hrtime.bigint()+2000000000n).toString();
    const deadline=performance.now()+2000;
    const tasks=[Promise.resolve().then(()=>session()?.stop()),Promise.resolve().then(()=>launcher.stop()),
      Promise.resolve().then(async()=>{
        if(!receipt() && !child.stdio[3].destroyed)child.stdio[3].write(JSON.stringify({type:'stop',deadlineNs})+'\n');
        const [owner,status]=await Promise.all([closed,childClosed]);
        child.stdin.end();
        if(!owner.removed || owner.ownershipUnconfirmed || owner.cleanupFailure || status.termination)fail();
        if(unknown || performance.now()>=deadline)fail();
        captured.cleanup();helperReleased=true;
        if(status.code!==0)fail();
        return owner;
      })];
    stopping=(async()=>{
      let timer;
      try{
        const outcomes=await Promise.race([Promise.allSettled(tasks),new Promise((_,reject)=>{
          timer=setTimeout(()=>reject(new Error('PLAYGROUND-UI-OWNER-CLEANUP')),Math.max(1,deadline-performance.now()));
        })]);
        const errors=outcomes.filter(value=>value.status==='rejected').map(value=>value.reason);
        if(errors.length)throw new AggregateError(errors,'PLAYGROUND-UI-OWNER-CLEANUP');
        if(launcher.recovery().host)fail();
        return receipt();
      }catch(error){unknown=true;throw error;}finally{clearTimeout(timer);}
    })();
    return stopping;
  }
  return Object.freeze({stop,recovery});
}
