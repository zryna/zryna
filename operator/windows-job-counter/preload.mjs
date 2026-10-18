// Rewrite only the already copied diagnostic Job helper, outside the clean product checkout.
import assert from 'node:assert/strict';
import childProcess from 'node:child_process';
import {syncBuiltinESMExports} from 'node:module';
import {readFileSync,writeFileSync,mkdirSync,chmodSync,lstatSync} from 'node:fs';
import {dirname,basename,resolve,join,relative} from 'node:path';
import {createHash} from 'node:crypto';
assert.equal(process.platform,'win32');assert.equal(process.version,'v22.22.1');
const sha=b=>createHash('sha256').update(b).digest('hex');const root=resolve(process.env.M6_DIAG_ROOT),output=resolve(process.env.M6_DIAG_OUTPUT);
const prepared=JSON.parse(readFileSync(join(root,'PREPARATION.json'))),effective=readFileSync(join(root,'owner-windows-job.instrumented.ps1'));
assert.equal(sha(effective),prepared.effective.sha256);assert.equal(effective.length,prepared.effective.bytes);
const patches=join(root,'overlays');mkdirSync(patches);const originalSpawn=childProcess.spawn;let count=0;
childProcess.spawn=function(program,args,options){
 if(Array.isArray(args)&&args.length===5&&args[3]==='-File'&&basename(args[4])==='owner.ps1'){
  const owner=resolve(args[4]),path=join(dirname(owner),'owner-windows-job.ps1'),rel=relative(join(output,'commands'),path);
  assert(rel&&!rel.startsWith('..')&&!resolve(path).startsWith(resolve(process.env.M6_DIAG_SOURCE)));assert(/owner-material[\\/]owner-windows-job\.ps1$/.test(rel));
  assert(/powershell[\\/]7[\\/]pwsh\.exe$/i.test(program));const before=readFileSync(path),st=lstatSync(path);assert(st.isFile()&&!st.isSymbolicLink());
  assert.equal(sha(before),prepared.original.sha256);assert.equal(before.length,prepared.original.bytes);
  const id=String(++count).padStart(3,'0'),snapshot=join(patches,id+'.original.ps1');writeFileSync(snapshot,before,{flag:'wx'});
  chmodSync(path,0o600);writeFileSync(path,effective);chmodSync(path,0o400);assert(readFileSync(path).equals(effective));
  writeFileSync(join(patches,id+'.json'),JSON.stringify({createdAt:new Date().toISOString(),program,args,originalCopiedPath:path,originalCapture:{bytes:before.length,sha256:sha(before)},effectiveCopy:{bytes:effective.length,sha256:sha(effective)},originalSnapshot:snapshot,scope:'Diagnostic-only captured helper overlay. Original owner-client jobCapture hash describes pre-overlay bytes; this record describes the effective executing copy. No original acceptance assertion or guard changed; not sourceE CI proof.'},null,2)+'\n',{flag:'wx'});
 }
 return originalSpawn.call(this,program,args,options);
};
syncBuiltinESMExports();
