import { dirname, join, resolve, basename } from 'node:path';
import { existsSync, mkdirSync } from 'node:fs';
import { captureCompilerMaterials, selectCompilerCrates } from './materials.mjs';
import { validateCompilerTree } from './material-tree.mjs';
import { github } from './github.mjs';
import { captureSource } from './source.mjs';
import { bytes, command, describe, id, ordinary, parseCanonical, requireValue, sha256,
  TARGET, PRODUCT, writeNew } from './common.mjs';
import { inputPaths } from '../../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
function fixedEnvironment({sourceRoot,cargoHome,targetRoot,source,cargo,rustc,environment}) {
  for(const path of [sourceRoot,cargoHome,targetRoot,cargo,rustc]) {
    requireValue(typeof path==='string' && path===resolve(path) && !/[\s\x00-\x1f]/.test(path),
      'compiler build fixed absolute paths');
  }
  return {PATH:`${dirname(rustc)}:/usr/bin:/bin`,HOME:dirname(cargoHome),CARGO_HOME:cargoHome,
    RUSTUP_HOME:environment.RUSTUP_HOME,LANG:'C.UTF-8',TZ:'UTC',RUSTC:rustc,CARGO:cargo,
    CARGO_TARGET_DIR:targetRoot,CARGO_INCREMENTAL:'0',CARGO_BUILD_JOBS:'2',CARGO_TERM_COLOR:'never',
    SOURCE_DATE_EPOCH:String(source.sourceDateEpoch),ZRYNA_PLAYGROUND_SOURCE_COMMIT:source.commit,
    RUSTFLAGS:[`--remap-path-prefix=${sourceRoot}=/zryna/source`,
      `--remap-path-prefix=${cargoHome}=/zryna/cargo`,`--remap-path-prefix=${targetRoot}=/zryna/target`].join(' ')};
}
export async function buildCompiler({sourceRoot,cargoHome,output,admissionRoot,replica,
  environment=process.env,api=github()}={}) {
  requireValue(process.platform==='linux' && process.arch==='x64' && [1,2].includes(replica),
    'compiler independent Linux replica');
  const sourceCapture=captureSource({cwd:sourceRoot,environment});
  const admission=parseCanonical(ordinary(join(admissionRoot,'admission.json')));
  requireValue(bytes(sourceCapture.source).equals(bytes(admission.source)) &&
    admission.run.id===id(environment.GITHUB_RUN_ID) && admission.run.attempt===id(environment.GITHUB_RUN_ATTEMPT),
  'compiler build admission source/run');
  requireValue(!existsSync(output),'compiler build new output root');mkdirSync(output,{mode:0o700});
  const jobs=await api.request(`/repos/zryna/zryna/actions/runs/${admission.run.id}/attempts/${admission.run.attempt}/jobs?per_page=100`);
  requireValue(Array.isArray(jobs.jobs) && jobs.total_count===jobs.jobs.length && jobs.jobs.length<=100,
    'compiler actual build job listing');
  const own=jobs.jobs.filter(job=>job.name===`independent compiler replica ${replica}`);
  requireValue(own.length===1 && own[0].run_id===admission.run.id && own[0].run_attempt===admission.run.attempt &&
    own[0].head_sha===admission.source.commit && own[0].status==='in_progress' && environment.GITHUB_JOB==='build',
  'compiler actual own running replica job');
  const jobIdentity={id:id(own[0].id),name:own[0].name,key:'build',runnerId:id(own[0].runner_id)};
  writeNew(join(output,'job-observation.json'),bytes(jobs));

  const targetRoot=join(dirname(output),'target-release');
  requireValue(!existsSync(targetRoot),'compiler build fresh target root');
  for(let path=sourceRoot;;path=dirname(path)) {
    requireValue(!existsSync(join(path,'.cargo/config')) && !existsSync(join(path,'.cargo/config.toml')),
      'compiler build no ambient Cargo config');if(dirname(path)===path)break;
  }
  requireValue(!existsSync(join(cargoHome,'config')) && !existsSync(join(cargoHome,'config.toml')),
    'compiler build no Cargo home override');
  const tool=(name)=>command('rustup',['which','--toolchain','1.97.1',name],{cwd:sourceRoot}).stdout.toString().trim();
  const cargo=tool('cargo'),rustc=tool('rustc');
  const env=fixedEnvironment({sourceRoot,cargoHome,targetRoot,source:admission.source,cargo,rustc,environment});
  const proofs=[];
  function run(executable,args,maximum=2*1024*1024,timeout=120000) {
    try {
      const result=command(executable,args,{cwd:sourceRoot,env,maximum,timeout});
      const number=String(proofs.length+1).padStart(2,'0');
      writeNew(join(output,`command-${number}.stdout`),result.stdout);
      writeNew(join(output,`command-${number}.stderr`),result.stderr);proofs.push(result.record);return result.stdout;
    } catch(error) {
      if(error.proof){writeNew(join(output,'failure.json'),bytes(error.proof.record));
        writeNew(join(output,'failure.stdout'),error.proof.stdout);writeNew(join(output,'failure.stderr'),error.proof.stderr);}
      throw error;
    }
  }
  requireValue(run(cargo,['--version']).toString().startsWith('cargo 1.97.1 ') &&
    run(rustc,['--version']).toString().startsWith('rustc 1.97.1 '),'compiler actual toolchain versions');
  function graph(suffix) {
    const metadataBytes=run(cargo,['metadata','--format-version=1','--locked','--offline','--filter-platform',TARGET],
      64*1024*1024);
    writeNew(join(output,`cargo-metadata${suffix}.json`),metadataBytes);
    const metadata=JSON.parse(metadataBytes);
    const treeBytes=run(cargo,['tree','--locked','--offline','--target',TARGET,'--package',PRODUCT,
      '--edges','normal,build','--prefix','none','--format','{p}']);
    const treeProof=bytes(validateCompilerTree(treeBytes,metadata,sourceRoot));
    writeNew(join(output,`tree-identities${suffix}.json`),treeProof);
    return {metadataBytes,metadata,treeBytes,treeProof};
  }
  const beforeGraph=graph(''),cargoLock=ordinary(join(sourceRoot,'Cargo.lock'));
  const selected=selectCompilerCrates(beforeGraph.metadata,sourceRoot,beforeGraph.treeBytes);
  const crateArchives=selected.map(pkg=>{
    const cacheRoot=dirname(pkg.manifest_path),index=basename(dirname(cacheRoot));
    return {identity:`${pkg.name}-${pkg.version}`,cacheRoot,
      archive:ordinary(join(cargoHome,'registry/cache',index,`${pkg.name}-${pkg.version}.crate`),64*1024*1024)};
  });
  const upstreamPath=environment.M6_WASMTIME_LICENSE;
  const upstreamLicense=upstreamPath?ordinary(upstreamPath):null;
  const capture=value=>captureCompilerMaterials({metadata:value.metadata,cargoTree:value.treeBytes,
    source:admission.source,sourceRoot,cargoLock,crateArchives,upstreamLicense});
  const material=capture(beforeGraph);
  run(cargo,['build','--locked','--offline','--release','--target',TARGET,'--package',PRODUCT,'--bin',PRODUCT],
    4*1024*1024,30*60*1000);
  const afterGraph=graph('-after');
  requireValue(bytes(beforeGraph.metadata).equals(bytes(afterGraph.metadata)) &&
    beforeGraph.treeProof.equals(afterGraph.treeProof),'compiler build unchanged actual Cargo selection');
  const after=capture(afterGraph);
  requireValue(material.materials.equals(after.materials) && material.closure.equals(after.closure),
    'compiler build unchanged dependency cache');
  const afterSource=captureSource({cwd:sourceRoot,environment});
  requireValue(bytes(afterSource.source).equals(bytes(admission.source)),'compiler build unchanged source');
  const binary=ordinary(join(targetRoot,TARGET,'release',PRODUCT),128*1024*1024);
  writeNew(join(output,'compiler.bin'),binary);writeNew(join(output,'rust-materials.json'),material.materials);
  writeNew(join(output,'material-closure.json'),material.closure);
  mkdirSync(join(output,'licenses'),{mode:0o700});
  for(const file of material.files){const path=join(output,file.path);mkdirSync(dirname(path),{recursive:true,mode:0o700});writeNew(path,file.data);}
  const inputs=Object.fromEntries(Object.entries(inputPaths).map(([key,path])=>[key,describe(path,ordinary(join(sourceRoot,path)))]));
  const receipt={format:'zryna.playground-compiler-replica.v2',version:2,source:admission.source,replica,
    run:admission.run,job:jobIdentity,runner:environment.RUNNER_NAME,
    compiler:describe('compiler.bin',binary),inputs,materials:describe('rust-materials.json',material.materials),
    closure:describe('material-closure.json',material.closure),tree:describe('tree-identities.json',beforeGraph.treeProof),
    revalidation:{metadata:describe('cargo-metadata-after.json',afterGraph.metadataBytes),
      tree:describe('tree-identities-after.json',afterGraph.treeProof)},toolchain:{rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'},
    tools:{cargo:describe('cargo',ordinary(cargo,64*1024*1024)),rustc:describe('rustc',ordinary(rustc,256*1024*1024))},
    recipe:{target:TARGET,package:PRODUCT,profile:'release',locked:true,incremental:false,sourceCommit:admission.source.commit},
    commands:proofs};
  writeNew(join(output,'replica.json'),bytes(receipt));return receipt;
}
