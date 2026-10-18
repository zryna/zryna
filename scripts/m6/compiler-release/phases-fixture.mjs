// Inert synthetic receipts/ELF only; no compiler execution or actual qualification.
import { mkdtempSync,mkdirSync,writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';import { join,dirname } from 'node:path';
import { bytes,describe,sha256 } from './common.mjs';
import { graphFixture } from './material-fixtures.mjs';import { validateCompilerTree } from './material-tree.mjs';
import { compilerPublicationFixture } from '../../../examples/playground/restricted/daemon/compiler-publication-fixture.mjs';
import { binaryPath,inputPaths } from '../../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { qualificationFixture } from './qualification-fixture.mjs';import { createQualificationPacket } from './qualification.mjs';
export async function phaseFixture(){
  const root=mkdtempSync(join(tmpdir(),'m6-compiler-replicas-fixture-'));
  const publication=compilerPublicationFixture(),source=publication.source;
  const admission={format:'zryna.playground-compiler-admission.v1',version:1,
    status:'protected-tag-source-admitted',source,publishing:true,run:{id:12,attempt:1},
    qualification:null,ci:{...publication.provenance.requiredCi,jobsSha256:'e'.repeat(64)}};
  const binary=publication.files.find(f=>f.path===binaryPath).data,materials=publication.materials;
  const sourceRoot=join(root,'source');mkdirSync(sourceRoot);
  const inputs=Object.fromEntries(Object.entries(inputPaths).map(([key,path])=>{
    const data=Buffer.from('synthetic:'+path),target=join(sourceRoot,path);mkdirSync(dirname(target),{recursive:true});writeFileSync(target,data);
    return [key,describe(path,data)];
  }));
  for(const file of publication.files.filter(f=>['LICENSE','NOTICE'].includes(f.path)))writeFileSync(join(sourceRoot,file.path),file.data);
  const roots=[join(root,'first'),join(root,'second')],records=[];
  for(let index=0;index<2;index++){
    const dir=roots[index];mkdirSync(join(dir,'licenses'),{recursive:true});
    for(const file of publication.files.filter(f=>f.path.startsWith('licenses/'))){const target=join(dir,file.path);mkdirSync(dirname(target),{recursive:true});writeFileSync(target,file.data);}
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
      inputs,
      toolchain:{rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'},tools:{},
      recipe:{target:'x86_64-unknown-linux-gnu',package:'zryna-playground-compiler',profile:'release',locked:true,
        incremental:false,sourceCommit:source.commit},commands};
    records.push(record);writeFileSync(join(dir,'compiler.bin'),binary);writeFileSync(join(dir,'rust-materials.json'),materials);
    writeFileSync(join(dir,'material-closure.json'),closure);writeFileSync(join(dir,'tree-identities.json'),tree);writeFileSync(join(dir,'replica.json'),bytes(record));
    writeFileSync(join(dir,'cargo-metadata.json'),metadata);writeFileSync(join(dir,'cargo-metadata-after.json'),metadata);
    writeFileSync(join(dir,'tree-identities-after.json'),tree);
  }
  const q=qualificationFixture();q.record.source={repository:source.repository,commit:source.commit,tree:source.tree};
  q.record.compiler={bytes:binary.length,sha256:sha256(binary)};q.review.source=q.record.source;q.review.compiler=q.record.compiler;
  q.review.qualificationSha256=sha256(bytes(q.record));q.review.rawInventorySha256=sha256(bytes(q.record.files));
  const packet=await createQualificationPacket(q.record,q.review,q.files);
  const selection={ciRun:1,ciAttempt:1,qualificationAsset:456,qualificationSha256:sha256(packet),reviewSha256:sha256(bytes(q.review))};
  admission.selection=selection;
  const admissionRoot=join(root,'admission');mkdirSync(admissionRoot);writeFileSync(join(admissionRoot,'admission.json'),bytes(admission));
  const qualification={transport:{releaseId:789,assetId:456,tag:'m6-qualification-fixture',draft:true},compiler:q.record.compiler,
    packet:describe('qualification.tar.gz',packet),reviewSha256:selection.reviewSha256};
  const qualified={...admission,status:'protected-tag-admitted',qualification};
  return{root,roots,records,admission,binary,sourceRoot,admissionRoot,packet,qualified};
}
