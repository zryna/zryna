import assert from 'node:assert/strict';
import {readdir,realpath,lstat,writeFile} from 'node:fs/promises';
import {join,dirname} from 'node:path';
import {performance} from 'node:perf_hooks';
import {descriptor,ordinaryRoot} from './scripts/m6/installed-editor/files.mjs';
assert.equal(process.platform,'win32');assert.equal(process.version,'v22.22.1');
const root=await realpath(process.env.M6_ZIP_DIAGNOSTIC_ROOT);await ordinaryRoot(root);
const deadline=performance.now()+15000;
const programFiles=await realpath(process.env.ProgramFiles);await ordinaryRoot(programFiles);
const git=join(programFiles,'Git');await ordinaryRoot(git);
const entries=await readdir(git,{withFileTypes:true});assert(entries.length<=128);
const directories=entries.filter(x=>x.isDirectory()).map(x=>x.name).sort();
const paths=new Set([join(git,'bin','curl.exe'),join(git,'cmd','curl.exe'),join(git,'usr','bin','curl.exe'),join(git,'mingw64','bin','curl.exe'),join(git,'ucrt64','bin','curl.exe')]);
for(const name of directories){paths.add(join(git,name,'bin','curl.exe'));paths.add(join(git,name,'curl.exe'));}
assert(paths.size<=261);const probes=[];
for(const selected of paths){assert(performance.now()<deadline);let earlier=null,pin=null,refusal=null;
try{await ordinaryRoot(dirname(selected));assert.equal(await realpath(selected),selected);const s=await lstat(selected,{bigint:true});earlier={nlink:String(s.nlink),size:String(s.size)};pin=await descriptor(selected,deadline);}catch(e){refusal=String(e.message).slice(0,2048);}
probes.push({selected,earlier,earlierIsNotDescriptorGuardProof:true,pin,refusal,targetExecuted:false});}
await writeFile(join(root,'GIT_INSTALLATION_METADATA.json'),JSON.stringify({source:process.env.GITHUB_SHA,programFiles,git,entries:entries.map(x=>({name:x.name,directory:x.isDirectory(),file:x.isFile(),symlink:x.isSymbolicLink()})),probes,diagnosticOnly:true,qualification:false,targetsExecuted:0},null,2)+'\n',{flag:'wx'});
