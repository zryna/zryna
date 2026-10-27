import test from 'node:test';import assert from 'node:assert/strict';
import { readFileSync,writeFileSync,existsSync,readdirSync,rmSync } from 'node:fs';
import { join } from 'node:path';import { createHash } from 'node:crypto';import { fileURLToPath } from 'node:url';
import { phaseFixture } from './phases-fixture.mjs';
import { assembleCompiler } from './assemble.mjs';import { prepareAdmission } from './admission.mjs';
import { publishCompiler,verifyAdmissionAgreement } from './publish.mjs';
import { expectedJobs } from './gates.mjs';import { bytes,sha256,REF,WORKFLOW,TAG } from './common.mjs';
const repo=fileURLToPath(new URL('../../../',import.meta.url));
const assembly=(f,output,extra={})=>assembleCompiler({sourceRoot:f.sourceRoot,admissionRoot:f.admissionRoot,
  firstRoot:f.roots[0],secondRoot:f.roots[1],output:join(f.root,output),...extra});
async function withFixture(body){const f=await phaseFixture();try{await body(f);}finally{rmSync(f.root,{recursive:true,force:true});}}

test('read-only explicit deferred assembly yields identical three unsigned files without a private packet',()=>withFixture(async f=>{
  assert.equal((await assembly(f,'deferred',{deferQualification:true})).status,'unsigned-unqualified-publication');
  assert.equal(existsSync(join(f.admissionRoot,'qualification.tar.gz')),false);
  writeFileSync(join(f.admissionRoot,'admission.json'),bytes(f.qualified));writeFileSync(join(f.admissionRoot,'qualification.tar.gz'),f.packet);
  await assembly(f,'qualified');
  for(const name of ['compiler-envelope.json','candidate-policy.json','zryna-playground-compiler-0.1.0-x86_64-unknown-linux-gnu.tar.gz'])
    assert(readFileSync(join(f.root,'deferred',name)).equals(readFileSync(join(f.root,'qualified',name))));
}));
test('default assembly rejects deferred source admission before producing an archive',()=>withFixture(async f=>{
  await assert.rejects(assembly(f,'closed'),/explicit admission phase/);assert.equal(existsSync(join(f.root,'closed')),false);
}));
for(const [name,change]of [
  ['qualified admission',f=>f.admission=f.qualified],['false publishing',f=>f.admission.publishing=false],
  ['unexpected qualification',f=>f.admission.qualification={}],
])test(`explicit deferred assembly rejects ${name}`,()=>withFixture(async f=>{
  change(f);writeFileSync(join(f.admissionRoot,'admission.json'),bytes(f.admission));
  await assert.rejects(assembly(f,'closed',{deferQualification:true}),/explicit admission phase/);
}));
for(const problem of ['missing packet','wrong binary','missing case'])test(`default qualified assembly rejects ${problem} before archive output`,()=>withFixture(async f=>{
  writeFileSync(join(f.admissionRoot,'admission.json'),bytes(f.qualified));
  if(problem!=='missing packet'){
    const {qualificationFixture}=await import('./qualification-fixture.mjs');const {createQualificationPacket}=await import('./qualification.mjs');
    const q=qualificationFixture();q.record.source={repository:f.admission.source.repository,commit:f.admission.source.commit,tree:f.admission.source.tree};
    if(problem==='missing case'){q.record.compiler=f.qualified.qualification.compiler;q.record.obligations.pop();}
    q.review.source=q.record.source;q.review.compiler=q.record.compiler;q.review.qualificationSha256=sha256(bytes(q.record));
    q.review.rawInventorySha256=sha256(bytes(q.record.files));const packet=await createQualificationPacket(q.record,q.review,q.files);
    f.qualified.selection.qualificationSha256=sha256(packet);f.qualified.selection.reviewSha256=sha256(bytes(q.review));
    writeFileSync(join(f.admissionRoot,'admission.json'),bytes(f.qualified));writeFileSync(join(f.admissionRoot,'qualification.tar.gz'),packet);
  }
  await assert.rejects(assembly(f,'failed'));assert.equal(existsSync(join(f.root,'failed','compiler-envelope.json')),false);
}));
test('manual candidate retains exactly its four original unqualified outputs',()=>withFixture(async f=>{
  writeFileSync(join(f.admissionRoot,'admission.json'),bytes({...f.admission,publishing:false,status:'candidate-build-only'}));
  await assembly(f,'manual',{deferQualification:true});assert.deepEqual(readdirSync(join(f.root,'manual')).sort(),
    ['candidate.json','compiler.bin','reproduction.json','rust-materials.json']);
  assert.equal(JSON.parse(readFileSync(join(f.root,'manual','candidate.json'))).status,'unpublished-unqualified-candidate');
}));
for(const [name,change]of [
  ['source',(_,o)=>o.source={...o.source,tree:'f'.repeat(40)}],['selection',(_,o)=>o.selection={...o.selection,qualificationAsset:999}],
  ['run',(_,o)=>o.run={...o.run,id:999}],['original qualified',(_,o)=>{o.status='protected-tag-admitted';o.qualification={};}],
  ['fresh deferred',f=>{f.status='protected-tag-source-admitted';f.qualification=null;}],['fake fresh qualification',f=>f.qualification={}],
  ['wrong fresh packet',f=>f.qualification.packet.sha256='f'.repeat(64)],
])test(`publisher phase agreement rejects ${name}`,()=>withFixture(async f=>{
  const fresh=structuredClone(f.qualified),original=structuredClone(f.admission);change(fresh,original);
  assert.throws(()=>verifyAdmissionAgreement(fresh,original));
}));
test('publisher agrees only on fresh qualification and matching deferred source selection/run',()=>withFixture(async f=>{
  verifyAdmissionAgreement(f.qualified,f.admission);
}));

