// Maintainer-selected exact packet -> one private draft. No final release/tag/publication authority.
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,mkdirSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {openQualification} from '../scripts/m6/compiler-release/qualification.mjs';
const root=process.env.RUNNER_TEMP+'/m6-qualification-upload';mkdirSync(root,{mode:0o700});
let stage='selected-packet-validation';
try{
const pins=JSON.parse(readFileSync(new URL('./qualification-pins.json',import.meta.url))),packet=readFileSync(new URL('./m6-compiler-qualification.tar.gz',import.meta.url));
const sha=b=>createHash('sha256').update(b).digest('hex');
assert.equal(pins.source.repository,'https://github.com/zryna/zryna');assert(/^[a-f0-9]{40}$/.test(pins.source.commit)&&/^[a-f0-9]{40}$/.test(pins.source.tree));assert.notEqual(pins.source.commit,'5d79dba18117a7ff41ff08d328aa94838c0fce9c','Known failed ordinary Chrome source is forbidden');assert.equal(pins.source.commit,'b3be654c452ea51f2c136a75d2eb89995eb4bd66');assert.equal(pins.source.tree,'3fa38d7b62b06a64ac5385d610be164f9b1961b8');
assert.equal(process.env.GITHUB_SHA,process.env.APPROVED_OPERATOR_COMMIT);assert(/^[a-f0-9]{40}$/.test(process.env.APPROVED_OPERATOR_COMMIT));assert.equal(pins.qualificationSha256,process.env.APPROVED_PACKET_SHA256);assert.equal(pins.reviewSha256,process.env.APPROVED_REVIEW_SHA256);
assert.equal(packet.length,pins.bytes);assert.equal(sha(packet),pins.qualificationSha256);assert.equal(pins.tag,'m6-qualification-'+pins.source.commit+'-20261018-1');
assert.equal(process.env.GITHUB_EVENT_NAME,'workflow_dispatch');assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');
const opened=await openQualification(packet,{source:pins.source,compiler:pins.compiler,selection:{qualificationSha256:pins.qualificationSha256,reviewSha256:pins.reviewSha256}});assert.equal(opened.record.obligations.length,68);
let serial=0;
async function api(path,{method='GET',body,status=200,upload=false}={}){
 assert(/^\/repos\/zryna\/zryna\//.test(path));const headers={authorization:'Bearer '+process.env.GITHUB_TOKEN,accept:'application/vnd.github+json','accept-encoding':'identity','x-github-api-version':'2022-11-28'};
 if(body!==undefined)headers['content-type']=upload?'application/octet-stream':'application/json';
 const response=await fetch('https://'+(upload?'uploads':'api')+'.github.com'+path,{method,headers,body:body===undefined?undefined:upload?body:JSON.stringify(body),redirect:'error',signal:AbortSignal.timeout(120000)});
 let size=0;const blocks=[];for await(const b of response.body){size+=b.length;assert(size<=2097152);blocks.push(b)}const raw=Buffer.concat(blocks);
 writeFileSync(root+'/'+String(++serial).padStart(3,'0')+'.json',raw,{flag:'wx',mode:0o600});assert.equal(response.status,status,'Actual API refusal; preserve existing draft/asset and never retry without readback');return JSON.parse(raw);
}
const main=await api('/repos/zryna/zryna/branches/main');assert(main.protected&&main.commit.sha===pins.source.commit);const source=await api('/repos/zryna/zryna/git/commits/'+pins.source.commit);assert.equal(source.tree.sha,pins.source.tree);
for(let page=1;page<=10;page++){const rows=await api('/repos/zryna/zryna/releases?per_page=100&page='+page);assert(Array.isArray(rows)&&rows.length<=100&&!rows.some(r=>r.tag_name===pins.tag));if(rows.length<100)break;assert(page<10)}
stage='create-one-private-draft';
const release=await api('/repos/zryna/zryna/releases',{method:'POST',status:201,body:{tag_name:pins.tag,target_commitish:pins.source.commit,name:'Private M6 compiler qualification transport',body:'Actual reviewed68-case qualification packet only. Mutable draft transport; no public acceptance or final compiler release.',draft:true,prerelease:true,generate_release_notes:false}});assert(release.draft&&release.tag_name===pins.tag&&release.assets.length===0);
stage='one-fixed-asset-upload';
const asset=await api('/repos/zryna/zryna/releases/'+release.id+'/assets?name=m6-compiler-qualification.tar.gz',{method:'POST',status:201,upload:true,body:packet});assert.equal(asset.name,'m6-compiler-qualification.tar.gz');assert.equal(asset.state,'uploaded');assert.equal(asset.size,pins.bytes);assert.equal(asset.digest,'sha256:'+pins.qualificationSha256);
const actual=await api('/repos/zryna/zryna/releases/'+release.id);assert(actual.draft&&actual.id===release.id&&actual.assets.length===1&&actual.assets[0].id===asset.id&&actual.assets[0].digest===asset.digest);
// Actual complete byte readback is independent of upload response metadata.
stage='complete-independent-asset-readback';
let next='https://api.github.com/repos/zryna/zryna/releases/assets/'+asset.id,readback;
for(let redirects=0;redirects<=2;redirects++){
 const u=new URL(next);assert(u.protocol==='https:'&&!u.username&&!u.password&&!u.hash);
 assert(u.hostname==='api.github.com'||u.hostname==='release-assets.githubusercontent.com'||u.hostname.endsWith('.blob.core.windows.net'));
 const headers={accept:'application/octet-stream','accept-encoding':'identity','x-github-api-version':'2022-11-28'};if(u.hostname==='api.github.com')headers.authorization='Bearer '+process.env.GITHUB_TOKEN;
 const response=await fetch(next,{headers,redirect:'manual',signal:AbortSignal.timeout(120000)});
 if([301,302,303,307,308].includes(response.status)){assert(redirects<2);const target=response.headers.get('location');assert(target);await response.body?.cancel();next=new URL(target,next).href;continue}
 assert.equal(response.status,200);let count=0;const chunks=[];
 for await(const chunk of response.body){count+=chunk.length;assert(count<=pins.bytes);chunks.push(chunk)}readback=Buffer.concat(chunks);break;
}
assert(readback?.equals(packet));assert.equal(sha(readback),pins.qualificationSha256);
writeFileSync(root+'/READBACK.json',JSON.stringify({assetId:asset.id,bytes:readback.length,sha256:sha(readback),completeActualDownload:true},null,2)+'\n',{flag:'wx'});
stage='final-private-draft-state';
const final=await api('/repos/zryna/zryna/releases/'+release.id);
assert(final.id===release.id&&final.draft===true&&final.tag_name===pins.tag&&final.target_commitish===pins.source.commit&&final.assets.length===1);
const [finalAsset]=final.assets;assert(finalAsset.id===asset.id&&finalAsset.name==='m6-compiler-qualification.tar.gz'&&finalAsset.state==='uploaded'&&finalAsset.size===pins.bytes&&finalAsset.digest==='sha256:'+pins.qualificationSha256);
const result={status:'actual-reviewed-qualification-uploaded-private-draft-only',source:pins.source,releaseId:release.id,assetId:asset.id,tag:pins.tag,bytes:asset.size,qualificationSha256:pins.qualificationSha256,reviewSha256:pins.reviewSha256,runId:Number(process.env.GITHUB_RUN_ID),attempt:Number(process.env.GITHUB_RUN_ATTEMPT),operator:process.env.GITHUB_SHA,publicSupportClaim:false};writeFileSync(root+'/RESULT.json',JSON.stringify(result,null,2)+'\n',{flag:'wx'});console.log(JSON.stringify(result));

}catch(error){
 const safe=String(error.message??error).replace(/Bearer\s+\S+/gi,'Bearer REDACTED').replace(/https?:\/\/[^\s)]+/g,'URL_REDACTED').slice(0,1500);
 writeFileSync(root+'/FAILURE.json',JSON.stringify({status:'failed-no-acceptance',stage,name:error.name,message:safe},null,2)+'\n',{flag:'wx',mode:0o600});throw Error('Qualification transport failed at '+stage+'; retained sanitized FAILURE.json');
}
