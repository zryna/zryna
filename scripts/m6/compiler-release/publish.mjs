import { join } from 'node:path';
import { existsSync, mkdirSync } from 'node:fs';
import { bytes, describe, ordinary, parseCanonical, requireValue, sha256, writeNew, TAG, id } from './common.mjs';
import { github } from './github.mjs';
import { observeMain } from './gates.mjs';
import { prepareAdmission } from './admission.mjs';
import { compareReplicas } from './replicas.mjs';
import { assembleCompiler } from './assemble.mjs';
import { acquireSigner, signPublicationBlob } from './signing.mjs';
import { retainPublicationEvidence, evidenceIndex } from './evidence.mjs';
import { directoryFiles } from './files.mjs';
export async function verifyCurrentArtifacts(api,admission) {
  const {id:runId,attempt}=admission.run;
  const response=await api.request(`/repos/zryna/zryna/actions/runs/${runId}/artifacts?per_page=100`);
  requireValue(Array.isArray(response.artifacts) && response.total_count===response.artifacts.length &&
    response.artifacts.length<=100,'compiler complete same-run artifact listing');
  const names=[`compiler-admission-${runId}-${attempt}`,`compiler-build-1-${runId}-${attempt}`,
    `compiler-build-2-${runId}-${attempt}`,`compiler-assembled-${runId}-${attempt}`];
  return names.map(name=>{
    const matches=response.artifacts.filter(a=>a.name===name);requireValue(matches.length===1,'compiler unique same-run artifact');
    const artifact=matches[0];
    requireValue(artifact.expired===false && artifact.workflow_run?.id===runId &&
      artifact.workflow_run.head_sha===admission.source.commit &&
      /^sha256:[a-f0-9]{64}$/.test(artifact.digest) && Number.isSafeInteger(artifact.size_in_bytes) &&
      artifact.size_in_bytes>0 && artifact.size_in_bytes<=256*1024*1024,'compiler actual same-source artifact identity');
    return {id:id(artifact.id),name,digest:artifact.digest,bytes:artifact.size_in_bytes};
  });
}
export async function verifyReplicaJobs(api,admission,records) {
  const {id:runId,attempt}=admission.run;
  const response=await api.request(`/repos/zryna/zryna/actions/runs/${runId}/attempts/${attempt}/jobs?per_page=100`);
  requireValue(Array.isArray(response.jobs) && response.total_count===response.jobs.length &&
    response.jobs.length<=100,'compiler final complete replica job observation');
  for(const record of records) {
    const matches=response.jobs.filter(job=>job.id===record.job.id);
    requireValue(matches.length===1 && matches[0].name===record.job.name && matches[0].runner_id===record.job.runnerId &&
      matches[0].run_id===runId && matches[0].run_attempt===attempt && matches[0].head_sha===admission.source.commit &&
      matches[0].status==='completed' && matches[0].conclusion==='success','compiler final actual successful replica job');
  }
}
export async function publishAssets(api,source,files) {
  const names=files.map(file=>file.path).sort();
  const wanted=['compiler-envelope.json','compiler-envelope.sigstore.json','candidate-policy.json',
    'zryna-playground-compiler-0.1.0-x86_64-unknown-linux-gnu.tar.gz','publication-evidence.tar.gz',
    'publication-evidence-index.json','publication-evidence-index.sigstore.json'].sort();
  requireValue(bytes(names).equals(bytes(wanted)),
    'compiler exact finite publication assets');
  for(let page=1;page<=10;page++) {
    const releases=await api.request(`/repos/zryna/zryna/releases?per_page=100&page=${page}`);
    requireValue(Array.isArray(releases) && releases.length<=100 && !releases.some(r=>r.tag_name===TAG),
      'compiler final release must never already exist');
    if(releases.length<100)break;requireValue(page<10,'compiler complete prepublication release listing');
  }
  const release=await api.request('/repos/zryna/zryna/releases',{method:'POST',status:201,
    body:{tag_name:TAG,target_commitish:source.commit,name:'Zryna playground compiler 0.1.0 prerelease',
      body:`Compiler-only prerelease from ${source.commit}. Source composition and final #410 acceptance remain separate.`,
      draft:true,prerelease:true,generate_release_notes:false}});
  requireValue(release.tag_name===TAG && release.draft===true && release.prerelease===true,
    'compiler newly created draft prerelease');
  const releaseId=id(release.id), expected=[];
  for(const file of files) {
    const uploaded=await api.request(`/repos/zryna/zryna/releases/${releaseId}/assets?name=${encodeURIComponent(file.path)}`,
      {method:'POST',status:201,binary:true,upload:true,body:file.data,maximum:2*1024*1024});
    const result=JSON.parse(uploaded);
    requireValue(result.name===file.path && result.state==='uploaded' && result.size===file.data.length &&
      result.digest===`sha256:${sha256(file.data)}`,'compiler uploaded immutable asset identity');
    expected.push({id:id(result.id),name:file.path,bytes:file.data.length,sha256:sha256(file.data)});
  }
  const listed=await api.request(`/repos/zryna/zryna/releases/${releaseId}/assets?per_page=100`);
  requireValue(Array.isArray(listed) && bytes(listed.map(a=>a.name).sort()).equals(bytes(names)) &&
    expected.every(e=>listed.some(a=>a.id===e.id && a.name===e.name && a.size===e.bytes && a.digest===`sha256:${e.sha256}`)),
  'compiler complete draft asset inventory');
  await observeMain(api,source,{tagged:true});
  const published=await api.request(`/repos/zryna/zryna/releases/${releaseId}`,
    {method:'PATCH',body:{draft:false,prerelease:true}});
  requireValue(published.id===releaseId && published.draft===false && published.prerelease===true,
    'compiler single publication transition');
  const actual=await api.request(`/repos/zryna/zryna/releases/${releaseId}`);
  requireValue(actual.id===releaseId && actual.tag_name===TAG && actual.draft===false &&
    actual.prerelease===true && actual.immutable===true && Array.isArray(actual.assets) &&
    bytes(actual.assets.map(a=>a.name).sort()).equals(bytes(names)) &&
    expected.every(e=>actual.assets.some(a=>a.id===e.id && a.name===e.name && a.size===e.bytes && a.digest===`sha256:${e.sha256}`)),
  'compiler actual immutable published prerelease');
  return {releaseId,url:actual.html_url,assets:expected,immutable:true};
}
function saveProof(root,name,proof) {
  writeNew(join(root,`${name}.command.json`),bytes(proof.signing.record));
  writeNew(join(root,`${name}.stdout`),proof.signing.stdout);writeNew(join(root,`${name}.stderr`),proof.signing.stderr);
  writeNew(join(root,`${name}.verification.json`),bytes({command:proof.verification.command,result:proof.verification.result}));
  writeNew(join(root,`${name}.verify.stdout`),proof.verification.stdout);
  writeNew(join(root,`${name}.verify.stderr`),proof.verification.stderr);
}
async function publishCompilerCore({sourceRoot,admissionRoot,firstRoot,secondRoot,assembledRoot,
  output,environment=process.env,api=github()}={}) {
  requireValue(environment.GITHUB_EVENT_NAME==='push','compiler publisher push-only authority');
  const freshRoot=`${output}-admission`,signerRoot=`${output}-signer`;
  const fresh=await prepareAdmission({sourceRoot,output:freshRoot,environment,api});
  const original=parseCanonical(ordinary(join(admissionRoot,'admission.json')));
  requireValue(fresh.publishing && bytes(fresh.source).equals(bytes(original.source)) &&
    bytes(fresh.selection).equals(bytes(original.selection)) && bytes(fresh.run).equals(bytes(original.run)),
  'compiler fresh original admission agreement');
  const laterStart=api.captures.length;
  const artifactProof=await verifyCurrentArtifacts(api,fresh);
  const replicas=compareReplicas(firstRoot,secondRoot,fresh);
  await verifyReplicaJobs(api,fresh,[replicas.first.record,replicas.second.record]);
  writeNew(join(freshRoot,'pipeline-artifacts.json'),bytes(artifactProof));
  const publication=await assembleCompiler({sourceRoot,admissionRoot:freshRoot,firstRoot,secondRoot,output});
  for(const path of ['compiler-envelope.json','candidate-policy.json',publication.document.archive.path]) {
    requireValue(ordinary(join(output,path),513*1024*1024).equals(ordinary(join(assembledRoot,path),513*1024*1024)),
      'compiler publisher independently reassembled exact artifact');
  }
  const signer=await acquireSigner({output:signerRoot});
  const envelopeProof=signPublicationBlob({signer,path:join(output,'compiler-envelope.json'),
    bundlePath:join(output,'compiler-envelope.sigstore.json'),sourceCommit:fresh.source.commit,environment});
  saveProof(freshRoot,'envelope-signature',envelopeProof);
  await observeMain(api,fresh.source,{tagged:true});
  const late=api.captures.slice(laterStart).map(({data,...record},index)=>{
    const path=`late-api-${String(index+1).padStart(3,'0')}.json`;writeNew(join(freshRoot,path),data);
    return {...record,file:describe(path,data)};
  });
  writeNew(join(freshRoot,'later-api-observations.json'),bytes(late));
  const evidence=await retainPublicationEvidence({admissionRoot:freshRoot,firstRoot,secondRoot,assembledRoot,
    output,source:fresh.source});
  const subjects=['compiler-envelope.json','compiler-envelope.sigstore.json','candidate-policy.json',
    publication.document.archive.path].map(path=>describe(path,ordinary(join(output,path),513*1024*1024)));
  const index=evidenceIndex(fresh.source,publication.document.binary,[...subjects,evidence]);
  writeNew(join(output,'publication-evidence-index.json'),index);
  const indexProof=signPublicationBlob({signer,path:join(output,'publication-evidence-index.json'),
    bundlePath:join(output,'publication-evidence-index.sigstore.json'),sourceCommit:fresh.source.commit,environment});
  saveProof(signerRoot,'evidence-index-signature',indexProof);
  const files=directoryFiles(output,{maximumFile:513*1024*1024,maximumTotal:600*1024*1024})
    .filter(file=>file.path!=='reproduction.json');
  await observeMain(api,fresh.source,{tagged:true});
  const publishStart=api.captures.length;
  try{
    const result=await publishAssets(api,fresh.source,files);
    writeNew(join(signerRoot,'publication-result.json'),bytes(result));return result;
  } finally {
    const records=api.captures.slice(publishStart).map(({data,...record},index)=>{
      const path=`publication-api-${String(index+1).padStart(3,'0')}.json`;
      writeNew(join(signerRoot,path),data);return {...record,file:describe(path,data)};
    });
    writeNew(join(signerRoot,'publication-api-inventory.json'),bytes(records));
  }
}

export async function publishCompiler(options={}) {
  const api=options.api??github(),root=`${options.output}-observations`;
  requireValue(!existsSync(root),'compiler new publisher observation root');mkdirSync(root,{mode:0o700});
  let primary;
  try{return await publishCompilerCore({...options,api});}
  catch(error){primary=error;writeNew(join(root,'failure.json'),bytes({status:'publisher-failed',message:error.message}));throw error;}
  finally{
    try{
      let total=0;
      requireValue(api.captures.length<=512,'compiler bounded publisher API capture count');
      const records=api.captures.map(({data,...record},index)=>{
        total+=data.length;requireValue(total<=256*1024*1024,'compiler bounded publisher raw captures');
        const path=`api-${String(index+1).padStart(3,'0')}.json`;writeNew(join(root,path),data);
        return {...record,file:describe(path,data)};
      });
      writeNew(join(root,'api-inventory.json'),bytes(records));
    }catch(error){if(primary)throw new AggregateError([primary,error],'compiler publisher primary and observation failure');throw error;}
  }
}
