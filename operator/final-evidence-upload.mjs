// One reviewed actual payload -> one create-only private draft -> one immutable publication.
// No guest, test, signature issuance or opaque-capability authentication occurs here.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync,readdirSync,lstatSync,realpathSync} from 'node:fs';
import {resolve,dirname,join} from 'node:path';
import {fileURLToPath} from 'node:url';
import {gunzipSync} from 'node:zlib';
import {bytes,parseCanonical,sha256,exactKeys} from '../scripts/distribution/canonical.mjs';
import {decodeTar} from '../scripts/distribution/archive-tar.mjs';
import {requiredSourceGates} from '../scripts/m6/source-registry.mjs';
import {validateSourceM6Record} from '../scripts/m6/source-evidence.mjs';
import {validateRetainedReceipts} from '../scripts/m6/receipts.mjs';

const C='71fd69d99a39c22bdf262d1655a032aa76221f3e',T='351da60bd4037e0f96c286045c8943f41fa99b03';
const TAG='m6-source-evidence-'+C+'-1',REPO='/repos/zryna/zryna';
const HERE=dirname(fileURLToPath(import.meta.url)),WORK=resolve(HERE,'..'),INPUT=join(HERE,'final-evidence');
assert(process.env.RUNNER_TEMP&&process.env.RUNNER_TEMP.startsWith('/'));
const ROOT=join(process.env.RUNNER_TEMP,'m6-final-evidence');mkdirSync(ROOT,{mode:0o700});
let stage='closed-local-payload-validation',serial=0,releaseId=null;
const deadline=Date.now()+16*60*1000,uploaded=new Map();
const save=(name,value)=>writeFileSync(join(ROOT,name),Buffer.isBuffer(value)?value:bytes(value),{flag:'wx',mode:0o600});
function guard(){assert(Date.now()<deadline,'Whole fixed transport deadline exceeded');}
function ordinary(path,maximum){
 assert(realpathSync(path)===path);const s=lstatSync(path);assert(s.isFile()&&s.nlink===1&&!(s.mode&0o022)&&s.size>0&&s.size<=maximum);
 const data=readFileSync(path),after=lstatSync(path);for(const k of ['dev','ino','mode','nlink','size','mtimeMs','ctimeMs'])assert.equal(after[k],s[k]);assert.equal(data.length,s.size);return data;
}
function selected(pin,path,maximum){assert(Number.isSafeInteger(pin.bytes)&&pin.bytes>0&&pin.bytes<=maximum&&/^[a-f0-9]{64}$/.test(pin.sha256));const data=ordinary(path,maximum);assert.equal(data.length,pin.bytes);assert.equal(sha256(data),pin.sha256);return data;}
function timeout(){guard();return AbortSignal.timeout(Math.min(90000,Math.max(1,deadline-Date.now())));}
function allowed(path,method,upload){
 if(method==='GET')return !upload&&([REPO+'/branches/main',REPO+'/git/commits/'+C,REPO+'/git/ref/tags/'+TAG].includes(path)||new RegExp('^'+REPO+'/releases\\?per_page=100&page=(?:[1-9]|10)$').test(path)||releaseId!==null&&[REPO+'/releases/'+releaseId,REPO+'/releases/'+releaseId+'/assets?per_page=100'].includes(path));
 if(method==='POST'&&!upload)return stage==='create-private-draft'&&releaseId===null&&path===REPO+'/releases';
 if(method==='POST'&&upload)return stage==='upload-and-readback'&&releaseId!==null&&[...assetNames].some(n=>path===REPO+'/releases/'+releaseId+'/assets?name='+encodeURIComponent(n))&&!uploaded.has(decodeURIComponent(path.split('name=')[1]));
 return method==='PATCH'&&!upload&&stage==='publish-once'&&releaseId!==null&&path===REPO+'/releases/'+releaseId;
}
let assetNames;
async function api(path,{method='GET',body,status=200,upload=false}={}){
 guard();assert(allowed(path,method,upload),'Closed release endpoint/method only');assert(++serial<=160);
 const headers={authorization:'Bearer '+process.env.GITHUB_TOKEN,accept:'application/vnd.github+json','accept-encoding':'identity','x-github-api-version':'2022-11-28'};
 if(body!==undefined)headers['content-type']=upload?'application/octet-stream':'application/json';
 const response=await fetch('https://'+(upload?'uploads':'api')+'.github.com'+path,{method,headers,body:body===undefined?undefined:upload?body:JSON.stringify(body),redirect:'error',signal:timeout()});
 const chunks=[];let n=0;for await(const b of response.body){guard();n+=b.length;assert(n<=2*1024*1024);chunks.push(b);}const raw=Buffer.concat(chunks);save('API-'+String(serial).padStart(3,'0')+'.json',raw);
 assert.equal(response.status,status,'Actual API refusal: preserve partial state; never retry or overwrite');return JSON.parse(raw);
}
async function readback(asset,pin,phase){
 let next='https://api.github.com'+REPO+'/releases/assets/'+asset.id,data;
 for(let redirects=0;redirects<=2;redirects++){
  const u=new URL(next);assert(u.protocol==='https:'&&!u.username&&!u.password&&!u.hash&&(!u.port||u.port==='443'));
  assert(['api.github.com','release-assets.githubusercontent.com'].includes(u.hostname)||u.hostname.endsWith('.blob.core.windows.net'));
  if(u.hostname==='api.github.com')assert.equal(u.pathname,REPO+'/releases/assets/'+asset.id);
  const headers={accept:'application/octet-stream','accept-encoding':'identity','x-github-api-version':'2022-11-28'};if(u.hostname==='api.github.com')headers.authorization='Bearer '+process.env.GITHUB_TOKEN;
  const r=await fetch(next,{headers,redirect:'manual',signal:timeout()});
  if([301,302,303,307,308].includes(r.status)){assert(redirects<2);const location=r.headers.get('location');assert(location);await r.body?.cancel();next=new URL(location,next).href;continue;}
  assert.equal(r.status,200);let n=0;const chunks=[];for await(const b of r.body){guard();n+=b.length;assert(n<=pin.bytes);chunks.push(b);}data=Buffer.concat(chunks);break;
 }
 assert(data&&data.length===pin.bytes&&sha256(data)===pin.sha256);save(phase+'-'+pin.name,data);return {id:asset.id,...pin,completeActualDownload:true};
}
function matchAsset(a,pin){assert(Number.isSafeInteger(a.id)&&a.id>0&&a.name===pin.name&&a.state==='uploaded'&&a.size===pin.bytes&&a.digest==='sha256:'+pin.sha256);}
async function releaseState(draft,immutable){
 const release=await api(REPO+'/releases/'+releaseId),assets=await api(REPO+'/releases/'+releaseId+'/assets?per_page=100');
 assert(release.id===releaseId&&release.tag_name===TAG&&release.target_commitish===C&&release.draft===draft&&release.prerelease===true);
 if(immutable)assert.equal(release.immutable,true);else assert.notEqual(release.immutable,true);
 assert(Array.isArray(assets)&&assets.length===assetNames.length&&new Set(assets.map(a=>a.name)).size===assets.length);
 assert.deepEqual(assets.map(a=>a.name).sort(),assetNames);for(const a of assets){const known=uploaded.get(a.name);assert(known&&known.id===a.id);matchAsset(a,known.pin);}
 assert.deepEqual(release.assets.map(a=>a.name).sort(),assetNames);for(const a of release.assets){const known=uploaded.get(a.name);assert(known&&known.id===a.id);matchAsset(a,known.pin);}return {release,assets};
}
async function currentMain(){const main=await api(REPO+'/branches/main'),source=await api(REPO+'/git/commits/'+C);assert(main.protected===true&&main.commit.sha===C&&source.sha===C&&source.tree.sha===T);}
try{
 assert.equal(process.version,'v22.22.1');assert.equal(process.env.GITHUB_EVENT_NAME,'workflow_dispatch');assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');assert.equal(process.env.GITHUB_RUN_ATTEMPT,'1');
 assert(/^[a-f0-9]{40}$/.test(process.env.APPROVED_OPERATOR_COMMIT));assert.equal(process.env.GITHUB_SHA,process.env.APPROVED_OPERATOR_COMMIT);assert.notEqual(process.env.GITHUB_SHA,C);assert(process.env.GITHUB_REF.startsWith('refs/heads/'));
 const selectionRaw=ordinary(join(INPUT,'selection.json'),262144);assert.equal(sha256(selectionRaw),process.env.APPROVED_MANIFEST_SHA256);const selection=parseCanonical(selectionRaw);
 exactKeys(selection,['format','version','repository','source','tag','assets','review','contractPins']);assert.equal(selection.format,'zryna.m6-final-evidence-upload-selection.v1');assert.equal(selection.version,1);assert.equal(selection.repository,'zryna/zryna');assert.deepEqual(selection.source,{commit:C,tree:T});assert.equal(selection.tag,TAG);
 const contractPaths=['scripts/m6/source-evidence.mjs','scripts/m6/source-registry.mjs','scripts/m6/registry.mjs','scripts/m6/receipts.mjs','scripts/distribution/canonical.mjs','scripts/distribution/archive-tar.mjs','scripts/distribution/archive-entries.mjs','examples/playground/restricted/json.mjs','examples/playground/restricted/limits.mjs','examples/playground/restricted/daemon/compiler-publication-capability.mjs'];
 assert.deepEqual(selection.contractPins.map(p=>p.path),contractPaths);for(const p of selection.contractPins){exactKeys(p,['path','bytes','sha256']);selected(p,join(WORK,p.path),262144);}
 assetNames=['m6-source-evidence.json','m6-source-evidence-policy.json','m6-source-evidence-carriers.tar.gz','m6-source-evidence-capability-relationship.json',...requiredSourceGates.map(id=>id+'.json')].sort();assert.equal(assetNames.length,27);
 assert(Array.isArray(selection.assets));assert.deepEqual(selection.assets.map(p=>p.name),assetNames);assert.deepEqual(readdirSync(join(INPUT,'assets')).sort(),assetNames);
 const payload=new Map();let total=0;
 for(const p of selection.assets){exactKeys(p,['name','bytes','sha256']);const data=selected(p,join(INPUT,'assets',p.name),p.name==='m6-source-evidence-carriers.tar.gz'?24*1024*1024:262144);total+=data.length;assert(total<=32*1024*1024);payload.set(p.name,data);}
 exactKeys(selection.review,['bytes','sha256']);assert.equal(selection.review.sha256,process.env.APPROVED_REVIEW_SHA256);const review=parseCanonical(selected(selection.review,join(INPUT,'independent-review.json'),262144));
 assert.equal(review.format,'zryna.m6-final-evidence-upload-review.v1');assert.equal(review.version,1);assert.equal(review.status,'independently-reviewed-actual-final-payload');assert.deepEqual(review.source,selection.source);assert.deepEqual(review.assets,selection.assets);assert.equal(review.originalRelationshipVerifiedByRoot,true);
 const recordRaw=payload.get('m6-source-evidence.json'),policy=parseCanonical(payload.get('m6-source-evidence-policy.json'));const record=validateSourceM6Record(recordRaw,policy);assert.equal(record.integration.commit,C);assert.equal(record.integration.tree,T);
 assert.equal(review.recordSha256,sha256(recordRaw));assert.equal(review.policySha256,sha256(payload.get('m6-source-evidence-policy.json')));
 const archive=payload.get('m6-source-evidence-carriers.tar.gz');gunzipSync(archive,{maxOutputLength:20*1024*1024});const files=await decodeTar(archive,'m6-source-evidence-carriers',0);assert(files.every(f=>f.mode===0o644));validateRetainedReceipts(record,files.map(({path,data})=>({path,data})));
 assert.equal(review.carriersSha256,sha256(archive));
 for(const gate of record.gates){assert.equal(gate.receipt.url,'https://github.com/zryna/zryna/releases/download/'+TAG+'/'+gate.id+'.json');const actual=files.find(f=>f.path===gate.receipt.path);assert(actual&&actual.data.equals(payload.get(gate.id+'.json')));}
 const relationshipRaw=payload.get('m6-source-evidence-capability-relationship.json'),relationship=JSON.parse(relationshipRaw);
 assert.equal(review.capabilityRelationshipProofSha256,sha256(relationshipRaw));assert.equal(relationship.status,'actual-original-opaque-capability-validates-final-record');assert.equal(relationship.recordSha256,sha256(recordRaw));
 assert.equal(relationship.compilerArchiveSha256,record.publication.compilerArchiveSha256);assert.equal(relationship.compilerBinarySha256,record.publication.compilerBinarySha256);assert.equal(relationship.compositionSha256,record.publication.compositionSha256);
 save('VALIDATED_PAYLOAD.json',{status:'original-record-and-complete-carriers-validated-before-first-post',source:selection.source,manifestSha256:sha256(selectionRaw),reviewSha256:selection.review.sha256,assets:selection.assets,totalBytes:total,carrierFiles:files.length,rootVerifiedRelationshipProofBound:true,transportAuthenticatesOpaqueCapability:false});
 await currentMain();for(let page=1;page<=10;page++){const releases=await api(REPO+'/releases?per_page=100&page='+page);assert(Array.isArray(releases)&&releases.length<=100&&!releases.some(r=>r.tag_name===TAG));if(releases.length<100)break;assert(page<10);}
 await api(REPO+'/git/ref/tags/'+TAG,{status:404});
 stage='create-private-draft';save('CREATE_INTENT.json',{tag:TAG,targetCommit:C,approvedOperator:process.env.GITHUB_SHA});
 const draft=await api(REPO+'/releases',{method:'POST',status:201,body:{tag_name:TAG,target_commitish:C,name:'M6 exact-source evidence '+C,body:'Exact-source reviewed M6 evidence and original receipt carriers. Independent compiler publication and actual gate proofs are retained in the selected record.',draft:true,prerelease:true,generate_release_notes:false}});
 assert(Number.isSafeInteger(draft.id)&&draft.id>0&&draft.draft===true&&draft.tag_name===TAG&&draft.target_commitish===C&&draft.prerelease===true&&draft.assets.length===0&&draft.immutable!==true);releaseId=draft.id;
 stage='upload-and-readback';
 for(const p of selection.assets){save('UPLOAD_INTENT-'+p.name+'.json',{releaseId,...p});const asset=await api(REPO+'/releases/'+releaseId+'/assets?name='+encodeURIComponent(p.name),{method:'POST',status:201,upload:true,body:payload.get(p.name)});matchAsset(asset,p);assert(![...uploaded.values()].some(a=>a.id===asset.id));uploaded.set(p.name,{id:asset.id,pin:p});await readback(asset,p,'DRAFT');}
 await releaseState(true,false);await currentMain();
 stage='publish-once';save('PUBLISH_INTENT.json',{releaseId,tag:TAG,sourceCommit:C,assetCount:27,allAssetsActuallyDownloadedAndMatched:true});
 const published=await api(REPO+'/releases/'+releaseId,{method:'PATCH',body:{draft:false,prerelease:true,make_latest:'false'}});assert(published.id===releaseId&&published.draft===false&&published.prerelease===true);
 stage='immutable-published-readback';const actual=await releaseState(false,true);for(const a of actual.assets)await readback(a,uploaded.get(a.name).pin,'PUBLISHED');await releaseState(false,true);
 const ref=await api(REPO+'/git/ref/tags/'+TAG);assert.equal(ref.object.type,'commit');assert.equal(ref.object.sha,C);
 const result={status:'actual-final-source-evidence-published-immutable-and-fully-read-back',source:{commit:C,tree:T},releaseId,tag:TAG,immutable:true,assets:selection.assets,recordSha256:sha256(recordRaw),policySha256:review.policySha256,carriersSha256:review.carriersSha256,capabilityRelationshipProofSha256:review.capabilityRelationshipProofSha256,manifestSha256:sha256(selectionRaw),reviewSha256:selection.review.sha256,operator:process.env.GITHUB_SHA,runId:Number(process.env.GITHUB_RUN_ID),attempt:1,originalCapabilityAuthenticatedByTransport:false};save('RESULT.json',result);console.log(JSON.stringify(result));
}catch(error){
 const message=String(error.message??error).replace(/Bearer\s+\S+/gi,'Bearer REDACTED').replace(/https?:\/\/[^\s)]+/g,'URL_REDACTED').slice(0,1500);save('FAILURE.json',{status:'failed-no-transport-success-claim',stage,releaseId,name:error.name,message,retry:'none; preserve actual partial draft/uploads/publication and independently read back before any future decision'});throw Error('Final evidence transport failed at '+stage+'; inspect retained sanitized failure.');
}
