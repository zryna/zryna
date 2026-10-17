import test from 'node:test';import assert from 'node:assert/strict';
import { mkdtempSync,mkdirSync,writeFileSync,rmSync } from 'node:fs';import { tmpdir } from 'node:os';import { join } from 'node:path';
import { bytes,describe,sha256 } from './common.mjs';import { compareReplicas } from './replicas.mjs';
import { retainPublicationEvidence } from './evidence.mjs';
import { decodePublicationEvidence } from './evidence-layout.mjs';
import { ordinary } from './common.mjs';
import { graphFixture } from './material-fixtures.mjs';
import { validateCompilerTree } from './material-tree.mjs';
function fixtures(){
  const root=mkdtempSync(join(tmpdir(),'m6-compiler-replicas-fixture-'));
  const source={repository:'https://github.com/zryna/zryna',commit:'a'.repeat(40),tree:'b'.repeat(40),sourceDateEpoch:1};
  const admission={source,run:{id:12,attempt:1}},binary=Buffer.from('INERT SCHEMA FIXTURE ONLY'),materials=bytes({source,packages:[]});
  const roots=[join(root,'first'),join(root,'second')],records=[];
  for(let index=0;index<2;index++){
    const dir=roots[index];mkdirSync(join(dir,'licenses'),{recursive:true});writeFileSync(join(dir,'licenses','LICENSE'),Buffer.from('fixture license'));
    const graph=graphFixture(),sourceRoot=`/synthetic-source-${index}`;
    graph.metadata.workspace_root=sourceRoot;
    graph.metadata.packages[0].manifest_path=`${sourceRoot}/apps/zryna-playground-compiler/Cargo.toml`;
    const metadata=bytes(graph.metadata),rawTree=Buffer.from(`zryna-playground-compiler v0.1.0 (${sourceRoot}/apps/zryna-playground-compiler)\nautocfg v1.5.1\n`);
    const closure=bytes({metadataCanonicalSha256:sha256(metadata),source,packages:[],cacheProof:[]});
    const tree=bytes(validateCompilerTree(rawTree,graph.metadata,sourceRoot));
    const metadataArgs=['metadata','--format-version=1','--locked','--offline','--filter-platform','x86_64-unknown-linux-gnu'];
    const treeArgs=['tree','--locked','--offline','--target','x86_64-unknown-linux-gnu','--package',
      'zryna-playground-compiler','--edges','normal,build','--prefix','none','--format','{p}'];
    const buildArgs=['build','--locked','--offline','--release','--target','x86_64-unknown-linux-gnu',
      '--package','zryna-playground-compiler','--bin','zryna-playground-compiler'];
    const args=[['--version'],['--version'],metadataArgs,treeArgs,buildArgs,metadataArgs,treeArgs];
    const outputs=[Buffer.from('cargo 1.97.1 fixture\n'),Buffer.from('rustc 1.97.1 fixture\n'),metadata,rawTree,Buffer.alloc(0),metadata,rawTree];
    const commands=outputs.map((stdout,number)=>{
      const stem=`command-${String(number+1).padStart(2,'0')}`;writeFileSync(join(dir,stem+'.stdout'),stdout);
      writeFileSync(join(dir,stem+'.stderr'),Buffer.alloc(0));
      return {executable:number===1?'/synthetic/rustc':'/synthetic/cargo',args:args[number],status:0,signal:null,errorCode:null,
        stdoutBytes:stdout.length,stderrBytes:0,stdoutSha256:sha256(stdout),stderrSha256:sha256(Buffer.alloc(0))};
    });
    const record={format:'zryna.playground-compiler-replica.v2',version:2,source,replica:index+1,run:admission.run,
      job:{key:'build',name:`independent compiler replica ${index+1}`,id:20+index,runnerId:30+index},runner:'same-valid-display-name',
      compiler:describe('compiler.bin',binary),materials:describe('rust-materials.json',materials),closure:describe('material-closure.json',closure),tree:describe('tree-identities.json',tree),
      revalidation:{metadata:describe('cargo-metadata-after.json',metadata),tree:describe('tree-identities-after.json',tree)},
      inputs:{cargoLock:describe('Cargo.lock',Buffer.from('fixture'))},
      toolchain:{rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'},tools:{},
      recipe:{target:'x86_64-unknown-linux-gnu',package:'zryna-playground-compiler',profile:'release',locked:true,
        incremental:false,sourceCommit:source.commit},commands};
    records.push(record);writeFileSync(join(dir,'compiler.bin'),binary);writeFileSync(join(dir,'rust-materials.json'),materials);
    writeFileSync(join(dir,'material-closure.json'),closure);writeFileSync(join(dir,'tree-identities.json'),tree);writeFileSync(join(dir,'replica.json'),bytes(record));
    writeFileSync(join(dir,'cargo-metadata.json'),metadata);writeFileSync(join(dir,'cargo-metadata-after.json'),metadata);
    writeFileSync(join(dir,'tree-identities-after.json'),tree);
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
    const {files,replicas}=await decodePublicationEvidence(data,f.admission.source);
    assert.equal(files.some(file=>file.path.endsWith('compiler.bin')),false);
    assert.equal(replicas.flat().some(file=>file.path.endsWith('compiler.bin')),false);
    assert(files.some(file=>file.path==='assembly/reproduction.json'));
  }finally{rmSync(f.root,{recursive:true,force:true});}
});

test('replica v2 requires complete successful seven-command before/after selection revalidation',()=>{
  for(const mutation of ['old five commands','changed after metadata','changed after tree','old v1 receipt',
    'altered raw command','arbitrary build argument','missing final raw command']){
    const f=fixtures();try{
      if(mutation==='old five commands')f.records[1].commands.length=5;
      if(mutation==='old v1 receipt'){f.records[1].format='zryna.playground-compiler-replica.v1';f.records[1].version=1;}
      if(mutation==='altered raw command')writeFileSync(join(f.roots[1],'command-07.stdout'),Buffer.from('altered'));
      if(mutation==='arbitrary build argument')f.records[1].commands[4].args=[...f.records[1].commands[4].args,'--all-features'];
      if(mutation==='missing final raw command')rmSync(join(f.roots[1],'command-07.stdout'));
      if(mutation==='changed after metadata'){
        const data=bytes({fixtureOnly:true,independentRoot:'changed'});
        writeFileSync(join(f.roots[1],'cargo-metadata-after.json'),data);
        f.records[1].revalidation.metadata=describe('cargo-metadata-after.json',data);
      }
      if(mutation==='changed after tree'){
        const data=bytes({identitySetSha256:'e'.repeat(64),packages:[]});
        writeFileSync(join(f.roots[1],'tree-identities-after.json'),data);
        f.records[1].revalidation.tree=describe('tree-identities-after.json',data);
      }
      writeFileSync(join(f.roots[1],'replica.json'),bytes(f.records[1]));
      assert.throws(()=>compareReplicas(...f.roots,f.admission));
    }finally{rmSync(f.root,{recursive:true,force:true});}
  }
});
