import {access,lstat,realpath,writeFile} from 'node:fs/promises';
import {constants} from 'node:fs';
import {delimiter,join} from 'node:path';
import {performance} from 'node:perf_hooks';
import {descriptor} from './scripts/m6/installed-editor/files.mjs';
const deadline=performance.now()+15000;
const observations=[];
for(const program of ['git','tar']) {
  let selected;
  for(const dir of (process.env.PATH??process.env.Path??'').split(delimiter).filter(Boolean)) {
    for(const candidate of [join(dir,program),join(dir,program+'.exe')]) {
      if(performance.now()>=deadline)throw new Error('Original metadata deadline expired');
      try {await access(candidate,constants.X_OK);selected=await realpath(candidate);break;}catch{}
    }
    if(selected)break;
  }
  if(!selected)throw new Error('Expected original tool unavailable');
  const stat=await lstat(selected,{bigint:true});
  const tuple=Object.fromEntries(['dev','ino','mode','uid','gid','nlink','size','mtimeNs','ctimeNs'].map(k=>[k,String(stat[k])]));
  let pin=null,error=null;
  try {pin=await descriptor(selected,deadline);}catch(e){error=String(e.message).slice(0,2048);}
  observations.push({program,selected,earlierMetadataTuple:tuple,earlierTupleIsNotDescriptorGuardProof:true,ordinaryFile:stat.isFile(),symbolicLink:stat.isSymbolicLink(),strictDescriptorAccepted:pin!==null,pin,error});
}
const result={diagnosticOnly:true,qualification:false,targetExecuted:false,node:process.version,platform:process.platform,runnerImage:process.env.ImageOS,runnerImageVersion:process.env.ImageVersion,observations};
const text=JSON.stringify(result,null,2)+'\n';if(Buffer.byteLength(text)>32768)throw new Error('Metadata cap');
await writeFile(process.env.M6_TOOL_METADATA_RESULT,text,{flag:'wx'});
process.stdout.write(text);
