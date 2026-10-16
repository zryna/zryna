import { encodeTar } from '../../distribution/archive-tar.mjs';
import { bytes, describe, ordinary, requireValue, writeNew } from './common.mjs';
import { directoryFiles } from './files.mjs';
import { join } from 'node:path';
export async function retainPublicationEvidence({admissionRoot,firstRoot,secondRoot,assembledRoot,output,source}) {
  const files=[];
  for(const [prefix,root] of [['admission',admissionRoot],['replica1',firstRoot],['replica2',secondRoot]]) {
    files.push(...directoryFiles(root,{maximumFiles:400,maximumFile:64*1024*1024,maximumTotal:96*1024*1024,exclude:['compiler.bin']}).map(file=>({...file,path:`${prefix}/${file.path}`})));
  }
  files.push({path:'assembly/reproduction.json',mode:0o644,
    data:ordinary(join(assembledRoot,'reproduction.json'))});
  files.sort((a,b)=>a.path<b.path?-1:1);
  requireValue(files.length<=500 && files.reduce((n,f)=>n+f.data.length,0)<=96*1024*1024,
    'compiler finite durable admission/build evidence');
  const archive=await encodeTar('m6-compiler-publication-evidence',files,source.sourceDateEpoch);
  writeNew(join(output,'publication-evidence.tar.gz'),archive);
  return describe('publication-evidence.tar.gz',archive);
}
export function evidenceIndex(source,compiler,files) {
  const ordered=[...files].sort((a,b)=>a.path<b.path?-1:1);
  requireValue(ordered.length===5 && new Set(ordered.map(file=>file.path)).size===5,
    'compiler evidence index closed subjects');
  return bytes({format:'zryna.playground-compiler-publication-evidence.v1',version:1,source,compiler,
    scope:'actual-protected-admission-build-and-review; final-public-acceptance-remains-separate',files:ordered});
}
