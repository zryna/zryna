// Fixed diagnostic operator only; no public/same-run/GUI acceptance from transport.
import assert from 'node:assert/strict';
import { execFileSync, spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, realpathSync, writeFileSync, openSync, fsyncSync, closeSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { performance } from 'node:perf_hooks';
const SOURCE='d3c2ade51730bc9998a6b65e196facecfebfa455',TREE='74ba710078f14779c9c753bd494b277c9ac988ec';
const RUN=38982854099,REPO=1350402316,CAP=128*1024*1024,end=performance.now()+180000;
const selected={linux:{os:'ubuntu-24.04',id:12083003628,bytes:55958902,
 sha:'e4e182edb6f93f93bd9dba8784a72fd4e3f6d2aedf0030d43ece68cbe6608df8',
 build:117259308756,accept:117263909953,archive:'zryna-setup-0.1.0-candidate.3-x86_64-unknown-linux-gnu.tar.gz'},
 win32:{os:'windows-2022',id:12083692448,bytes:125331054,
 sha:'8043d2bd89307b8d5096b844cdaf6f475a0e1c1e0bb69c0504d54a17d26da8a4',
 build:117259308787,accept:117263909943,archive:'zryna-setup-0.1.0-candidate.3-x86_64-pc-windows-msvc.zip'}}[process.platform];
assert(['linux','win32'].includes(process.platform));
assert(selected && process.version==='v22.22.1' && process.arch==='x64');
assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');
assert.equal(process.env.GITHUB_REF,'refs/heads/m6-editor-composed-diagnostic-20261017');
const base=process.env.M6_DIAGNOSTIC_BASE;
assert.equal(base,resolve(process.env.RUNNER_TEMP));
assert.equal(realpathSync(base),base);
const root=join(base,'m6-crossrun-input'),handoff=join(base,'m6-editor-handoff');
mkdirSync(root,{mode:0o700});
const hash=b=>createHash('sha256').update(b).digest('hex');
const left=max=>{const n=Math.floor(end-performance.now());assert(n>0,'Shared180s transport budget');return Math.min(n,max);};
function put(name,data){const fd=openSync(join(root,name),'wx',0o600);
 try{writeFileSync(fd,data);fsyncSync(fd);}finally{closeSync(fd);}}
const record={diagnosticOnly:true,canonicalSameRunAcceptance:false,product:{commit:SOURCE,tree:TREE},
 operator:process.env.GITHUB_SHA,run:RUN,artifact:selected.id,verified:false,phase:'INITIALIZED'};
const transportEnv={...process.env};delete transportEnv.GH_DEBUG;
const extractorEnv={...transportEnv};delete extractorEnv.GH_TOKEN;delete extractorEnv.GITHUB_TOKEN;
function gh(path,limit,timeout){left(timeout);const bytes=execFileSync('gh',
 ['api','--hostname','github.com','--method','GET','repos/zryna/zryna/'+path],
 {timeout:left(timeout),killSignal:'SIGKILL',maxBuffer:limit,encoding:'buffer',windowsHide:true,
  stdio:['ignore','pipe','pipe'],env:transportEnv});left(timeout);assert(bytes.length<=limit);return bytes;}
function api(path,name){record.phase='GH_'+name;const raw=gh(path,2*1024*1024,30000);
 record.phase='DECODE_'+name;put(name+'.raw.json',raw);
 return JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(raw));}
