// Private immutable workflow operator, never a source/publication/containment capability.
import assert from 'node:assert/strict';
import {mkdirSync,readFileSync,writeFileSync,openSync,closeSync,realpathSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {createRequire} from 'node:module';
import {dirname,join,resolve} from 'node:path';
import {pathToFileURL,fileURLToPath} from 'node:url';
import {performance} from 'node:perf_hooks';
assert.equal(process.argv.length,2);assert.equal(process.version,'v22.22.1');assert.equal(process.arch,'x64');assert(['linux','win32'].includes(process.platform));
const commit=process.env.SELECTED_FINAL_MAIN,tree=process.env.SELECTED_FINAL_TREE,operator=process.env.SELECTED_OPERATOR_COMMIT;
assert([commit,tree,operator].every(s=>/^[a-f0-9]{40}$/.test(s??'')));
const runId=Number(process.env.SELECTED_CI_RUN),attempt=Number(process.env.SELECTED_CI_ATTEMPT);assert(Number.isSafeInteger(runId)&&runId>0&&Number.isSafeInteger(attempt)&&attempt>0);
const artifact=JSON.parse(process.env.SELECTED_SETUP_ARTIFACT);assert.deepEqual(Object.keys(artifact).sort(),['bytes','expiresAt','id','sha256']);
assert(Number.isSafeInteger(artifact.id)&&artifact.id>0&&Number.isSafeInteger(artifact.bytes)&&artifact.bytes>0&&artifact.bytes<=256*1024*1024&&/^[a-f0-9]{64}$/.test(artifact.sha256)&&Date.parse(artifact.expiresAt)>Date.now());
const source=realpathSync(join(process.env.GITHUB_WORKSPACE,'source')),opRoot=realpathSync(join(process.env.GITHUB_WORKSPACE,'operator'));
const output=resolve(process.env.RUNNER_TEMP,'m6-installed-supplement');mkdirSync(output,{mode:0o700});
const resultFd=openSync(join(output,'RESULT.json'),'wx',0o400);let outcome={status:'failed'};
const sha=b=>createHash('sha256').update(b).digest('hex');const json=(name,value)=>writeFileSync(join(output,name),JSON.stringify(value,null,2)+'\n',{flag:'wx',mode:0o400});
const git=(root,args)=>execFileSync('git',['--no-replace-objects','-c','core.fsmonitor=false',...args],{cwd:root,timeout:10000,maxBuffer:2097152});
const sourceTuple=()=>({commit:git(source,['rev-parse','HEAD']).toString().trim(),tree:git(source,['rev-parse','HEAD^{tree}']).toString().trim(),status:git(source,['status','--porcelain','--untracked-files=all']).toString()});
const sourceImport=async rel=>{
 const data=readFileSync(join(source,rel));assert(data.equals(git(source,['show',commit+':'+rel])));
 return import(pathToFileURL(join(source,rel)).href);
};
let apiCount=0;
try{
 assert.equal(git(opRoot,['rev-parse','HEAD']).toString().trim(),operator);assert.equal(git(opRoot,['status','--porcelain','--untracked-files=all']).toString(),'');
 const before=sourceTuple();assert.deepEqual(before,{commit,tree,status:''});json('SOURCE_BEFORE.json',before);
 const {githubApi}=await sourceImport('scripts/m4/hosted-api.mjs');const rawApi=githubApi();
 const api=async path=>{assert(path.startsWith('/repos/zryna/zryna/'));assert(++apiCount<=150);const result=await rawApi(path);json('api-'+String(apiCount).padStart(3,'0')+'.json',result);return result};
 const {observeMain,expectedJobs,observeCi}=await sourceImport('scripts/m6/compiler-release/gates.mjs');
 const selection={repository:'zryna/zryna',commit,tree};const client={request:api};const main=await observeMain(client,selection);const inventory=await expectedJobs(source,client,selection);assert.equal(inventory.jobs.length,42);const ci=await observeCi(client,selection,runId,attempt,inventory);json('FULL_MAIN_ADMISSION.json',{selection,main,inventory,ci});
 const observed=await api('/repos/zryna/zryna/actions/artifacts/'+artifact.id);const baseline=process.platform==='linux'?'ubuntu-24.04':'windows-2022';
 assert.equal(observed.name,'portable-setup-'+baseline);assert.equal(observed.workflow_run.id,runId);assert.equal(observed.workflow_run.head_sha,commit);assert.equal(observed.expired,false);assert.equal(observed.expires_at,artifact.expiresAt);assert.equal(observed.size_in_bytes,artifact.bytes);assert.equal(observed.digest,'sha256:'+artifact.sha256);
 const jobs=await api('/repos/zryna/zryna/actions/runs/'+runId+'/attempts/'+attempt+'/jobs?per_page=100');
 const [job]=jobs.jobs.filter(j=>j.name==='portable setup / reproduce portable setup ('+baseline+')');assert(job&&job.conclusion==='success'&&job.status==='completed'&&job.head_sha===commit);
 assert(Date.parse(observed.created_at)>=Date.parse(job.started_at)&&Date.parse(observed.created_at)<=Date.parse(job.completed_at));
 const {downloadPublicationArchive}=await sourceImport('scripts/m4/retain-publication.mjs');
 const archive=await downloadPublicationArchive('zryna/zryna',artifact);writeFileSync(join(output,'original-setup-artifact.zip'),archive,{flag:'wx',mode:0o400});
 const {zipEntries}=await sourceImport('scripts/m4/hosted-archive.mjs');const members=new Map();zipEntries(archive,(f,data)=>{assert(!members.has(f.path));members.set(f.path,Buffer.from(data))});assert.equal(members.size,3);
 const receipt=JSON.parse(members.get('candidate-receipt.json'));assert.equal(receipt.format,'zryna.portable-setup-receipt.v1');assert.equal(receipt.sourceCommit,commit);assert.equal(receipt.sourceTree,tree);assert.equal(receipt.status,'review-candidate');assert.equal(receipt.productionAdmission,'forbidden');
 const target=process.platform==='linux'?'x86_64-unknown-linux-gnu':'x86_64-pc-windows-msvc';assert.equal(receipt.target,target);
 const filename='zryna-setup-0.1.0-candidate.3-'+target+(process.platform==='linux'?'.tar.gz':'.zip');assert.equal(receipt.archive.filename,filename);assert(members.has(filename)&&members.has('reproduction.json'));
 const reproduced=JSON.parse(members.get('reproduction.json'));assert.equal(reproduced.sourceCommit,commit);assert.equal(reproduced.target,target);assert.deepEqual(reproduced.serverReplicas,[receipt.serverSha256,receipt.serverSha256]);assert.deepEqual(reproduced.vsixReplicas,[receipt.vsixSha256,receipt.vsixSha256]);
 const bytes=members.get(filename);assert.equal(bytes.length,receipt.archive.size);assert.equal(sha(bytes),receipt.archive.sha256);
 for(const [n,data]of members)writeFileSync(join(output,n),data,{flag:'wx',mode:0o400});
 const rootName=filename.replace(process.platform==='linux'?/\.tar\.gz$/:/\.zip$/,'');let files;
 if(process.platform==='linux'){const {decodeTar}=await sourceImport('scripts/distribution/archive-tar.mjs');const epoch=Number(git(source,['show','-s','--format=%ct',commit]).toString().trim());files=await decodeTar(bytes,rootName,epoch)}
 else{const {decodeZip}=await sourceImport('scripts/distribution/archive-zip.mjs');files=decodeZip(bytes,rootName,['compiler/bin/zryna.exe','compiler/runtime/node/node.exe','bin/zryna-language-server.exe','install.sh'])}
 const manifest=files.find(f=>f.path==='setup.json');assert(manifest);assert.equal(sha(manifest.data),receipt.manifestSha256);
 const {extractVerifiedProductionFiles}=await sourceImport('scripts/distribution-release/run-installed-acceptance.mjs');const installation=join(output,'installation');const previousUmask=process.umask(0o022);try{extractVerifiedProductionFiles(installation,files)}finally{process.umask(previousUmask)}
 const require=createRequire(import.meta.url);const {verifyInstallation}=require(join(source,'editors/vscode-zryna/src/installation.cjs'));const checked=verifyInstallation(installation,receipt.manifestSha256);
 assert.equal(checked.manifest.sourceCommit,commit);assert.equal(checked.manifest.sourceTree,tree);
 const server=readFileSync(checked.serverPath);assert.equal(sha(server),receipt.serverSha256);const vsix=files.find(f=>f.path==='editor/zryna-0.5.0.vsix');assert.equal(sha(vsix.data),receipt.vsixSha256);
 json('PREPARATION.json',{source:selection,operator,runId,attempt,artifact,receipt,files:files.length,actualSetupManifestVerified:true,originalModesRetained:true,sourceCjsVerifierSha256:sha(readFileSync(join(source,'editors/vscode-zryna/src/installation.cjs'))),scope:'Actual same-source complete review-candidate setup materialized; no public setup admission.'});
 // The authenticated fetch token is not inherited by any native process owner or target.
 delete process.env.GITHUB_TOKEN;
 const script=join(dirname(fileURLToPath(import.meta.url)),'client.mjs');
 const env={PATH:process.env.PATH,LANG:'C.UTF-8',LC_ALL:'C.UTF-8',TZ:'UTC',HOME:output,USERPROFILE:output,TMPDIR:output,TEMP:output,TMP:output,...(process.platform==='win32'?{ProgramFiles:process.env.ProgramFiles,SystemRoot:process.env.SystemRoot,WINDIR:process.env.SystemRoot,PATHEXT:'.EXE',COMSPEC:join(process.env.SystemRoot,'System32/cmd.exe')}:{})};
 mkdirSync(join(output,'stage'),{mode:0o700});const argv=[script,'--source',source,'--installation',installation,'--manifest-sha256',receipt.manifestSha256,'--source-commit',commit,'--output',join(output,'captured')];
 json('NATIVE_INTENT.json',{program:process.execPath,argv,cwd:source,environment:env,clientSha256:sha(readFileSync(script)),lifetimeMs:120000,scope:'Unchanged native process owner; trusted fixed installed CLI/LSP supplement only, no hostile containment claim.'});
 const {launchOwned}=await sourceImport('scripts/m6/installed-editor/owner-client.mjs');
 const owner=await launchOwned(process.execPath,argv,{cwd:source,env,output,stage:join(output,'stage'),name:'FIXED_SUPPLEMENT',deadline:performance.now()+120000});
 const actual=await owner.finish();assert.equal(actual.ended.status,0);assert.equal(actual.ended.signal,null);
 const proof=JSON.parse(readFileSync(join(output,'FIXED_SUPPLEMENT.json')));assert.equal(proof.cleanupConfirmed,true);assert.equal(proof.ownerExit.status,0);assert.equal(proof.complete.failures.length,0);
 const captured=JSON.parse(readFileSync(join(output,'captured/RESULT.json')));assert.equal(captured.status,'captured-fixed-supplement-pass');assert.equal(captured.sourceCommit,commit);assert.equal(captured.manifestSha256,receipt.manifestSha256);assert.equal(captured.rows.length,6);
 verifyInstallation(installation,receipt.manifestSha256);const after=sourceTuple();assert.deepEqual(after,before);json('SOURCE_AFTER.json',after);
 outcome={status:'actual-installed-supplement-pass',source:selection,operator,runId,attempt,artifact,manifestSha256:receipt.manifestSha256,serverSha256:receipt.serverSha256,clientCases:captured.rows,ownerCleanupConfirmed:true,sourceUnchanged:true,scope:'Actual fixed installed CLI and LSP supplement, no public setup admission or hostile containment claim.'};console.log(JSON.stringify(outcome));
}catch(error){outcome.failure=String(error.stack??error).slice(0,5000);throw error}finally{writeFileSync(resultFd,JSON.stringify(outcome,null,2)+'\n');closeSync(resultFd)}
