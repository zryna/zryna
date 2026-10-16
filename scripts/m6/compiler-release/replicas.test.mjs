import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtempSync,mkdirSync,writeFileSync,rmSync } from 'node:fs';import { tmpdir } from 'node:os';import { join } from 'node:path';
import { bytes,describe,sha256 } from './common.mjs';import { compareReplicas } from './replicas.mjs';
import { retainPublicationEvidence } from './evidence.mjs';import { decodeTar } from '../../distribution/archive-tar.mjs';
import { ordinary } from './common.mjs';
function fixtures(){
  const root=mkdtempSync(join(tmpdir(),'m6-compiler-replicas-fixture-'));
  const source={repository:'https://github.com/zryna/zryna',commit:'a'.repeat(40),tree:'b'.repeat(40),sourceDateEpoch:1};
  const admission={source,run:{id:12,attempt:1}},binary=Buffer.from('INERT SCHEMA FIXTURE ONLY'),materials=bytes({source,packages:[]});
  const roots=[join(root,'first'),join(root,'second')],records=[];
  for(let index=0;index<2;index++){
    const dir=roots[index];mkdirSync(join(dir,'licenses'),{recursive:true});writeFileSync(join(dir,'licenses','LICENSE'),Buffer.from('fixture license'));
    const closure=bytes({metadataCanonicalSha256:String(index).repeat(64),source,packages:[],cacheProof:[]});
    const tree=bytes({identitySetSha256:'d'.repeat(64),packages:[]});
    const record={format:'zryna.playground-compiler-replica.v1',version:1,source,replica:index+1,run:admission.run,
      job:{key:'build',name:`independent compiler replica ${index+1}`,id:20+index,runnerId:30+index},runner:'same-valid-display-name',
      compiler:describe('compiler.bin',binary),materials:describe('rust-materials.json',materials),closure:describe('material-closure.json',closure),tree:describe('tree-identities.json',tree),
      inputs:{cargoLock:describe('Cargo.lock',Buffer.from('fixture'))},
      toolchain:{rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'},tools:{},
      recipe:{target:'x86_64-unknown-linux-gnu',package:'zryna-playground-compiler',profile:'release',locked:true,
        incremental:false,sourceCommit:source.commit},commands:Array.from({length:5},()=>({status:0,signal:null,errorCode:null}))};
    records.push(record);writeFileSync(join(dir,'compiler.bin'),binary);writeFileSync(join(dir,'rust-materials.json'),materials);
    writeFileSync(join(dir,'material-closure.json'),closure);writeFileSync(join(dir,'tree-identities.json'),tree);writeFileSync(join(dir,'replica.json'),bytes(record));
  }
  return{root,roots,records,admission,binary};
}
test('independent replicas may share display name and retain distinct raw metadata identities',()=>{
  const f=fixtures();try{assert.equal(compareReplicas(...f.roots,f.admission).reproduction.status,'byte-identical');}
  finally{rmSync(f.root,{recursive:true,force:true});}
});
for(const name of ['same actual job','same actual runner','changed binary','changed material','changed closure semantics','changed source'])
  test(`real replica correspondence rejects ${name}`,()=>{
    const f=fixtures();try{
      if(name==='same actual job')f.records[1].job.id=f.records[0].job.id;
      if(name==='same actual runner')f.records[1].job.runnerId=f.records[0].job.runnerId;
      if(name==='changed source')f.records[1].source={...f.records[1].source,tree:'f'.repeat(40)};
      if(name==='changed binary'){const b=Buffer.from('DIFFERENT INERT FIXTURE');writeFileSync(join(f.roots[1],'compiler.bin'),b);f.records[1].compiler=describe('compiler.bin',b);}
      if(name==='changed material'){const b=bytes({source:f.admission.source,packages:['different']});writeFileSync(join(f.roots[1],'rust-materials.json'),b);f.records[1].materials=describe('rust-materials.json',b);}
      if(name==='changed closure semantics'){const b=bytes({metadataCanonicalSha256:'1'.repeat(64),source:f.admission.source,packages:['changed'],cacheProof:[]});writeFileSync(join(f.roots[1],'material-closure.json'),b);f.records[1].closure=describe('material-closure.json',b);}
      writeFileSync(join(f.roots[1],'replica.json'),bytes(f.records[1]));assert.throws(()=>compareReplicas(...f.roots,f.admission));
    }finally{rmSync(f.root,{recursive:true,force:true});}
  });
test('durable evidence excludes separately verified binaries BEFORE reading them',async()=>{
  const f=fixtures();try{
    const admissionRoot=join(f.root,'admission'),assembledRoot=join(f.root,'assembled'),output=join(f.root,'output');
    for(const path of [admissionRoot,assembledRoot,output])mkdirSync(path);
    writeFileSync(join(admissionRoot,'admission.json'),bytes(f.admission));
    writeFileSync(join(assembledRoot,'reproduction.json'),bytes({status:'fixture-only'}));
    // An unreadable/oversize excluded subject is not opened or part of this evidence payload.
    rmSync(join(f.roots[0],'compiler.bin'));mkdirSync(join(f.roots[0],'compiler.bin'));
    const descriptor=await retainPublicationEvidence({admissionRoot,firstRoot:f.roots[0],secondRoot:f.roots[1],
      assembledRoot,output,source:f.admission.source});
    const data=ordinary(join(output,descriptor.path));assert.equal(descriptor.sha256,sha256(data));
    const files=await decodeTar(data,'m6-compiler-publication-evidence',1);
    assert.equal(files.some(file=>file.path.endsWith('compiler.bin')),false);
    assert(files.some(file=>file.path==='assembly/reproduction.json'));
  }finally{rmSync(f.root,{recursive:true,force:true});}
});
