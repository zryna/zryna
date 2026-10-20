import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {existsSync,mkdirSync,readFileSync,readdirSync,lstatSync,renameSync,writeFileSync,cpSync,rmSync} from 'node:fs';
import {join,resolve,dirname,relative} from 'node:path';
import {canonical,sha256} from './registry.mjs';
import {strictJson} from './json.mjs';

const LIMIT=16*1024*1024;
export function bundleInventory(root){
 const entries=[];let bytes=0;
 function visit(directory){for(const entry of readdirSync(directory,{withFileTypes:true}).sort((a,b)=>a.name.localeCompare(b.name))){const path=join(directory,entry.name),st=lstatSync(path);assert(!st.isSymbolicLink(),'ordinary bundle');if(entry.isDirectory())visit(path);else{assert(entry.isFile()&&st.nlink===1&&st.size<=LIMIT);const data=readFileSync(path);bytes+=data.length;assert(bytes<=64*1024*1024&&entries.length<128);entries.push({path:relative(root,path).split('\\').join('/'),bytes:data.length,sha256:sha256(data)});}}}
 visit(root);assert(entries.length>0);return entries.sort((a,b)=>a.path<b.path?-1:1);
}
export async function runCliParity({registry,root,cli,node,output}){
 mkdirSync(output,{mode:0o700});const results=[],namespace='npp-'+sha256(Buffer.from(output)).slice(0,10);
 for(const [index,item]of registry.cliBuildCases.entries()){
  const stem=namespace+'-'+String(index).padStart(3,'0'),bundle=join(root,'.zryna/out',stem+'.build'),caseRoot=join(output,String(index).padStart(3,'0'));mkdirSync(caseRoot,{mode:0o700});
  assert(!existsSync(bundle),'new task-owned output name');const captures={};let failure;
  for(const provider of ['native','bootstrap','compare']){
   try{
    const args=['build',item.entrypoint,'--root',root,'--target','all','--name',stem,'--provider',provider,'--json'];
    if(item.profile!=='i32-v1')args.push('--profile',item.profile);
    if(provider!=='native')args.push('--node',node);
    const startedAt=new Date().toISOString(),process=spawnSync(cli,args,{cwd:root,shell:false,windowsHide:true,timeout:120000,maxBuffer:LIMIT});
    const stdout=process.stdout??Buffer.alloc(0),stderr=process.stderr??Buffer.alloc(0);writeFileSync(join(caseRoot,provider+'.stdout'),stdout,{flag:'wx',mode:0o600});writeFileSync(join(caseRoot,provider+'.stderr'),stderr,{flag:'wx',mode:0o600});
    writeFileSync(join(caseRoot,provider+'.command.json'),canonical({program:cli,args,cwd:root,startedAt,finishedAt:new Date().toISOString(),exitCode:process.status,signal:process.signal,error:process.error?.code??null,stdout:{bytes:stdout.length,sha256:sha256(stdout)},stderr:{bytes:stderr.length,sha256:sha256(stderr)}}),{flag:'wx',mode:0o600});
    assert(!process.error&&process.signal===null,'actual bounded compiler completion');assert.equal(stderr.length,0,'JSON diagnostics are complete stdout');
    const document=strictJson(stdout,LIMIT);assert.equal(document.ok,item.disposition==='accepted','independent frozen disposition');assert.equal(process.status===0,item.disposition==='accepted');
    assert.deepEqual(document.diagnostics.map(d=>d.code),item.diagnosticCodes,'independent ordered diagnostic codes');if(item.diagnostics)assert.deepEqual(document.diagnostics,item.diagnostics,'independent complete diagnostics and byte spans');
    let inventory=[];
    if(item.disposition==='accepted'){
     assert(existsSync(bundle),'genuine successful bundle');assert.equal(dirname(resolve(root,document.manifest)),bundle,'manifest belongs to owned bundle');inventory=bundleInventory(bundle);
     const manifest=JSON.parse(readFileSync(resolve(root,document.manifest)));assert(manifest&&typeof manifest==='object','real complete manifest');
     const retained=join(caseRoot,provider+'-bundle');assert(!existsSync(retained));
     try{renameSync(bundle,retained);}catch(error){if(error.code!=='EXDEV')throw error;cpSync(bundle,retained,{recursive:true,errorOnExist:true,force:false,dereference:false});assert.deepEqual(bundleInventory(retained),inventory);assert.deepEqual(bundleInventory(bundle),inventory);rmSync(bundle,{recursive:true});}
    }else assert(!existsSync(bundle),'rejected source publishes no bundle');
    captures[provider]={exitCode:process.status,stdout:stdout.toString('utf8'),inventory};
   }catch(error){failure={provider,message:String(error.message)};break;}
  }
  if(!failure)try{assert.deepEqual(captures.native,captures.bootstrap,'complete native/bootstrap response, diagnostics, manifests and artifact byte hashes');assert.deepEqual(captures.native,captures.compare,'comparison route retains original observable result');}catch(error){failure={message:String(error.message)};}
  results.push({id:item.id,status:failure?'failed':'passed',disposition:item.disposition,...failure?{failure}:{}});
 }
 assert.equal(results.length,88,'complete frozen CLI census');const failed=results.filter(r=>r.status==='failed').length,result={status:failed?'failed':'passed',executed:results.length,failed,skipped:0,results,scope:'Current source-checkout real CLI BUILD comparisons only. This does not establish installed-default, runtime, fault, IR or full-CI acceptance.'};
 writeFileSync(join(output,'RESULT.json'),canonical(result),{flag:'wx',mode:0o600});return result;
}
