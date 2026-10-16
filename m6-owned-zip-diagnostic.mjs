import assert from 'node:assert/strict';
import {mkdir,mkdtemp,realpath,lstat,writeFile,access,readdir} from 'node:fs/promises';
import {constants} from 'node:fs';
import {tmpdir} from 'node:os';
import {delimiter,join,resolve} from 'node:path';
import {performance} from 'node:perf_hooks';
import {acquire} from './scripts/m6/installed-editor-acquire.mjs';
import {capture,descriptor,ordinaryRoot} from './scripts/m6/installed-editor/files.mjs';
import {verifyEditor,verifyEditorArchive} from './scripts/m6/installed-editor/materials.mjs';
import {EDITOR_ARCHIVES} from './scripts/m6/installed-editor/pins.mjs';
import {command} from './scripts/m6/installed-editor/process.mjs';
assert.equal(process.platform,'win32');assert.equal(process.version,'v22.22.1');
const root=await realpath(process.env.M6_ZIP_DIAGNOSTIC_ROOT);
await ordinaryRoot(root);
let accepted=false,error=null,stage=null;
try {
  const metadata=[];const probeDeadline=performance.now()+15000;
  for(const program of ['git','tar','curl.exe']) {
    let selected;
    for(const directory of (process.env.PATH??process.env.Path??'').split(delimiter).filter(Boolean)) {
      for(const candidate of [join(directory,program),join(directory,program+'.exe')]) {
        assert(performance.now()<probeDeadline);
        try{await access(candidate,constants.X_OK);selected=await realpath(candidate);break;}catch{}
      }
      if(selected)break;
    }
    assert(selected);const earlier=await lstat(selected,{bigint:true});
    let pin=null,refusal=null;try{pin=await descriptor(selected,probeDeadline);}catch(e){refusal=String(e.message).slice(0,2048);}
    metadata.push({program,selected,earlierNlink:String(earlier.nlink),earlierSize:String(earlier.size),earlierTupleIsNotDescriptorGuardProof:true,pin,refusal,targetExecutedByProbe:false});
  }
  await writeFile(join(root,'TOOL_METADATA.json'),JSON.stringify(metadata,null,2)+'\n',{flag:'wx'});
  const archivePath=await acquire(join(root,'material'));
  const deadline=performance.now()+600000;
  stage=await mkdtemp(join(await realpath(tmpdir()),'m6-owned-zip-diagnostic-'));
  const commands=join(root,'commands');await mkdir(commands);
  const protectedEditor=await capture(archivePath,join(stage,'authenticated-editor.archive'),EDITOR_ARCHIVES.win32,deadline);
  const archive=await verifyEditorArchive(protectedEditor.captured.path,deadline);archive.originalCapture=protectedEditor.source;
  const editorRoot=join(stage,'editor');await mkdir(editorRoot);await ordinaryRoot(editorRoot);
  const helperSource=resolve('scripts/m6/installed-editor/extract-editor-windows.ps1');
  const helperPin=await descriptor(helperSource,deadline);
  const helper=await capture(helperSource,join(stage,'extract-editor-windows.ps1'),helperPin,deadline);
  const runtime=join(process.env.ProgramFiles,'PowerShell','7','pwsh.exe');
  const extraction={method:'owned-powershell-system-io-compression-zipfile',helper,runtime:await descriptor(await realpath(runtime),deadline)};
  await writeFile(join(root,'EXTRACTION_BEFORE.json'),JSON.stringify({archive,extraction,stage,source:process.env.GITHUB_SHA},null,2)+'\n',{flag:'wx'});
  await command(runtime,['-NoLogo','-NoProfile','-NonInteractive','-File',helper.captured.path,protectedEditor.captured.path,editorRoot],{cwd:stage,stage,env:{...process.env},output:commands,deadline,name:'extract-authenticated-editor'});
  const layout=[];
  for(const relative of ['', 'resources', 'resources/app', 'VSCode-win32-x64']){
    const path=relative?join(editorRoot,...relative.split('/')):editorRoot;
    let entries=null,error=null;
    try{const all=await readdir(path,{withFileTypes:true});assert(all.length<=256);entries=all.map(x=>({name:x.name,directory:x.isDirectory(),file:x.isFile(),symlink:x.isSymbolicLink()}));}
    catch(e){error=String(e.message).slice(0,2048);}
    layout.push({path,entries,error});
  }
  await writeFile(join(root,'EXTRACTED_ROOT_LAYOUT.json'),JSON.stringify({stage,editorRoot,layout,diagnosticOnly:true,qualification:false},null,2)+'\n',{flag:'wx'});
  const editor=await verifyEditor(editorRoot,deadline);
  await writeFile(join(root,'EXTRACTED_EDITOR.json'),JSON.stringify(editor,null,2)+'\n',{flag:'wx'});
  accepted=true;
} catch(e){error=String(e.stack??e).slice(0,16384);}
await writeFile(join(root,'DIAGNOSTIC_RESULT.json'),JSON.stringify({diagnosticOnly:true,qualification:false,acceptedZipExtractionOnly:accepted,installedHostAccepted:false,vsixChecksExecuted:0,editorGuiExecuted:false,source:process.env.GITHUB_SHA,stage,error},null,2)+'\n',{flag:'wx'});
if(!accepted)process.exitCode=1;
