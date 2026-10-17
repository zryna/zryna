import { bytes, describe, ordinary, requireValue, writeNew } from './common.mjs';
import { directoryFiles } from './files.mjs';
import { join } from 'node:path';
import { encodePublicationEvidence } from './evidence-layout.mjs';
export async function retainPublicationEvidence({admissionRoot,firstRoot,secondRoot,assembledRoot,output,source}) {
  const read=root=>directoryFiles(root,{maximumFiles:400,maximumFile:64*1024*1024,
    maximumTotal:96*1024*1024,exclude:['compiler.bin']});
  const admission=read(admissionRoot).map(file=>({...file,path:`admission/${file.path}`}));
  const archive=await encodePublicationEvidence(admission,[read(firstRoot),read(secondRoot)],
    {path:'assembly/reproduction.json',mode:0o644,data:ordinary(join(assembledRoot,'reproduction.json'))},source);
  writeNew(join(output,'publication-evidence.tar.gz'),archive);
  return describe('publication-evidence.tar.gz',archive);
}
export function evidenceIndex(source,compiler,files) {
  const ordered=[...files].sort((a,b)=>a.path<b.path?-1:1);
  requireValue(ordered.length===5 && new Set(ordered.map(file=>file.path)).size===5,
    'compiler evidence index closed subjects');
  return bytes({format:'zryna.playground-compiler-publication-evidence.v2',version:2,source,compiler,
    scope:'actual-protected-admission-build-and-review; final-public-acceptance-remains-separate',files:ordered});
}
