// Diagnostic only: original bounded read-only observations, not M4 acceptance.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import { syncBuiltinESMExports } from 'node:module';
import { mkdirSync, writeFileSync, readFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve, join } from 'node:path';
import { pathToFileURL } from 'node:url';
import { performance } from 'node:perf_hooks';
const source=process.cwd(),output=resolve(process.env.M4_PROBE_OUTPUT);
assert.equal(process.platform,'win32');assert.equal(process.version,'v22.22.1');
assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');
assert.equal(process.env.GITHUB_REF,'refs/heads/m6-m4-observation-diagnostic-20261016');
const sha=b=>createHash('sha256').update(b).digest('hex');
const observer=join(source,'scripts/m4/observe-context.mjs');
assert.equal(sha(readFileSync(observer)),'45e09ff08f259e00b1b9d0f9c355946e6f885eda9eabc35ea09e3699d85c5c0c');
mkdirSync(output,{mode:0o700});
const put=(name,bytes)=>writeFileSync(join(output,name),bytes,{flag:'wx'});
const caption=['-NoLogo','-NoProfile','-NonInteractive','-Command',
 '(Get-CimInstance Win32_OperatingSystem).Caption'];
const calls=[['git',['rev-parse','HEAD']],['git',['rev-parse','HEAD^{tree}']],
 ['git',['status','--porcelain=v1','--untracked-files=all']],['powershell.exe',caption]];
const original=childProcess.spawnSync;let count=0;
childProcess.spawnSync=(executable,args,options)=>{
 const number=count++;assert(number<calls.length);assert.deepEqual([executable,args],calls[number]);
 assert.equal(options.cwd,source);assert.equal(options.timeout,30000);assert.equal(options.maxBuffer,1048576);
 const start=performance.now(),result=original(executable,args,options);
 const record={call:number+1,executable,args,durationMs:performance.now()-start,status:result.status,
  signal:result.signal,errorCode:result.error?.code??null,
  captureEncoding:'Original spawnSync UTF8-decoded strings reencoded as UTF8; not native raw bytes',streams:{}};
 let streamFailure=false;
 for(const name of ['stdout','stderr']){
  const bytes=Buffer.from(result[name]??'');
  if(bytes.length>1048576){record.streams[name]={bytes:bytes.length,sha256:sha(bytes),
   retention:'NOT_RETAINED_REENCODED_OVER_LIMIT'};streamFailure=true;continue;}
  const file=String(number+1)+'-'+name;put(file,bytes);
  record.streams[name]={path:file,bytes:bytes.length,sha256:sha(bytes),retention:'RETAINED'};
 }
 put(String(number+1)+'.json',JSON.stringify(record,null,2)+'\n');
 assert(!streamFailure,'Reencoded stream over1MiB cannot pass');return result;
};
syncBuiltinESMExports();
try{
 const {observedSource,readObservation}=await import(pathToFileURL(observer));
 const selected=observedSource('cd95a99fc1cb0e36b14551b75452454fd9bbe456',
  'c5743f0e5d5655df8184c27837a97ea7c484955e','refs/heads/feat/410-cloud-continuation');
 const text=readObservation('powershell.exe',caption);
 assert.equal(count,4);put('RESULT.json',JSON.stringify({diagnosticOnly:true,actualM4Acceptance:false,
  source:selected,caption:text,calls:count,status:'read-only-observations-completed'},null,2)+'\n');
}catch(error){put('FAILURE.json',JSON.stringify({diagnosticOnly:true,calls:count,
 message:String(error.message)},null,2)+'\n');process.exitCode=1;}
finally{childProcess.spawnSync=original;syncBuiltinESMExports();}
