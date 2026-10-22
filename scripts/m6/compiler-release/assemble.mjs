import { mkdirSync } from 'node:fs';
import { join } from 'node:path';
import { createUnsignedCompilerPublication } from '../../../examples/playground/restricted/daemon/compiler-publication-producer.mjs';
import { inputPaths, binaryPath } from '../../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { bytes, describe, ordinary, parseCanonical, requireValue, writeNew, WORKFLOW, REF,
  TARGET, PRODUCT } from './common.mjs';
import { compareReplicas } from './replicas.mjs';
import { openQualification } from './qualification.mjs';
const README=source=>Buffer.from(`Zryna restricted playground compiler 0.1.0\n\nLinux x86_64 compiler executable built from ${source.commit}.\nThis archive contains the compiler and its license/build materials.\nUse the reviewed-source playground launcher from this exact source revision.\nNode and the legacy nine-file provider closure require independent authentication\nfrom the original immutable v0.2.3 release. Chrome, source composition, runtime\nimages and operator-sealed materials require their separately reviewed approvals.\nNo standalone toolkit, automatic guest authority or unrestricted host execution\nis granted by acquiring this archive.\n`);
export async function assembleCompiler({sourceRoot,admissionRoot,firstRoot,secondRoot,output,deferQualification=false}={}) {
  const admission=parseCanonical(ordinary(join(admissionRoot,'admission.json')));
  requireValue(typeof deferQualification==='boolean','compiler assembly qualification phase');
  requireValue(admission.publishing ? admission.status===(deferQualification?'protected-tag-source-admitted':'protected-tag-admitted') &&
    (deferQualification ? admission.qualification===null : admission.qualification!==null && typeof admission.qualification==='object') :
    admission.status==='candidate-build-only' && admission.qualification===null,
  'compiler assembly explicit admission phase');
  const {first,second,reproduction}=compareReplicas(firstRoot,secondRoot,admission);
  for(const [key,path] of Object.entries(inputPaths)) {
    requireValue(bytes(first.record.inputs[key]).equals(bytes(describe(path,ordinary(join(sourceRoot,path))))),
      'compiler assembly exact locked source inputs');
  }
  mkdirSync(output,{recursive:false,mode:0o700});
  writeNew(join(output,'reproduction.json'),bytes({format:'zryna.playground-compiler-reproduction.v1',version:1,
    source:admission.source,first:first.record,second:second.record,reproduction}));
  if(!admission.publishing) {
    writeNew(join(output,'compiler.bin'),first.binary);writeNew(join(output,'rust-materials.json'),first.materials);
    writeNew(join(output,'candidate.json'),bytes({format:'zryna.playground-compiler-build-candidate.v1',version:1,
      status:'unpublished-unqualified-candidate',source:admission.source,
      compiler:{bytes:first.binary.length,sha256:reproduction.firstBuildSha256},run:admission.run,
      ci:admission.ci,reproduction,inputs:first.record.inputs,materials:first.record.materials,closure:first.record.closure}));
    return {status:'unpublished-unqualified-candidate'};
  }
  if(!deferQualification)await openQualification(ordinary(join(admissionRoot,'qualification.tar.gz'),64*1024*1024),
    {source:admission.source,selection:admission.selection,
      compiler:{bytes:first.binary.length,sha256:reproduction.firstBuildSha256}});
  const ci={...admission.ci};delete ci.jobsSha256;
  const publication=await createUnsignedCompilerPublication({source:admission.source,materials:first.materials,
    files:[{path:binaryPath,mode:0o755,data:first.binary},
      ...['LICENSE','NOTICE'].map(path=>({path,mode:0o644,data:ordinary(join(sourceRoot,path))})),
      {path:'README.md',mode:0o644,data:README(admission.source)},...first.licenses],
    provenance:{workflow:{path:WORKFLOW,ref:REF,sha:admission.source.commit,event:'push'},
      toolchain:first.record.toolchain,inputs:first.record.inputs,
      build:{package:PRODUCT,profile:'release',target:TARGET,locked:true,sourceCommitEnv:admission.source.commit},
      requiredCi:ci,reproduction}});
  writeNew(join(output,'compiler-envelope.json'),publication.envelope);
  writeNew(join(output,publication.document.archive.path),publication.archive);
  writeNew(join(output,'candidate-policy.json'),bytes(publication.candidatePolicy));
  return deferQualification?{...publication,status:'unsigned-unqualified-publication'}:publication;
}
