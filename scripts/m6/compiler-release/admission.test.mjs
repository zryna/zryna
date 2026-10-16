import test from 'node:test';import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';import { parse } from 'yaml';
import { bytes, sha256 } from './common.mjs';
import { parseTag } from './source.mjs';
import { openQualification,validateQualification,createQualificationPacket,qualificationRegistry } from './qualification.mjs';
import { qualificationFixture } from './qualification-fixture.mjs';
import { observeCi } from './gates.mjs';
const trailers=['M6-Full-CI-Run:123','M6-Full-CI-Attempt:1','M6-Qualification-Asset:456',
  `M6-Qualification-SHA256:${'d'.repeat(64)}`,`M6-Qualification-Review-SHA256:${'e'.repeat(64)}`];
const tag=lines=>Buffer.from(`object ${'a'.repeat(40)}\ntype commit\ntag v0.1.0-playground-compiler.1\ntagger Fixture <fixture@example.invalid> 1 +0000\n\nFixture tag parsing only\n\n${lines.join('\n')}\n`);
test('protected tag binds exactly five closed positive selections',()=>{
  assert.deepEqual(parseTag(tag(trailers)).selection,{ciRun:123,ciAttempt:1,qualificationAsset:456,
    qualificationSha256:'d'.repeat(64),reviewSha256:'e'.repeat(64)});
});
for(const [name,mutate] of [
  ['missing',lines=>lines.slice(1)],['duplicate',lines=>[...lines.slice(0,4),lines[0]]],
  ['unknown',lines=>[...lines.slice(0,4),'M6-Remote-URL:https://example.invalid']],
  ['zero',lines=>['M6-Full-CI-Run:0',...lines.slice(1)]],
  ['unsafe numeric',lines=>['M6-Full-CI-Run:9999999999999999',...lines.slice(1)]],
  ['mixed-case hash',lines=>[...lines.slice(0,4),`M6-Qualification-Review-SHA256:${'E'.repeat(64)}`]],
])test(`tag rejects ${name} admission`,()=>assert.throws(()=>parseTag(tag(mutate(trailers)))));
test('wrong tag or nested tag cannot select admission',()=>{
  assert.throws(()=>parseTag(Buffer.from(tag(trailers).toString().replace('type commit','type tag'))));
  assert.throws(()=>parseTag(Buffer.from(tag(trailers).toString().replace('compiler.1','compiler.2'))));
});
test('candidate schema and packet own exact complete fixture bytes without activation',async()=>{
  const f=qualificationFixture();
  assert.equal(validateQualification(f.record,f.review,f.files,f).status,'candidate-qualified-not-published');
  const packet=await createQualificationPacket(f.record,f.review,f.files);
  const opened=await openQualification(packet,{...f,selection:{...f.selection,qualificationSha256:sha256(packet)}});
  assert.equal(opened.files.length,3);assert.equal(opened.record.obligations.length,qualificationRegistry.obligations.length);
});
for(const [name,mutate] of [
  ['missing owning case',f=>f.record.obligations.pop()],['skip',f=>f.record.obligations[0].status='skip'],
  ['invented case',f=>f.record.obligations[0].id='invented'],['wrong binary',f=>f.compiler={...f.compiler,bytes:124}],
  ['wrong source',f=>f.source={...f.source,tree:'f'.repeat(40)}],
  ['unlisted raw',f=>f.files.push({path:'raw/extra',mode:0o644,data:Buffer.from('extra')})],
  ['modified raw',f=>f.files[0].data=Buffer.from('changed')],
  ['wrong publication',f=>f.record.publishedInputs.archiveSha256='f'.repeat(64)],
  ['unbound review',f=>f.selection.reviewSha256='f'.repeat(64)],
  ['no readonly proof',f=>f.review.confirmation={...f.review.confirmation,path:'raw/missing'}],
  ['unknown fields',f=>f.record.callerURL='https://example.invalid'],
])test(`candidate fails closed for ${name}`,()=>{
  const f=qualificationFixture();mutate(f);
  f.review.qualificationSha256=sha256(bytes(f.record));
  f.review.rawInventorySha256=sha256(bytes(f.record.files));
  if(name!=='unbound review')f.selection.reviewSha256=sha256(bytes(f.review));
  assert.throws(()=>validateQualification(f.record,f.review,f.files,f));
});
test('packet does not accept bytes without the maintainer-selected packet digest',async()=>{
  const f=qualificationFixture(),packet=await createQualificationPacket(f.record,f.review,f.files);
  await assert.rejects(openQualification(packet,{...f,selection:{...f.selection,qualificationSha256:'f'.repeat(64)}}));
});
function ciFixture(){
  const commit='a'.repeat(40),tree='b'.repeat(40),runId=12,attempt=2;
  const job={id:34,name:'required owning job',run_id:runId,run_attempt:attempt,head_sha:commit,
    status:'completed',conclusion:'success',check_run_url:'https://api.github.com/repos/zryna/zryna/check-runs/56'};
  const run={id:runId,run_attempt:attempt,repository:{full_name:'zryna/zryna'},path:'.github/workflows/ci.yml',
    event:'workflow_dispatch',head_branch:'main',head_sha:commit,status:'completed',conclusion:'success'};
  const check={id:56,head_sha:commit,status:'completed',conclusion:'success',app:{slug:'github-actions'}};
  return {source:{commit,tree},runId,attempt,job,run,check,inventory:{jobs:[{name:job.name}]}};
}
for(const [name,mutate] of [
 ['success',()=>{}],['wrong attempt',f=>f.run.run_attempt=1],['PR run',f=>f.run.event='pull_request'],
 ['skipped job',f=>f.job.conclusion='skipped'],['wrong source',f=>f.job.head_sha='c'.repeat(40)],
 ['wrong check source',f=>f.check.head_sha='c'.repeat(40)],['wrong check app',f=>f.check.app.slug='unknown'],
])test(`full CI ${name}`,async()=>{
  const f=ciFixture();mutate(f);const api={request:async path=>path.includes('/jobs?')?{total_count:1,jobs:[f.job]}:
    path.includes('/check-runs/')?f.check:f.run};
  const promise=observeCi(api,f.source,f.runId,f.attempt,f.inventory);
  if(name==='success')assert.equal((await promise).headTree,f.source.tree);else await assert.rejects(promise);
});
test('publisher write and OIDC permissions are unavailable to dispatch/build jobs',()=>{
  const workflow=parse(readFileSync(new URL('../../../.github/workflows/playground-compiler-release.yml',import.meta.url),'utf8'));
  assert.deepEqual(workflow.permissions,{contents:'read',actions:'read',checks:'read'});
  for(const [name,job]of Object.entries(workflow.jobs)){
    if(name==='publish'){
      assert.equal(job.permissions.contents,'write');assert.equal(job.permissions['id-token'],'write');
      assert.equal(job.permissions.checks,'read');
      assert.match(job.if,/github.event_name == 'push'/);assert.equal(job.environment,undefined);
    }else assert.equal(job.permissions,undefined);
    for(const step of job.steps??[])if(step.uses)assert.match(step.uses,/@[a-f0-9]{40}$/);
  }
});

test('qualification obligation list cannot be mutated after its registry pin',()=>{
  assert.throws(()=>qualificationRegistry.obligations.push('forged'));
});