// Authenticated API/Git schema simulation only: no network, tag, builds, signer or release.
async function admissionApi(f,privateFailure='403'){
  const source=f.admission.source,selection=f.admission.selection,calls=[],captures=[];
  let inventory;
  const request=async(path,options={})=>{
    calls.push({path,options});let data;
    if(path.includes('/contents/')){const rel=path.split('/contents/')[1].split('?')[0],raw=readFileSync(join(repo,rel));
      data={type:'file',path:rel,encoding:'base64',size:raw.length,content:raw.toString('base64'),
        sha:createHash('sha1').update(Buffer.from(`blob ${raw.length}\0`)).update(raw).digest('hex')};}
    else if(path.endsWith('/branches/main'))data={name:'main',protected:true,commit:{sha:source.commit},protection:{required_status_checks:{
      contexts:['adapter','m0','m2','m3','rust (ubuntu-latest)','rust (windows-latest)'],
      checks:['adapter','m0','m2','m3','rust (ubuntu-latest)','rust (windows-latest)'].map(context=>({context}))}}};
    else if(path.endsWith('/releases/tags/v0.2.3'))data={id:388328382,tag_name:'v0.2.3',immutable:true,draft:false};
    else if(path.includes('/git/commits/'))data={sha:source.commit,tree:{sha:source.tree}};
    else if(path.includes('/git/ref/tags/'))data={ref:REF,object:{type:'tag',sha:source.tagObject}};
    else if(path.includes('/rulesets/'))data={id:22824582,name:'immutable-release-tags',target:'tag',enforcement:'active',
      conditions:{ref_name:{include:['refs/tags/v*'],exclude:[]}},rules:['creation','update','deletion'].map(type=>({type}))};
    else if(path.includes('/jobs?'))data={total_count:inventory.jobs.length,jobs:inventory.jobs.map((job,i)=>({id:100+i,name:job.name,
      run_id:1,run_attempt:1,head_sha:source.commit,status:'completed',conclusion:'success',check_run_url:'https://api.github.com/repos/zryna/zryna/check-runs/'+(200+i)}))};
    else if(path.includes('/check-runs/'))data={id:Number(path.split('/').at(-1)),head_sha:source.commit,status:'completed',conclusion:'success',app:{slug:'github-actions'}};
    else if(path.includes('/actions/runs/'))data={id:1,run_attempt:1,repository:{full_name:'zryna/zryna'},path:'.github/workflows/ci.yml',
      event:'workflow_dispatch',head_branch:'main',head_sha:source.commit,status:'completed',conclusion:'success'};
    else if(path.includes('/releases/assets/')){
      if(privateFailure==='403'){captures.push({method:'GET',path,status:403,data:Buffer.from('{"message":"Resource not accessible by integration"}\n')});throw Error('compiler release API status 403');}
      data={id:456,name:'m6-compiler-qualification.tar.gz',state:'uploaded',size:f.packet.length,digest:'sha256:'+sha256(f.packet)};
    }else if(path.includes('/releases?'))data=privateFailure==='missing-draft'?[]:[{id:789,draft:true,tag_name:'m6-qualification-fixture',assets:[{id:456}]}];
    else throw Error('Unexpected API '+path);
    captures.push({method:'GET',path,status:200,data:bytes(data)});return data;
  };
  const api={request,captures,asset:async()=>privateFailure==='wrong-packet'?Buffer.from('WRONG PRIVATE PACKET'):f.packet};
  inventory=await expectedJobs(repo,api,source);assert.equal(inventory.jobs.length,44);
  assert.deepEqual(inventory.jobs.filter(job=>job.name.startsWith('m7 generic (')).map(job=>job.name).sort(),
    ['m7 generic (ubuntu-latest)','m7 generic (windows-latest)']);
  calls.length=0;captures.length=0;
  const eventPath=join(f.root,'event.json');writeFileSync(eventPath,bytes({ref:REF,created:true,deleted:false,forced:false,repository:{full_name:'zryna/zryna'}}));
  const environment={GITHUB_EVENT_NAME:'push',GITHUB_REF:REF,GITHUB_REF_TYPE:'tag',GITHUB_REF_PROTECTED:'true',
    GITHUB_REPOSITORY:'zryna/zryna',GITHUB_SHA:source.commit,GITHUB_WORKFLOW_SHA:source.commit,
    GITHUB_WORKFLOW_REF:'zryna/zryna/'+WORKFLOW+'@'+REF,GITHUB_EVENT_PATH:eventPath,GITHUB_RUN_ID:'12',GITHUB_RUN_ATTEMPT:'1'};
  const rawTag=Buffer.from(`object ${source.commit}\ntype commit\ntag ${TAG}\ntagger Fixture <fixture@example.invalid> 1 +0000\n\nFixture only\n\n`+
    [`M6-Full-CI-Run:${selection.ciRun}`,`M6-Full-CI-Attempt:${selection.ciAttempt}`,`M6-Qualification-Asset:${selection.qualificationAsset}`,
      `M6-Qualification-SHA256:${selection.qualificationSha256}`,`M6-Qualification-Review-SHA256:${selection.reviewSha256}`].join('\n')+'\n');
  const spawn=(_,args)=>{
    let stdout;if(args[0]==='status')stdout=Buffer.alloc(0);
    else if(args[0]==='rev-parse')stdout=Buffer.from((args[1]==='HEAD'?source.commit:source.tree)+'\n');
    else if(args[0]==='show-ref')stdout=Buffer.from(source.tagObject+'\n');
    else if(args[0]==='cat-file')stdout=args[1]==='-t'?Buffer.from('tag\n'):rawTag;
    else if(args[0]==='show')stdout=args[1]==='-s'?Buffer.from(source.sourceDateEpoch+'\n'):readFileSync(join(repo,WORKFLOW));
    else throw Error('Unexpected Git '+args);
    return{status:0,signal:null,stdout,stderr:Buffer.alloc(0)};
  };
  return{api,environment,spawn,calls};
}
test('explicit source-only admission never accesses a draft inaccessible to contents-read',()=>withFixture(async f=>{
  const setup=await admissionApi(f);const admission=await prepareAdmission({...setup,sourceRoot:repo,output:join(f.root,'admitted'),deferQualification:true});
  assert.equal(admission.status,'protected-tag-source-admitted');assert.equal(admission.qualification,null);
  assert.equal(setup.calls.some(c=>c.path.includes('/releases/assets/')||c.path.includes('/releases?')),false);
}));
test('fresh fully-qualified admission output agrees with the deferred handoff and retains exact packet bytes',()=>withFixture(async f=>{
  const setup=await admissionApi(f,'valid'),output=join(f.root,'fresh');
  const fresh=await prepareAdmission({...setup,sourceRoot:repo,output,deferQualification:false});
  assert.equal(fresh.status,'protected-tag-admitted');verifyAdmissionAgreement(fresh,f.admission);
  assert(readFileSync(join(output,'qualification.tar.gz')).equals(f.packet));
}));
test('default admission stays closed on private packet403 and retains that failure',()=>withFixture(async f=>{
  const setup=await admissionApi(f);const output=join(f.root,'failed');
  await assert.rejects(prepareAdmission({...setup,sourceRoot:repo,output}),/status 403/);
  assert.equal(existsSync(join(output,'admission.json')),false);assert.equal(JSON.parse(readFileSync(join(output,'failure.json'))).status,'admission-failed');
}));
for(const problem of ['403','missing-draft','wrong-packet'])test(`publisher rejects private qualification ${problem} before signer or release mutation`,()=>withFixture(async f=>{
  const setup=await admissionApi(f,problem),output=join(f.root,'publication');
  await assert.rejects(publishCompiler({...setup,sourceRoot:repo,admissionRoot:f.admissionRoot,output,deferQualification:true}));
  assert.equal(existsSync(output+'-signer'),false);assert.equal(existsSync(output),false);
  assert(setup.calls.every(c=>!c.options.method||c.options.method==='GET'));
  assert.equal(JSON.parse(readFileSync(output+'-observations/failure.json')).status,'publisher-failed');
}));