const streamBytes=value=>Buffer.isBuffer(value)?value:Buffer.from(value??'');
function retainExtractor(stdout,stderr,overflow){let overLimit=overflow;record.extractorStreams={};
 for(const [name,raw] of [['stdout',stdout],['stderr',stderr]]){
  const bytes=streamBytes(raw);const entry={bytes:bytes.length,sha256:hash(bytes),
   path:null,retention:overflow?'NOT_RETAINED_BUFFER_OVERFLOW':bytes.length>1024*1024?'NOT_RETAINED_OVER_LIMIT':'PENDING'};
  record.extractorStreams[name]=entry;
  if(overflow || bytes.length>1024*1024){overLimit=true;continue;}
  put('EXTRACTOR.'+name,bytes);entry.path='EXTRACTOR.'+name;entry.retention='RETAINED';
 }
 assert(!overLimit,'Extractor stream over1MiB is not retained and cannot pass');
}
function httpCode(raw){const bytes=streamBytes(raw);if(bytes.length>1024*1024)return null;
 const allowed=new Set([400,401,403,404,408,410,422,429,500,502,503,504]);
 const codes=[...new Set([...bytes.toString('utf8').matchAll(/\bHTTP ([1-5][0-9]{2})\b/g)]
  .map(match=>Number(match[1])).filter(code=>allowed.has(code)))];
 return codes.length===1?codes[0]:null;
}
try{
 const run=api('actions/runs/'+RUN,'RUN');
 const commit=api('git/commits/'+SOURCE,'COMMIT');
 const artifacts=api('actions/runs/'+RUN+'/artifacts?per_page=100','ARTIFACTS');
 record.phase='SOURCE_API_VALIDATE';
 assert(run.id===RUN && run.head_sha===SOURCE && run.run_attempt===1 && run.repository.id===REPO
  && run.path==='.github/workflows/ci.yml' && run.head_branch==='feat/410-cloud-continuation');
 assert(commit.sha===SOURCE && commit.tree.sha===TREE);
 for(const [kind,id] of [['build',selected.build],['accept',selected.accept]]){
  const job=api('actions/jobs/'+id,kind.toUpperCase());
  record.phase='VALIDATE_'+kind.toUpperCase();
  assert(job.id===id && job.run_id===RUN && job.head_sha===SOURCE && job.run_attempt===1
   && job.status==='completed' && job.conclusion==='success');
  assert.equal(job.name,'portable setup / '+(kind==='build'
   ?'reproduce portable setup':'fresh portable acceptance')+' ('+selected.os+')');
 }
 record.phase='ARTIFACT_METADATA_VALIDATE';
 assert(artifacts.total_count===artifacts.artifacts.length && artifacts.total_count<=100);
 const matches=artifacts.artifacts.filter(a=>a.name==='portable-setup-'+selected.os);
 assert.equal(matches.length,1);const a=matches[0];
 assert(a.id===selected.id && a.size_in_bytes===selected.bytes && a.digest==='sha256:'+selected.sha
  && a.expired===false && a.workflow_run.id===RUN && a.workflow_run.repository_id===REPO
  && a.workflow_run.head_sha===SOURCE);
 record.phase='GH_ARTIFACT_ZIP';
 const raw=gh('actions/artifacts/'+selected.id+'/zip',CAP,120000);
 record.phase='ARTIFACT_DIGEST_VERIFY';
 assert.equal(raw.length,selected.bytes);assert.equal(hash(raw),selected.sha);
 record.phase='ARTIFACT_WRITE';put('handoff.transport.zip',raw);left(30000);
 record.phase='EXTRACTOR_SOURCE_VERIFY';
 const extractor=join(dirname(fileURLToPath(import.meta.url)),'extract-handoff.py');
 assert.equal(hash(readFileSync(extractor)),'97894b17642c03c91ba6059bad574ec5fac0f6c9380c2c526c5950bce4cca1cf');
 record.phase='EXTRACTOR';
 const child=spawnSync(process.platform==='win32'?'python':'python3',
 ['-I','-B',extractor,join(root,'handoff.transport.zip'),handoff,selected.archive],
 {timeout:left(30000),killSignal:'SIGKILL',maxBuffer:1024*1024,encoding:'buffer',windowsHide:true,
  stdio:['ignore','pipe','pipe'],env:extractorEnv});
 record.extractorExit={status:child.status,signal:child.signal,
  errorCode:typeof child.error?.code==='string'?child.error.code:null};
 retainExtractor(child.stdout,child.stderr,child.error?.code==='ENOBUFS');
 if(child.error)throw child.error;
 assert.equal(child.status,0,'Fixed extractor must exit0');assert.equal(child.signal,null);
 left(1);record.phase='EXTRACTOR_RESULT_PARSE';const output=child.stdout;
 const decoded=JSON.parse(new TextDecoder('utf-8',{fatal:true}).decode(output));
 assert(decoded.crcVerified===true && decoded.members.length===3 && decoded.expandedBytes<=CAP);
 record.phase='COMPLETE';
 Object.assign(record,{verified:true,transport:{bytes:raw.length,sha256:hash(raw)},...decoded});
 put('TRANSPORT_VERIFICATION.json',Buffer.from(JSON.stringify(record,null,2)+'\n'));
}catch(error){const stdout=streamBytes(error.stdout),stderr=streamBytes(error.stderr);
 record.failure={phase:record.phase,name:error.name,code:typeof error.code==='string'?error.code
  :(record.phase==='EXTRACTOR'?record.extractorExit?.errorCode:null)??null,
 status:error.status??(record.phase==='EXTRACTOR'?record.extractorExit?.status:null)??null,
 signal:error.signal??(record.phase==='EXTRACTOR'?record.extractorExit?.signal:null)??null,
 stdoutBytes:stdout.length,stdoutSha256:hash(stdout),stderrBytes:stderr.length,stderrSha256:hash(stderr),
 httpCode:record.phase.startsWith('GH_')?httpCode(stderr):null};
 put('TRANSPORT_FAILURE.json',Buffer.from(JSON.stringify(record,null,2)+'\n'));
 throw new Error('Fixed cross-run handoff failed; inspect retained bounded phase/stream records.');}
