// Fixed diagnostic operator only; no public/same-run/GUI acceptance from transport.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdirSync, readFileSync, realpathSync, writeFileSync, openSync, fsyncSync, closeSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
const SOURCE='c9fc03ba855a37a635446dd8a6d42b1bb6f434fa',TREE='87154e66958769a266d1dc6e4368633ff782f1c8';
const RUN=38970116357,REPO=1350402316,CAP=128*1024*1024,end=performance.now()+180000;
const selected={linux:{os:'ubuntu-24.04',id:12076244446,bytes:55958932,
 sha:'ab259375622f407b12f1013550ab836690b7baca4e885ae72d2d0df221486438',
 build:117206136208,accept:117214850041,archive:'zryna-setup-0.1.0-candidate.3-x86_64-unknown-linux-gnu.tar.gz'},
 win32:{os:'windows-2022',id:12077548189,bytes:125331054,
 sha:'bd6f6f72933483bfa107d37dd38cf8cb97f95909fb47fcde79242fb9063a2efe',
 build:117206135666,accept:117214849929,archive:'zryna-setup-0.1.0-candidate.3-x86_64-pc-windows-msvc.zip'}}[process.platform];
assert(selected && process.version==='v22.22.1' && process.arch==='x64');
assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');
assert.equal(process.env.GITHUB_REF,'refs/heads/m6-editor-crossrun-diagnostic-20261016');
const base=process.env.M6_DIAGNOSTIC_BASE;
assert.equal(base,process.platform==='linux'?'/tmp':resolve(process.env.RUNNER_TEMP));
assert.equal(realpathSync(base),base);
const root=join(base,'m6-crossrun-input'),handoff=join(base,'m6-editor-handoff');
mkdirSync(root,{mode:0o700});
const hash=b=>createHash('sha256').update(b).digest('hex');
const left=max=>{const n=Math.floor(end-performance.now());assert(n>0,'Shared180s transport budget');return Math.min(n,max);};
function put(name,data){const fd=openSync(join(root,name),'wx',0o600);
 try{writeFileSync(fd,data);fsyncSync(fd);}finally{closeSync(fd);}}
const record={diagnosticOnly:true,canonicalSameRunAcceptance:false,product:{commit:SOURCE,tree:TREE},
 operator:process.env.GITHUB_SHA,run:RUN,artifact:selected.id,verified:false};
function gh(path,limit,timeout){left(timeout);const bytes=execFileSync('gh',
 ['api','--hostname','github.com','--method','GET','repos/zryna/zryna/'+path],
 {timeout:left(timeout),killSignal:'SIGKILL',maxBuffer:limit,encoding:'buffer',windowsHide:true,
  stdio:['ignore','pipe','pipe'],env:process.env});left(timeout);assert(bytes.length<=limit);return bytes;}
function api(path,name){const raw=gh(path,2*1024*1024,30000);put(name+'.raw.json',raw);
 return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));}
try{
 const run=api('actions/runs/'+RUN,'RUN');
 const commit=api('git/commits/'+SOURCE,'COMMIT');
 const artifacts=api('actions/runs/'+RUN+'/artifacts?per_page=100','ARTIFACTS');
 assert(run.id===RUN && run.head_sha===SOURCE && run.run_attempt===1 && run.repository.id===REPO
  && run.path==='.github/workflows/m6-editor-diagnostic.yml' && run.head_branch==='m6-editor-diagnostic-20261016');
 assert(commit.sha===SOURCE && commit.tree.sha===TREE);
 for(const [kind,id] of [['build',selected.build],['accept',selected.accept]]){
  const job=api('actions/jobs/'+id,kind.toUpperCase());
  assert(job.id===id && job.run_id===RUN && job.head_sha===SOURCE && job.run_attempt===1
   && job.status==='completed' && job.conclusion==='success');
  assert.equal(job.name,'early portable setup diagnostic / '+(kind==='build'
   ?'reproduce portable setup':'fresh portable acceptance')+' ('+selected.os+')');
 }
 assert(artifacts.total_count===artifacts.artifacts.length && artifacts.total_count<=100);
 const matches=artifacts.artifacts.filter(a=>a.name==='portable-setup-'+selected.os);
 assert.equal(matches.length,1);const a=matches[0];
 assert(a.id===selected.id && a.size_in_bytes===selected.bytes && a.digest==='sha256:'+selected.sha
  && a.expired===false && a.workflow_run.id===RUN && a.workflow_run.repository_id===REPO
  && a.workflow_run.head_sha===SOURCE);
 const raw=gh('actions/artifacts/'+selected.id+'/zip',CAP,120000);
 assert.equal(raw.length,selected.bytes);assert.equal(hash(raw),selected.sha);
 put('handoff.transport.zip',raw);left(30000);
 const extractor=join(dirname(fileURLToPath(import.meta.url)),'extract-handoff.py');
 assert.equal(hash(readFileSync(extractor)),'97894b17642c03c91ba6059bad574ec5fac0f6c9380c2c526c5950bce4cca1cf');
 const output=execFileSync(process.platform==='win32'?'python':'python3',
 ['-I','-B',extractor,join(root,'handoff.transport.zip'),handoff,selected.archive],
 {timeout:left(30000),killSignal:'SIGKILL',maxBuffer:1024*1024,encoding:'buffer',windowsHide:true,
  stdio:['ignore','pipe','pipe'],env:process.env});left(1);
 const decoded=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(output));
 assert(decoded.crcVerified===true && decoded.members.length===3 && decoded.expandedBytes<=CAP);
 Object.assign(record,{verified:true,transport:{bytes:raw.length,sha256:hash(raw)},...decoded});
 put('TRANSPORT_VERIFICATION.json',Buffer.from(JSON.stringify(record,null,2)+'\n'));
}catch(error){record.failure={name:error.name,code:typeof error.code==='string'?error.code:null,
 status:error.status??null,signal:error.signal??null,stdoutBytes:error.stdout?.length??0,
 stderrBytes:error.stderr?.length??0};
 put('TRANSPORT_FAILURE.json',Buffer.from(JSON.stringify(record,null,2)+'\n'));
 throw new Error('Fixed cross-run handoff failed; inspect retained bounded metadata.');}
