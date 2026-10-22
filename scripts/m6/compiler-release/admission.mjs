import { existsSync, mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { bytes, describe, id, ordinary, receiptSource, requireValue, sha256, writeNew, TAG }
  from './common.mjs';
import { github } from './github.mjs';
import { captureSource } from './source.mjs';
import { expectedJobs, observeCi, observeMain } from './gates.mjs';
import { openQualification } from './qualification.mjs';
async function qualificationTransport(api, selection) {
  const asset=await api.request(`/repos/zryna/zryna/releases/assets/${id(selection.qualificationAsset)}`);
  requireValue(asset.id===selection.qualificationAsset && asset.state==='uploaded' &&
    asset.name==='m6-compiler-qualification.tar.gz' && Number.isSafeInteger(asset.size) &&
    asset.size>0 && asset.size<=64*1024*1024 &&
    (!asset.digest || asset.digest===`sha256:${selection.qualificationSha256}`), 'compiler qualification transport asset');
  let selected;
  for(let page=1;page<=10;page++) {
    const releases=await api.request(`/repos/zryna/zryna/releases?per_page=100&page=${page}`);
    requireValue(Array.isArray(releases) && releases.length<=100,'compiler qualification bounded release listing');
    for(const release of releases) if(release.assets?.some(a=>a.id===asset.id)) {
      requireValue(!selected && release.draft===true && release.tag_name!==TAG &&
        /^m6-qualification-[A-Za-z0-9._-]{1,80}$/.test(release.tag_name), 'compiler private draft qualification transport');
      selected={releaseId:id(release.id),assetId:asset.id,tag:release.tag_name,draft:true};
    }
    if(releases.length<100) break;
    requireValue(page<10,'compiler qualification complete release listing');
  }
  requireValue(selected,'compiler qualification selected draft release');
  const packet=await api.asset(asset.id);
  requireValue(packet.length===asset.size && sha256(packet)===selection.qualificationSha256,
    'compiler qualification actual transport bytes');
  return {packet,transport:selected};
}
async function prepareAdmissionCore({ sourceRoot, output, environment=process.env, api=github(), spawn, deferQualification=false }={}) {
  requireValue(typeof deferQualification==='boolean','compiler admission qualification phase');
  const captured=captureSource({cwd:sourceRoot,environment,spawn});
  const {source,publishing}=captured;
  if(publishing) {
    const event=JSON.parse(ordinary(environment.GITHUB_EVENT_PATH));
    requireValue(event.ref===`refs/tags/${TAG}` && event.created===true && event.deleted===false &&
      event.forced===false && event.repository?.full_name==='zryna/zryna', 'compiler release tag creation event');
  }
  await observeMain(api,source,{tagged:publishing});
  const inventory=await expectedJobs(sourceRoot,api,source);
  const selection=publishing?captured.selection:{ciRun:id(environment.M6_CI_RUN),ciAttempt:id(environment.M6_CI_ATTEMPT)};
  const ci=await observeCi(api,source,selection.ciRun,selection.ciAttempt,inventory);
  let qualification;
  if(publishing && !deferQualification) {
    const transport=await qualificationTransport(api,selection);
    const opened=await openQualification(transport.packet,{source,selection});
    qualification={...transport,record:opened.record,review:opened.review};
  }
  const admission={format:'zryna.playground-compiler-admission.v1',version:1,
    status:publishing?(deferQualification?'protected-tag-source-admitted':'protected-tag-admitted'):'candidate-build-only',source,publishing,selection,
    workflow:captured.workflow,ci,workflowInventory:inventory,
    run:{id:id(environment.GITHUB_RUN_ID),attempt:id(environment.GITHUB_RUN_ATTEMPT)},
    qualification:qualification?{transport:qualification.transport,compiler:qualification.record.compiler,
      packet:describe('qualification.tar.gz',qualification.packet),reviewSha256:selection.reviewSha256}:null};
  if(captured.rawTag) writeNew(join(output,'tag.txt'),captured.rawTag);
  if(qualification) writeNew(join(output,'qualification.tar.gz'),qualification.packet);
  mkdirSync(join(output,'api'),{mode:0o700});
  const requests=api.captures.map((capture,index)=>{
    const {data,...record}=capture;
    const packetCopy=qualification && data.equals(qualification.packet);
    const path=packetCopy?'qualification.tar.gz':`api/${String(index+1).padStart(3,'0')}.json`;
    if(!packetCopy)writeNew(join(output,path),data);return {...record,file:describe(path,data)};
  });
  admission.observations=requests;
  writeNew(join(output,'admission.json'),bytes(admission));return admission;
}

export async function prepareAdmission(options={}) {
  const api=options.api??github();
  requireValue(!existsSync(options.output),'compiler new admission output');
  mkdirSync(options.output,{recursive:false,mode:0o700});
  try{return await prepareAdmissionCore({...options,api});}
  catch(error){
    mkdirSync(join(options.output,'failed-api'),{mode:0o700});
    const captures=api.captures.map(({data,...record},index)=>{
      const path=`failed-api/${String(index+1).padStart(3,'0')}.json`;
      writeNew(join(options.output,path),data);return {...record,file:describe(path,data)};
    });
    writeNew(join(options.output,'failure.json'),bytes({status:'admission-failed',message:error.message,captures}));
    if(error.proof){writeNew(join(options.output,'failure.stdout'),error.proof.stdout);
      writeNew(join(options.output,'failure.stderr'),error.proof.stderr);}
    throw error;
  }
}
