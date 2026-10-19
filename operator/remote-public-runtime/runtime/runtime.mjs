// One original opaque publication capability: authenticate, compose, await approval, run five, exit.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,lstatSync,readdirSync,mkdirSync,chmodSync,chownSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {execFileSync} from 'node:child_process';
import {dirname,resolve,join,relative,isAbsolute} from 'node:path';
import {pathToFileURL} from 'node:url';
const sha=b=>createHash('sha256').update(b).digest('hex');
const exact=(v,k)=>{assert(v&&typeof v==='object'&&!Array.isArray(v));assert.deepEqual(Object.keys(v).sort(),[...k].sort());};
const ordinary=(path,maximum=1024**3,minimum=1)=>{
 assert(isAbsolute(path)&&resolve(path)===path);let p=path;
 while(true){const s=lstatSync(p);assert(!s.isSymbolicLink()&&s.uid===0&&!(s.mode&0o022));if(p===dirname(p))break;p=dirname(p);}
 const s=lstatSync(path);assert(s.isFile()&&s.nlink===1&&s.size>=minimum&&s.size<=maximum);const b=readFileSync(path),t=lstatSync(path);
 for(const k of ['dev','ino','size','mtimeMs','ctimeMs','uid','mode','nlink'])assert.equal(t[k],s[k]);assert.equal(b.length,s.size);return b;
};
const pin=(path,min=1)=>{const b=ordinary(path,1024**3,min);return {path,bytes:b.length,sha256:sha(b)};};
const checked=p=>{exact(p,['path','bytes','sha256']);assert.deepEqual(pin(p.path),p);return ordinary(p.path);};
const descriptor=p=>{exact(p,['path','bytes','sha256','ownerUid']);assert.equal(p.ownerUid,0);assert.deepEqual({...pin(p.path),ownerUid:0},p);return ordinary(p.path);};
const identity=v=>({commit:v.commit,tree:v.tree});
const decode=b=>JSON.parse(b.toString());
const [manifestPath]=process.argv.slice(2);assert.equal(process.argv.length,3);assert.equal(process.version,'v22.22.1');assert.equal(process.getuid(),0);
const manifestRaw=ordinary(manifestPath,65536);assert.equal(sha(manifestRaw),process.env.M6_REMOTE_MANIFEST_SHA256);const m=decode(manifestRaw);
assert.equal(m.approval.phase,'public');assert.equal(m.hostRoot,process.env.M6_REMOTE_HOST_ROOT);
assert.equal(process.env.GITHUB_RUN_ID,m.run.id);assert.equal(process.env.GITHUB_RUN_ATTEMPT,m.run.attempt);assert.equal(process.env.GITHUB_SHA,m.approval.operatorCommit);
const c=decode(checked(m.files.context));exact(c,['format','version','source','frozenMain','candidateReview','seed','prepare','publication','runtime','documentation']);
assert.equal(c.format,'zryna.m6-remote-public-context.v1');assert.equal(c.version,1);assert.deepEqual(c.source,m.source);assert.deepEqual(m.approval.source,identity(c.source));
const root=m.approval.root,baseline=c.prepare.root,operators=c.runtime.operatorRoot,output=c.runtime.outputRoot,repo=c.source.path;
assert.equal(root,m.hostRoot+'/public-composition');assert.equal(baseline,root+'/baseline');assert.equal(operators,root+'/operators');assert.equal(output,m.hostRoot+'/public-observations');
assert.equal(m.approval.prepared,'PREPARATION.json');assert.deepEqual(m.approval.approved,[{candidate:'approval.public.candidate.json',output:'approval.json'},{candidate:'ui-approval.candidate.json',output:'ui-approval.json'}]);
assert.deepEqual(m.approval.run,{id:Number(m.run.id),attempt:Number(m.run.attempt)});
for(const key of ['node','python','runtime','waitApproval'])checked(m.files[key]);assert.equal(process.execPath,m.files.node.path);assert.equal(import.meta.filename,m.files.runtime.path);
const frozen=decode(checked(c.frozenMain));assert.deepEqual(identity(frozen),identity(c.source));
const candidate=decode(checked(c.candidateReview));assert.equal(candidate.status,'official-two-replica-candidate-acquired-and-reassembled');assert.deepEqual(candidate.source,identity(c.source));
const git=(...args)=>execFileSync('git',['--no-replace-objects',...args],{cwd:repo,timeout:30000,maxBuffer:2097152});
const clean=()=>{assert.equal(git('rev-parse','HEAD').toString().trim(),c.source.commit);assert.equal(git('rev-parse','HEAD^{tree}').toString().trim(),c.source.tree);assert.equal(git('status','--porcelain=v1','--untracked-files=all').length,0);};clean();
let journalRoot=root;
const CALLER_UID=1000,CALLER_GID=1000,callerView=root+'/caller-view',callerJournal=output+'/caller-journal';
const save=(name,value)=>{const raw=Buffer.isBuffer(value)?value:Buffer.from(JSON.stringify(value,null,2)+'\n');writeFileSync(join(journalRoot,name),raw,{flag:'wx',mode:0o400});return raw;};
const bound=decode(ordinary(operators+'/BOUND_PUBLIC.json'));assert.equal(bound.format,'zryna.m6-remote-public-binding.v1');assert.deepEqual(bound.source,c.source);for(const p of bound.files)checked(p);
assert.deepEqual(bound.caller,{uid:CALLER_UID,gid:CALLER_GID,supplementaryGroups:[]});assert.equal(bound.callerView,callerView);
const callerPin=path=>{
 assert(path.startsWith(output+'/')&&resolve(path)===path);let parent=dirname(path);
 while(parent===output||parent.startsWith(output+'/')){const s=lstatSync(parent);assert(s.isDirectory()&&!s.isSymbolicLink()&&s.uid===CALLER_UID&&s.gid===CALLER_GID&&!(s.mode&0o022));if(parent===output)break;parent=dirname(parent);}
 const s=lstatSync(path);assert(s.isFile()&&!s.isSymbolicLink()&&s.uid===CALLER_UID&&s.gid===CALLER_GID&&s.nlink===1&&s.size<=2*1024*1024&&!(s.mode&0o7022));const raw=readFileSync(path),after=lstatSync(path);
 for(const key of ['dev','ino','size','uid','gid','mode','nlink','mtimeMs','ctimeMs'])assert.equal(s[key],after[key]);assert.equal(raw.length,s.size);return {path,bytes:raw.length,sha256:sha(raw)};
};
const readonlyCopy=(source,name,executable=false)=>{const before=pin(source),raw=ordinary(source),target=join(callerView,name);writeFileSync(target,raw,{flag:'wx',mode:executable?0o555:0o444});assert.deepEqual(pin(source),before);const actual=pin(target);assert.equal(actual.bytes,before.bytes);assert.equal(actual.sha256,before.sha256);return {original:before,view:actual};};
exact(c.publication,['selection','verifier','acquisition']);const selected=decode(checked(c.publication.selection));
exact(selected,['format','version','status','source','originalSelection','originalVerifierPolicy','inputs']);
assert.equal(selected.format,'zryna.m6-remote-publication-selection.v1');assert.equal(selected.version,1);assert.equal(selected.status,'actual-immutable-publication-inputs-independently-approved');assert.deepEqual(selected.source,identity(c.source));
const original=decode(checked(selected.originalSelection));assert.equal(original.status,selected.status);
const inputNames=['envelope','bundle','archive','policy','verifierPolicy','trustedRoot'];exact(selected.inputs,inputNames);exact(original.inputs,inputNames);
const data={};for(const name of inputNames){data[name]=descriptor(selected.inputs[name]);if(name!=='verifierPolicy')for(const key of ['bytes','sha256'])assert.equal(selected.inputs[name][key],original.inputs[name][key]);}
const originalVerifierRaw=checked(selected.originalVerifierPolicy);for(const key of ['bytes','sha256'])assert.equal(selected.originalVerifierPolicy[key],original.inputs.verifierPolicy[key]);
const verifierPolicy=decode(data.verifierPolicy),originalVerifier=decode(originalVerifierRaw);checked(c.publication.verifier);
assert.deepEqual(verifierPolicy,{...originalVerifier,path:c.publication.verifier.path,ownerUid:0});assert.equal(verifierPolicy.version,'3.0.5');
assert.equal(verifierPolicy.bytes,135112256);assert.equal(verifierPolicy.sha256,'db15cc99e6e4837daabab023742aaddc3841ce57f193d11b7c3e06c8003642b2');
assert.equal(verifierPolicy.bytes,c.publication.verifier.bytes);assert.equal(verifierPolicy.sha256,c.publication.verifier.sha256);
const acquisition=decode(checked(c.publication.acquisition));assert.equal(acquisition.status,'fresh-actual-immutable-publication-verified-awaiting-independent-input-selection');assert.deepEqual(acquisition.source,identity(c.source));
assert.equal(acquisition.assets.length,7);for(const k of ['envelopeSignatureActual','evidenceIndexSignatureActual','completeEvidenceDecoded','replicasRevalidated','originalQualifiedBinaryMatched'])assert.equal(acquisition[k],true);
for(const name of inputNames)for(const key of ['bytes','sha256'])assert.equal(acquisition.inputs[name][key],original.inputs[name][key]);
const policy=decode(data.policy);assert.deepEqual(identity(policy.source),identity(c.source));assert.equal(policy.source.ref,'refs/tags/v0.1.0-playground-compiler.3');
assert.equal(policy.binary.bytes,candidate.compiler.bytes);assert.equal(policy.binary.sha256,candidate.compiler.sha256);
exact(c.documentation,['selection']);const documentation=decode(checked(c.documentation.selection));exact(documentation,['format','version','source','authentication','production']);
assert.equal(documentation.format,'zryna.m6-remote-documentation-selection.v1');assert.equal(documentation.version,1);assert.deepEqual(documentation.source,identity(c.source));const docsAuth=decode(checked(documentation.authentication)),docsProduction=decode(checked(documentation.production));assert(['official-docs-authenticated-and-vendored-bytes-identical','official-docs-authenticated-and-source-bundle-identical'].includes(docsAuth.status));assert.deepEqual(identity(docsAuth.expected.source),identity(c.source));assert.equal(docsProduction.status,'actual-new-source-production-124-route-content-and-header-check-passed');assert.equal(docsProduction.compilerSourceCommit,c.source.commit);
save('PUBLIC_INPUT_SELECTION.json',{publication:c.publication,documentation:c.documentation,selectedInputs:selected.inputs,originalSelection:selected.originalSelection,originalVerifierPolicy:selected.originalVerifierPolicy,scope:'Actual selected immutable publication and documentation bytes; original verifier authentication below remains mandatory.'});
const state=await import(pathToFileURL(operators+'/m6-410-public-publication-state-20261018.mjs'));
const {composition,validatePolicy}=await import(pathToFileURL(repo+'/examples/playground/restricted/daemon/launcher-policy.mjs'));
let completed=0;
try{
 const acquired=await state.initializeActualPublication({policy,verifierPolicy,envelope:data.envelope,bundle:data.bundle,archive:data.archive,trustedRoot:data.trustedRoot});
 save('authenticated-receipt.json',Buffer.from(JSON.stringify(acquired.receipt)));save('actual-verification.json',{command:acquired.verification.command,result:acquired.verification.result});save('actual-verification.stdout',Buffer.from(acquired.verification.stdout));save('actual-verification.stderr',Buffer.from(acquired.verification.stderr));
 const reviewed=decode(ordinary(baseline+'/reviewed-launcher.candidate.json'));assert.deepEqual(identity(reviewed.source),identity(c.source));
 assert.equal(acquired.receipt.source.commit,reviewed.source.commit);assert.equal(acquired.receipt.source.tree,reviewed.source.tree);assert.equal(acquired.receipt.compiler.sha256,reviewed.compiler.sha256);assert.equal(acquired.receipt.compiler.bytes,reviewed.compiler.bytes);
 reviewed.compilerPublication={status:'authenticated',type:acquired.receipt.type,receipt:{...pin(root+'/authenticated-receipt.json'),ownerUid:0}};
 reviewed.compositionSha256=sha(Buffer.from(JSON.stringify(composition(reviewed))));
 const approval=decode(ordinary(baseline+'/approval.candidate.json'));assert.deepEqual(approval.cpus,[0,1]);assert.equal(approval.engine_uid,65534);approval.composition_sha256=reviewed.compositionSha256;
 save('policy-input.public.json',approval);checked(c.prepare.policyModule);
 const policyBytes=execFileSync(m.files.python.path,['-I','-B',c.prepare.policyModule.path,'--record',root+'/policy-input.public.json'],{timeout:10000,maxBuffer:262144});
 approval.policy_sha256=sha(policyBytes);save('complete-policy.public.json',policyBytes);const approvalBytes=save('approval.public.candidate.json',approval);
 reviewed.approval={path:root+'/approval.json',bytes:approvalBytes.length,sha256:sha(approvalBytes),ownerUid:0};validatePolicy(reviewed,'authenticated');
 save('reviewed-launcher.public.json',reviewed);save('source-composition.json',Buffer.from(JSON.stringify(composition(reviewed))));save('ui-approval.candidate.json',ordinary(baseline+'/ui-approval.candidate.json'));save('REMOTE_PUBLIC_CONTEXT.json',checked(m.files.context));
 mkdirSync(callerView,{mode:0o755});const copies=[];
 for(const [source,name,executable]of [[baseline+'/compiler.bin','compiler.bin',true],[reviewed.template.path,'loader.js',false],[root+'/authenticated-receipt.json','authenticated-receipt.json',false],[root+'/source-composition.json','source-composition.json',false],[baseline+'/OWNER_TOOLS.json','OWNER_TOOLS.json',false],[root+'/approval.public.candidate.json','approval.json',false],[root+'/ui-approval.candidate.json','ui-approval.json',false]])copies.push(readonlyCopy(source,name,executable));
 const projected=structuredClone(reviewed);
 for(const [descriptor,name]of [[projected.compiler,'compiler.bin'],[projected.template,'loader.js'],[projected.approval,'approval.json'],[projected.compilerPublication.receipt,'authenticated-receipt.json']]){const actual=pin(callerView+'/'+name);assert.equal(descriptor.bytes,actual.bytes);assert.equal(descriptor.sha256,actual.sha256);descriptor.path=actual.path;descriptor.ownerUid=0;}
 validatePolicy(projected,'authenticated');assert.deepEqual(composition(projected),composition(reviewed));assert.equal(projected.compositionSha256,reviewed.compositionSha256);
 writeFileSync(callerView+'/reviewed-launcher.public.json',JSON.stringify(projected,null,2)+'\n',{flag:'wx',mode:0o444});
 save('CALLER_VIEW.json',{caller:{uid:CALLER_UID,gid:CALLER_GID,supplementaryGroups:[]},root:callerView,copies,projectedDescriptor:pin(callerView+'/reviewed-launcher.public.json'),originalDescriptor:pin(root+'/reviewed-launcher.public.json'),sameOriginalComposition:true,approvals:[{canonical:root+'/approval.json',candidate:pin(root+'/approval.public.candidate.json'),view:pin(callerView+'/approval.json')},{canonical:root+'/ui-approval.json',candidate:pin(root+'/ui-approval.candidate.json'),view:pin(callerView+'/ui-approval.json')}],scope:'Readonly pre-PREP candidate projection; actual root0400 approvals must match after protected tag. No authority is minted by these copies.'});
 clean();for(const p of bound.files)checked(p);mkdirSync(output,{mode:0o700});chownSync(output,CALLER_UID,CALLER_GID);mkdirSync(callerJournal,{mode:0o700});chownSync(callerJournal,CALLER_UID,CALLER_GID);
 save('CALLER_OUTPUT_SELECTION.json',{output,callerJournal,uid:CALLER_UID,gid:CALLER_GID,mode:0o700,scope:'Fresh caller-owned output paths outside immutable preparation inventory; actual future contents are not preapproved pass evidence.'});
 save('APPROVAL_WAIT_INTENT.json',{argv:[m.files.python.path,'-I','-B',m.files.waitApproval.path,manifestPath],timeoutSeconds:m.limits.approvalSeconds});
 // Complete closed-file inventory only. Live Node stdout/stderr remain outside this root.
 const files=[];let total=0;
 const walk=directory=>{for(const name of readdirSync(directory).sort()){const path=join(directory,name),s=lstatSync(path);assert(!s.isSymbolicLink()&&s.uid===0&&!(s.mode&0o022));if(s.isDirectory()){walk(path);continue;}assert(s.isFile());const p=pin(path,0);assert(p.bytes<=134217728);const rel=relative(root,path);assert(!['PREPARATION.json','approval.json','ui-approval.json'].includes(rel));files.push({path:rel,bytes:p.bytes,sha256:p.sha256});total+=p.bytes;assert(total<=536870912);}};walk(root);assert(files.length>=2&&files.length<=2048);
 save('PREPARATION.json',{format:'zryna.m6-remote-preparation.v1',version:1,phase:'public',source:identity(c.source),operatorCommit:m.approval.operatorCommit,run:m.approval.run,files});
 const prepareCommand=pin(m.hostRoot+'/prepare.json');const prepared=pin(root+'/PREPARATION.json');
 writeFileSync(m.hostRoot+'/PREPARED.json',JSON.stringify({format:'zryna.m6-host-public-preparation-completed.v1',approval:false,acceptance:false,manifest:pin(manifestPath),preparation:prepared,prepareCommand,finishedAt:new Date().toISOString()},null,2)+'\n',{flag:'wx',mode:0o400});
 // Fixed child shares this runtime's owned process group. It cannot create the protected tag.
 try{execFileSync(m.files.python.path,['-I','-B',m.files.waitApproval.path,manifestPath],{cwd:repo,timeout:m.limits.approvalSeconds*1000,maxBuffer:1048576,stdio:['ignore','pipe','pipe']});save('APPROVAL_WAIT_RESULT.json',{exit:0});}
 catch(error){save('APPROVAL_WAIT_RESULT.json',{exit:error.status??null,signal:error.signal??null});save('APPROVAL_WAIT.stdout',Buffer.from(error.stdout??[]));save('APPROVAL_WAIT.stderr',Buffer.from(error.stderr??[]));throw error;}
 assert.deepEqual({...pin(root+'/approval.json'),ownerUid:0},reviewed.approval);assert(ordinary(root+'/ui-approval.json').equals(ordinary(root+'/ui-approval.candidate.json')));clean();for(const p of bound.files)checked(p);
 const viewProof=decode(ordinary(root+'/CALLER_VIEW.json'));assert.equal(viewProof.root,callerView);for(const row of viewProof.approvals){assert.equal(lstatSync(row.canonical).mode&0o777,0o400);assert.deepEqual(pin(row.view.path),row.view);assert(ordinary(row.canonical).equals(ordinary(row.view.path)));}for(const row of viewProof.copies){assert.deepEqual(pin(row.original.path),row.original);assert.deepEqual(pin(row.view.path),row.view);}assert.deepEqual(pin(viewProof.projectedDescriptor.path),viewProof.projectedDescriptor);
 const before=decode(Buffer.from(JSON.stringify(Object.fromEntries(readFileSync('/proc/self/status','utf8').trim().split('\n').map(line=>{const at=line.indexOf(':');return [line.slice(0,at),line.slice(at+1).trim()];})))));assert.equal(before.NoNewPrivs,'1');
 save('CALLER_DROP_INTENT.json',{pid:process.pid,fromUid:process.getuid(),fromGid:process.getgid(),toUid:CALLER_UID,toGid:CALLER_GID,supplementaryGroups:[],noNewPrivs:before.NoNewPrivs,preDropCapBnd:before.CapBnd,source:identity(c.source),scope:'Same Node/original opaque WeakMap capability, before any launcher construction. Canonical actual approvals remain root0400.'});
 Object.assign(process.env,{GIT_CONFIG_COUNT:'1',GIT_CONFIG_KEY_0:'safe.directory',GIT_CONFIG_VALUE_0:'/workspace/zryna',GIT_CONFIG_NOSYSTEM:'1',GIT_CONFIG_GLOBAL:'/dev/null',GIT_NO_REPLACE_OBJECTS:'1'});
 process.setgroups([]);process.setgid(CALLER_GID);process.setuid(CALLER_UID);journalRoot=callerJournal;
 const status=Object.fromEntries(readFileSync('/proc/self/status','utf8').trim().split('\n').map(line=>{const at=line.indexOf(':');return [line.slice(0,at),line.slice(at+1).trim()];}));
 assert.deepEqual(status.Uid.split(/\s+/),Array(4).fill(String(CALLER_UID)));assert.deepEqual(status.Gid.split(/\s+/),Array(4).fill(String(CALLER_GID)));assert.equal(status.Groups.trim(),'');assert.equal(status.NoNewPrivs,'1');for(const key of ['CapInh','CapPrm','CapEff','CapAmb'])assert.equal(BigInt('0x'+status[key]),0n);
 save('CALLER_CREDENTIALS.json',{pid:process.pid,uid:process.getuid(),euid:process.geteuid(),gid:process.getgid(),egid:process.getegid(),supplementaryGroups:status.Groups.trim()?status.Groups.trim().split(/\s+/).map(Number):[],nodeGetgroups:process.getgroups(),status,capBoundingMeaning:'Actual residual bounding set is recorded; NNP1 plus all real/effective/saved/fs UIDs1000 and no permitted/effective/inheritable/ambient caps is the original nonroot boundary. Host all-five-zero is not inferred; guests retain original zero-five requirement.',originalCapability:'Same process/module instance retained, never serialized/reissued.',beforeFirstLauncher:true});clean();
 save('ACTUAL_RUN_START.json',{startedAt:new Date().toISOString(),source:identity(c.source),compositionSha256:reviewed.compositionSha256,prepared,scope:'Five actual public cohorts, same genuine original-module capability; no private fixture capability.'});
 const jobs=[['parity','M6_410_ACTUAL_PUBLISHED_HTTP_PARITY','actual-nineteen-cli-http-browser-comparison-matched'],['ui','M6_410_ACTUAL_PUBLISHED_UI','actual-initial-abort-corpus-ax-ui-pass'],['boundaries','M6_410_ACTUAL_PUBLISHED_HTTP_BOUNDARIES','actual-http-boundary-cohort-pass'],['isolation','M6_410_ACTUAL_PUBLISHED_SESSION_ISOLATION','actual-source-session-and-contained-browser-isolation-pass'],['documentation','M6_410_ACTUAL_PUBLISHED_DOCUMENTATION_EXERCISE','actual-published-guide-source-reject-restore-and-browser42-pass']];
 for(const [name,prefix,status]of jobs){const start=new Date().toISOString();save('COHORT_'+name+'_INTENT.json',{name,startedAt:start});await import(pathToFileURL(operators+'/m6-410-run-public-'+name+'-20261018.mjs'));const resultPath=output+'/'+prefix+'_'+c.source.commit+'_RUN1/RESULT.json',actualResultPin=callerPin(resultPath),rawResult=readFileSync(resultPath);assert.equal(rawResult.length,actualResultPin.bytes);assert.equal(sha(rawResult),actualResultPin.sha256);assert.deepEqual(callerPin(resultPath),actualResultPin);const result=decode(rawResult);assert.equal(result.status,status);save('COHORT_'+name+'_COMPLETE.json',{name,startedAt:start,finishedAt:new Date().toISOString(),result:callerPin(resultPath),status});completed++;}
 save('ACTUAL_RUN_COMPLETE.json',{status:'five-original-public-cohorts-complete',completed,source:identity(c.source),finishedAt:new Date().toISOString(),scope:'Public runtime observations completed. Separate independent59 review, read-only cleanup, durable materials and final23 remain required. Original opaque capability discarded on process exit; later relationship validation requires fresh original authentication of identical immutable publication inputs.'});
}catch(error){save('FAILURE.json',{status:'public-runtime-stopped-at-first-failure',completed,source:identity(c.source),type:error.name,message:String(error.message).slice(0,1500),hostFailure:error.hostFailure??null,scope:'No retry, no five-cohort success or qualification claim. Actual underlying cohort failure and original cleanup receipts retained.'});throw error;}
