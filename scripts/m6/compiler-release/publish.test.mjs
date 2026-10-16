import test from 'node:test';import assert from 'node:assert/strict';
import { publishAssets,verifyCurrentArtifacts,verifyReplicaJobs } from './publish.mjs';
import { sha256,TAG } from './common.mjs';
const names=['compiler-envelope.json','compiler-envelope.sigstore.json','candidate-policy.json',
'zryna-playground-compiler-0.1.0-x86_64-unknown-linux-gnu.tar.gz','publication-evidence.tar.gz',
'publication-evidence-index.json','publication-evidence-index.sigstore.json'];
const source={commit:'a'.repeat(40),tree:'b'.repeat(40),tagObject:'c'.repeat(40)};
const files=()=>names.map(path=>({path,data:Buffer.from(`SCHEMA FIXTURE ONLY ${path}\n`)}));
function publisherFixture({existing=false,immutable=true,changedUpload=false,extra=false,drift=false,renamedFinal=false}={}){
  const calls=[],assets=[];const api={request:async(path,options={})=>{
    calls.push({path,options});
    if(path.endsWith('/branches/main'))return{name:'main',protected:true,commit:{sha:drift?'f'.repeat(40):source.commit},protection:{required_status_checks:{
      contexts:['adapter','m0','m2','m3','rust (ubuntu-latest)','rust (windows-latest)'],
      checks:['adapter','m0','m2','m3','rust (ubuntu-latest)','rust (windows-latest)'].map(context=>({context}))}}};
    if(path.endsWith('/releases/tags/v0.2.3'))return{id:388328382,tag_name:'v0.2.3',immutable:true,draft:false};
    if(path.includes('/git/commits/'))return{sha:source.commit,tree:{sha:source.tree}};
    if(path.includes('/git/ref/tags/'))return{ref:`refs/tags/${TAG}`,object:{type:'tag',sha:source.tagObject}};
    if(path.includes('/rulesets/'))return{id:22824582,name:'immutable-release-tags',target:'tag',enforcement:'active',
      conditions:{ref_name:{include:['refs/tags/v*'],exclude:[]}},rules:['creation','update','deletion'].map(type=>({type}))};
    if(path.includes('/releases?'))return existing?[{tag_name:TAG}]:[];
    if(path==='/repos/zryna/zryna/releases' && options.method==='POST')return{id:123,tag_name:TAG,draft:true,prerelease:true};
    if(options.upload){
      const name=new URL(`https://uploads.github.com${path}`).searchParams.get('name');
      const asset={id:assets.length+1,name,state:'uploaded',size:options.body.length,
        digest:`sha256:${changedUpload?'f'.repeat(64):sha256(options.body)}`};assets.push(asset);return Buffer.from(JSON.stringify(asset));
    }
    if(path.includes('/assets?'))return extra?[...assets,{id:999,name:'unexpected'}]:assets;
    if(options.method==='PATCH')return{id:123,draft:false,prerelease:true};
    const actual=assets.map(asset=>({...asset}));
    if(renamedFinal)[actual[0].name,actual[1].name]=[actual[1].name,actual[0].name];
    return{id:123,tag_name:TAG,draft:false,prerelease:true,immutable,assets:actual,html_url:'https://github.com/zryna/zryna/releases/tag/'+TAG};
  }};return{api,calls};
}
test('pure publisher protocol creates one draft and publishes exact seven hashed assets once',async()=>{
  const f=publisherFixture();const result=await publishAssets(f.api,source,files());
  assert.equal(result.immutable,true);assert.equal(result.assets.length,7);
  assert.equal(f.calls.filter(c=>c.options.method==='PATCH').length,1);
  assert.equal(f.calls.some(c=>c.options.method==='DELETE'),false);
});
for(const [name,options,patches]of [['existing final release',{existing:true},0],['wrong uploaded digest',{changedUpload:true},0],
  ['unlisted draft asset',{extra:true},0],['main drift before publication',{drift:true},0],['mutable published result',{immutable:false},1],
  ['swapped final asset names',{renamedFinal:true},1]])
  test(`publisher rejects ${name}`,async()=>{
    const f=publisherFixture(options);await assert.rejects(publishAssets(f.api,source,files()));
    assert.equal(f.calls.filter(c=>c.options.method==='PATCH').length,patches);
    if(options.existing)assert.equal(f.calls.some(c=>c.options.method==='POST'),false);
  });
test('publisher rejects unknown or missing asset before mutation',async()=>{
  const f=publisherFixture(),input=files();input[0].path='unknown';
  await assert.rejects(publishAssets(f.api,source,input));assert.equal(f.calls.length,0);
});
test('current-run artifact handoff rejects expired/cross-source/duplicate transports',async()=>{
  const source={commit:'a'.repeat(40)},run={id:123,attempt:2};
  const artifacts=['compiler-admission','compiler-build-1','compiler-build-2','compiler-assembled'].map((prefix,index)=>({
    id:index+1,name:`${prefix}-123-2`,expired:false,size_in_bytes:3,digest:`sha256:${'b'.repeat(64)}`,
    workflow_run:{id:123,head_sha:source.commit}}));
  const api={request:async()=>({total_count:artifacts.length,artifacts})};
  assert.equal((await verifyCurrentArtifacts(api,{source,run})).length,4);
  artifacts[0].expired=true;await assert.rejects(verifyCurrentArtifacts(api,{source,run}));artifacts[0].expired=false;
  artifacts[0].workflow_run.head_sha='c'.repeat(40);await assert.rejects(verifyCurrentArtifacts(api,{source,run}));
});

test('final successful replica jobs must retain exact distinct source/runner/attempt binding',async()=>{
  const source={commit:'a'.repeat(40)},run={id:123,attempt:2};
  const records=[1,2].map(replica=>({job:{id:replica,name:`independent compiler replica ${replica}`,runnerId:replica+10}}));
  const jobs=records.map(record=>({...record.job,runner_id:record.job.runnerId,run_id:123,run_attempt:2,
    head_sha:source.commit,status:'completed',conclusion:'success'}));
  const api={request:async()=>({total_count:jobs.length,jobs})};
  await verifyReplicaJobs(api,{source,run},records);
  jobs[1].runner_id=99;await assert.rejects(verifyReplicaJobs(api,{source,run},records));jobs[1].runner_id=12;
  jobs[1].conclusion='skipped';await assert.rejects(verifyReplicaJobs(api,{source,run},records));jobs[1].conclusion='success';
  jobs[1].run_attempt=3;await assert.rejects(verifyReplicaJobs(api,{source,run},records));
});

test('publisher-wide failure retains raw captured API bytes before any signing or release action',async()=>{
  const {mkdtempSync,readFileSync,rmSync}=await import('node:fs');
  const {tmpdir}=await import('node:os');const {join}=await import('node:path');
  const {publishCompiler}=await import('./publish.mjs');
  const root=mkdtempSync(join(tmpdir(),'m6-publisher-failure-fixture-')),output=join(root,'publication');
  const data=Buffer.from('{"fixture":"raw-authenticated-response-simulation"}\n');
  try{
    await assert.rejects(publishCompiler({output,environment:{GITHUB_EVENT_NAME:'workflow_dispatch'},
      api:{captures:[{method:'GET',path:'/repos/zryna/zryna/fixture',status:403,bytes:data.length,sha256:sha256(data),data}]}}));
    assert(readFileSync(`${output}-observations/api-001.json`).equals(data));
    const proof=JSON.parse(readFileSync(`${output}-observations/failure.json`));assert.equal(proof.status,'publisher-failed');
  }finally{rmSync(root,{recursive:true,force:true});}
});
