// Read-only authentication diagnostic. The historical draft is not qualification evidence.
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync} from 'node:fs';
import {join} from 'node:path';
const source={commit:'62635fabedcccf8c505f1a0360471425c22ea560',tree:'56e5e37fdc68cc49ac5db7c86a2f0933ded047fb'};
const draftId=416355512,tag='m6-raw-evidence-5d79dba18117a7ff41ff08d328aa94838c0fce9c-1';
assert.equal(process.env.GITHUB_REPOSITORY,'zryna/zryna');assert.equal(process.env.GITHUB_EVENT_NAME,'workflow_dispatch');
assert(/^[a-f0-9]{40}$/.test(process.env.APPROVED_OPERATOR_COMMIT??''));assert.equal(process.env.GITHUB_SHA,process.env.APPROVED_OPERATOR_COMMIT);
assert.equal(process.version,'v22.22.1');assert(process.env.GITHUB_TOKEN);
const output=join(process.env.RUNNER_TEMP,'m6-private-draft-read');mkdirSync(output,{mode:0o700});
const requests=[];
function subject(value){
 assert(value?.id===draftId&&typeof value.draft==='boolean'&&value.tag_name===tag&&Array.isArray(value.assets));
 return {id:value.id,draft:value.draft,tag:value.tag_name,assetCount:value.assets.length};
}
async function get(path,label){
 assert(['/repos/zryna/zryna/releases/416355512','/repos/zryna/zryna/releases?per_page=100&page=1','/repos/zryna/zryna/releases?per_page=100&page=2'].includes(path));
 try{
  const response=await fetch('https://api.github.com'+path,{method:'GET',headers:{authorization:'Bearer '+process.env.GITHUB_TOKEN,accept:'application/vnd.github+json','accept-encoding':'identity','x-github-api-version':'2022-11-28'},redirect:'error',signal:AbortSignal.timeout(30000)});
  const record={label,httpStatus:response.status};requests.push(record);
  if(response.status!==200){await response.body?.cancel();return null}
  const chunks=[];let bytes=0;for await(const chunk of response.body){bytes+=chunk.length;assert(bytes<=2*1024*1024);chunks.push(chunk)}
  const value=JSON.parse(Buffer.concat(chunks).toString('utf8'));
  if(label==='direct'){record.subject=subject(value)}
  else{assert(Array.isArray(value)&&value.length<=100);record.returnedRows=value.length;const found=value.filter(row=>row.id===draftId);assert(found.length<=1);record.subject=found.length?subject(found[0]):null}
  return record;
 }catch(error){
  const code=String(error.cause?.code??error.code??'');const safe=/^(?:ECONNRESET|ENOTFOUND|ETIMEDOUT|ABORT_ERR|UND_ERR_[A-Z_]+)$/.test(code)?code:'DIAGNOSTIC_REQUEST_ERROR';
  if(requests.at(-1)?.label===label)requests.at(-1).errorCode=safe;else requests.push({label,httpStatus:null,errorCode:safe});return null;
 }
}
const direct=await get('/repos/zryna/zryna/releases/416355512','direct');
await get('/repos/zryna/zryna/releases?per_page=100&page=1','list-page-1');
await get('/repos/zryna/zryna/releases?per_page=100&page=2','list-page-2');
const foundByList=requests.some(row=>row.label!=='direct'&&row.subject?.id===draftId);
const visible=direct?.subject?.draft===true&&foundByList&&requests.every(row=>row.httpStatus===200&&!row.errorCode);
const result={format:'zryna.m6-private-draft-read-diagnostic.v1',status:visible?'contents-read-private-draft-visible':'contents-read-private-draft-not-demonstrated',source,operator:process.env.GITHUB_SHA,runId:Number(process.env.GITHUB_RUN_ID),attempt:Number(process.env.GITHUB_RUN_ATTEMPT),permissions:{contents:'read',actions:'none',idToken:'none'},fixedDraftId:draftId,requests,visibleByDirectGet:direct?.subject?.draft===true,visibleInFirstTwoListPages:foundByList,mutations:0,qualificationClaim:false,scope:'Official native Actions GITHUB_TOKEN read-only draft visibility diagnostic only. Historical UNQUALIFIED draft is not adopted as current runtime/qualification evidence.'};
writeFileSync(join(output,'RESULT.json'),JSON.stringify(result,null,2)+'\n',{flag:'wx',mode:0o600});
console.log(JSON.stringify({status:result.status,httpStatuses:requests.map(row=>row.httpStatus),visibleByDirectGet:result.visibleByDirectGet,visibleInFirstTwoListPages:foundByList,mutations:0}));
