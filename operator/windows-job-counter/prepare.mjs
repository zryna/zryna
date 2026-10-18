// Diagnostic transport/selection only. Product checkout stays exact E and clean.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,appendFileSync} from 'node:fs';
import {resolve,join} from 'node:path';
import {execFileSync} from 'node:child_process';
import {createHash} from 'node:crypto';
const [repoArg,rootArg]=process.argv.slice(2),repo=resolve(repoArg),root=resolve(rootArg);
assert.equal(process.argv.length,4);assert.equal(process.platform,'win32');assert.equal(process.version,'v22.22.1');
const source={commit:'71fd69d99a39c22bdf262d1655a032aa76221f3e',tree:'351da60bd4037e0f96c286045c8943f41fa99b03'};
const sha=b=>createHash('sha256').update(b).digest('hex');const save=(n,v)=>writeFileSync(join(root,n),JSON.stringify(v,null,2)+'\n',{flag:'wx'});
const git=(...a)=>execFileSync('git',['--no-replace-objects',...a],{cwd:repo,timeout:15000}).toString().trim();
assert.equal(git('rev-parse','HEAD'),source.commit);assert.equal(git('rev-parse','HEAD^{tree}'),source.tree);assert.equal(git('status','--porcelain','--untracked-files=all'),'');mkdirSync(root);
const original=readFileSync(join(repo,'scripts/m6/installed-editor/owner-windows-job.ps1'));assert.equal(sha(original),'e1a78c8b3210ca19126b1bcdb4f5ae904d016f7b978b709b7f95d76f136ea1e6');
let text=original.toString();
const a='if (!QueryInformationJobObject(job, 3, memory, (uint)bytes, out returned)) throw new Win32Exception();';
const b='if (!QueryInformationJobObject(job, 3, memory, (uint)bytes, out returned)) { int error = Marshal.GetLastWin32Error(); throw new Win32Exception(error, $"Private job query failed; assigned=unavailable; count=unavailable; returned={returned}; allocated={bytes}; win32Error={error}."); }';
const x='throw new InvalidOperationException("Bounded complete private job inventory required.");';
const y='throw new InvalidOperationException($"Bounded complete private job inventory required; assigned={assigned}; count={count}; returned={returned}; allocated={bytes}; maximum={maximum}.");';
assert.equal(text.split(a).length,2);assert.equal(text.split(x).length,2);text=text.replace(a,b).replace(x,y);
writeFileSync(join(root,'owner-windows-job.original.ps1'),original,{flag:'wx'});writeFileSync(join(root,'owner-windows-job.instrumented.ps1'),text,{flag:'wx'});
save('PREPARATION.json',{source,startedAt:new Date().toISOString(),original:{bytes:original.length,sha256:sha(original)},effective:{bytes:Buffer.byteLength(text),sha256:sha(text)},scope:'Two diagnostic exception messages only; original guards,257capacity,queries,deadlines,ownership andcleanup unchanged. Failed-query assigned/count are unavailable because Win32 did not return a valid complete result.'});
const paths=['/repos/zryna/zryna/actions/runs/39164226628','/repos/zryna/zryna/actions/artifacts/12151852650','/repos/zryna/zryna/actions/runs/39164226628/jobs?per_page=100'];
const values=[];
for(const [i,path]of paths.entries()){
 const response=await fetch('https://api.github.com'+path,{headers:{authorization:'Bearer '+process.env.GITHUB_TOKEN,accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28'},redirect:'error',signal:AbortSignal.timeout(25000)});
 assert.equal(response.status,200);const raw=Buffer.from(await response.arrayBuffer());assert(raw.length<=2097152);writeFileSync(join(root,'api-'+i+'.json'),raw,{flag:'wx'});values.push(JSON.parse(raw));
}
const [run,artifact,jobs]=values;assert.equal(run.id,39164226628);assert.equal(run.head_sha,source.commit);assert.equal(run.head_branch,'main');assert.equal(run.status,'completed');
assert.equal(artifact.id,12151852650);assert.equal(artifact.name,'portable-setup-windows-2022');assert.equal(artifact.expired,false);assert.equal(artifact.size_in_bytes,125331054);assert.equal(artifact.digest,'sha256:27739487e10399e20942a09e494ad9c9d5eab2c6298f69c85b2f018a0a4e9ed5');assert.equal(artifact.workflow_run.id,run.id);assert.equal(artifact.workflow_run.head_sha,source.commit);
const portable=jobs.jobs.find(j=>j.id===117794484500);assert(portable);assert.equal(portable.name,'portable setup / reproduce portable setup (windows-2022)');assert.equal(portable.status,'completed');assert.equal(portable.conclusion,'success');assert.equal(portable.head_sha,source.commit);
save('HANDOFF_AUTHENTICATION.json',{source,runId:run.id,artifact,portableJob:{id:portable.id,name:portable.name,status:portable.status,conclusion:portable.conclusion},overallRunConclusion:run.conclusion,scope:'Passed portable producer from failed overall CI is reused only for diagnosis. Pinned download-artifact consumes exact artifact ID; original setup content/source validation remains unchanged.'});
const epoch=git('show','-s','--format=%ct','HEAD');
appendFileSync(process.env.GITHUB_ENV,`M6_DIAG_ROOT=${root}\nM6_DIAG_SOURCE=${repo}\nM6_DIAG_OUTPUT=${join(root,'installed-editor')}\nM6_SOURCE_EPOCH=${epoch}\n`);
