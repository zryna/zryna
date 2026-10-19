// Fixed private qualification entry after the separately verified protected-tag gate.
import assert from 'node:assert/strict';
import {readFileSync,lstatSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawn} from 'node:child_process';
import {join} from 'node:path';
assert.equal(process.argv.length,3);assert.equal(process.getuid(),0);assert.equal(process.version,'v22.22.1');
const sha=b=>createHash('sha256').update(b).digest('hex');
const manifestBytes=readFileSync(process.argv[2]);assert.equal(sha(manifestBytes),process.env.M6_REMOTE_MANIFEST_SHA256);
const m=JSON.parse(manifestBytes);assert.equal(m.approval.phase,'qualification');
assert.equal(process.env.GITHUB_RUN_ID,m.run.id);assert.equal(process.env.GITHUB_RUN_ATTEMPT,m.run.attempt);
assert.equal(process.env.GITHUB_SHA,m.approval.operatorCommit);
const root=m.approval.root,contextPath=join(root,'REMOTE_RUNTIME_CONTEXT.json'),contextBytes=readFileSync(contextPath);
assert.equal(contextBytes.length,m.files.context.bytes);assert.equal(sha(contextBytes),m.files.context.sha256);
const context=JSON.parse(contextBytes);assert.deepEqual(context.source,m.source);
const acceptance=JSON.parse(readFileSync(join(root,'APPROVAL_ACCEPTANCE.json')));
assert.equal(acceptance.format,'zryna.m6-remote-approval-consumption.v1');assert.equal(acceptance.guestExecution,false);
assert.equal(acceptance.tag,`v-m6-410-qualification-approval-${m.run.id}-${m.run.attempt}`);
const preparedPath=join(root,m.approval.prepared),preparedBytes=readFileSync(preparedPath),prepared=JSON.parse(preparedBytes);
assert.deepEqual(acceptance.prepared,{bytes:preparedBytes.length,sha256:sha(preparedBytes)});
const operatorRoot=join(root,'runtime-operators'),boundPath=join(operatorRoot,'BOUND_MANIFEST.json');
assert.equal(context.runtime.operatorRoot,operatorRoot);
const approvedFile=path=>{const relative=path.slice(root.length+1),row=prepared.files.find(r=>r.path===relative);
 assert(row&&path.startsWith(root+'/'));const data=readFileSync(path),s=lstatSync(path);
 assert(s.isFile()&&!s.isSymbolicLink()&&s.uid===0&&!(s.mode&0o022));assert.equal(data.length,row.bytes);assert.equal(sha(data),row.sha256);return data;};
const bound=JSON.parse(approvedFile(boundPath));assert.deepEqual(bound.source,m.source);
assert.equal(bound.fullQualification,false);assert.equal(bound.phase,'qualification');
for(const pair of m.approval.approved){const approved=readFileSync(join(root,pair.output)),candidate=readFileSync(join(root,pair.candidate));
 assert(approved.equals(candidate));assert.equal(lstatSync(join(root,pair.output)).mode&0o777,0o400);}
const runner=join(operatorRoot,'runner','run-cohorts.py');approvedFile(runner);approvedFile(join(operatorRoot,'runner','runtime_support.py'));
assert.equal(bound.interpreters.node.path,process.execPath);assert.equal(bound.interpreters.python.path,m.files.python.path);
const expectedCaller={uid:1000,gid:1000,supplementaryGroups:[]}; // ROOT-selected actual nonroot profile; preflight39180547128 retained separately.
assert.deepEqual(bound.caller,expectedCaller);assert([1000,65534].includes(bound.caller.uid));
assert.equal(bound.callerView.root,join(root,'caller-view'));
for(const pair of bound.callerView.approvals){
 assert(m.approval.approved.some(p=>join(root,p.output)===pair.canonical));
 assert(pair.view.path.startsWith(bound.callerView.root+'/'));
 const view=approvedFile(pair.view.path),actual=readFileSync(pair.canonical);
 assert.equal(view.length,pair.view.bytes);assert.equal(sha(view),pair.view.sha256);assert(view.equals(actual));
}
assert.equal(bound.callerView.approvals.length,2);
assert.deepEqual(bound.callerView.approvals.map(p=>p.canonical).sort(),m.approval.approved.map(p=>join(root,p.output)).sort());
const child=spawn(m.files.python.path,['-I','-B',runner,process.argv[2]],{cwd:'/workspace/zryna',env:process.env,stdio:'inherit',shell:false});
const result=await new Promise((resolve,reject)=>{child.once('error',reject);child.once('close',(code,signal)=>resolve({code,signal}));});
assert.deepEqual(result,{code:0,signal:null});
