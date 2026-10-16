import { join } from 'node:path';
import { bytes, exactKeys, ordinary, parseCanonical, requireValue, sha256, id } from './common.mjs';
import { directoryFiles, inventory } from './files.mjs';
import { validateCompilerTree } from './material-tree.mjs';

function commandProofs(root, commands, metadataBefore, metadataAfter, tree) {
  requireValue(Array.isArray(commands) && commands.length===7,'compiler seven actual command proofs');
  const metadataArgs=['metadata','--format-version=1','--locked','--offline','--filter-platform','x86_64-unknown-linux-gnu'];
  const treeArgs=['tree','--locked','--offline','--target','x86_64-unknown-linux-gnu','--package',
    'zryna-playground-compiler','--edges','normal,build','--prefix','none','--format','{p}'];
  const buildArgs=['build','--locked','--offline','--release','--target','x86_64-unknown-linux-gnu',
    '--package','zryna-playground-compiler','--bin','zryna-playground-compiler'];
  const expected=[['--version'],['--version'],metadataArgs,treeArgs,buildArgs,metadataArgs,treeArgs];
  const captured=commands.map((command,index)=>{
    exactKeys(command,['executable','args','status','signal','errorCode','stdoutBytes','stderrBytes','stdoutSha256','stderrSha256']);
    requireValue(command.status===0 && command.signal===null && command.errorCode===null &&
      typeof command.executable==='string' && command.executable.startsWith('/') &&
      command.executable.endsWith(index===1?'/rustc':'/cargo') &&
      command.executable===commands[index===1?1:0].executable && bytes(command.args).equals(bytes(expected[index])),
    'compiler actual fixed build command proof');
    const name=`command-${String(index+1).padStart(2,'0')}`;
    const maximum=[2,2,64,2,4,64,2][index]*1024*1024;
    const stdout=ordinary(join(root,`${name}.stdout`),maximum),stderr=ordinary(join(root,`${name}.stderr`),maximum);
    requireValue(stdout.length+stderr.length<=maximum && stdout.length===command.stdoutBytes &&
      stderr.length===command.stderrBytes && sha256(stdout)===command.stdoutSha256 && sha256(stderr)===command.stderrSha256,
    'compiler actual raw command correspondence');
    return stdout;
  });
  requireValue(captured[2].equals(metadataBefore) && captured[5].equals(metadataAfter),
    'compiler actual retained metadata command bytes');
  for(const index of [3,6]) {
    const metadata=JSON.parse(index===3?metadataBefore:metadataAfter);
    requireValue(bytes(validateCompilerTree(captured[index],metadata,metadata.workspace_root)).equals(tree),
      'compiler actual retained Cargo tree command bytes');
  }
}
export function loadReplica(root, expectedReplica, admission) {
  const record=parseCanonical(ordinary(join(root,'replica.json')));
  exactKeys(record,['format','version','source','replica','run','job','runner','compiler','inputs','materials',
    'closure','tree','revalidation','toolchain','tools','recipe','commands']);
  requireValue(record.format==='zryna.playground-compiler-replica.v2' && record.version===2 &&
    record.replica===expectedReplica && bytes(record.source).equals(bytes(admission.source)) &&
    bytes(record.run).equals(bytes(admission.run)) && record.job?.key==='build' &&
    record.job.name===`independent compiler replica ${expectedReplica}` &&
    id(record.job.id)===record.job.id && id(record.job.runnerId)===record.job.runnerId &&
    typeof record.runner==='string' && record.runner.length>0 && record.runner.length<=128,
  'compiler actual independent replica identity');
  const binary=ordinary(join(root,'compiler.bin'),128*1024*1024);
  const materials=ordinary(join(root,'rust-materials.json')),closure=ordinary(join(root,'material-closure.json'));
  const tree=ordinary(join(root,'tree-identities.json'));
  exactKeys(record.revalidation,['metadata','tree']);
  const metadataBefore=ordinary(join(root,'cargo-metadata.json'),64*1024*1024);
  const metadataAfter=ordinary(join(root,'cargo-metadata-after.json'),64*1024*1024);
  const treeAfter=ordinary(join(root,'tree-identities-after.json'));
  for(const [descriptor,data,path] of [[record.compiler,binary,'compiler.bin'],
    [record.materials,materials,'rust-materials.json'],[record.closure,closure,'material-closure.json'],[record.tree,tree,'tree-identities.json'],
    [record.revalidation.metadata,metadataAfter,'cargo-metadata-after.json'],
    [record.revalidation.tree,treeAfter,'tree-identities-after.json']]) {
    exactKeys(descriptor,['path','bytes','sha256']);
    requireValue(descriptor.path===path && descriptor.bytes===data.length && descriptor.sha256===sha256(data),
      'compiler replica exact bytes');
  }
  requireValue(bytes(JSON.parse(metadataBefore)).equals(bytes(JSON.parse(metadataAfter))) && tree.equals(treeAfter) &&
    sha256(bytes(JSON.parse(metadataBefore)))===parseCanonical(closure).metadataCanonicalSha256,
  'compiler replica unchanged actual Cargo selection');
  requireValue(bytes(record.toolchain).equals(bytes({rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'})) &&
    bytes(record.recipe).equals(bytes({target:'x86_64-unknown-linux-gnu',package:'zryna-playground-compiler',
      profile:'release',locked:true,incremental:false,sourceCommit:admission.source.commit})),
  'compiler replica fixed build recipe');
  commandProofs(root,record.commands,metadataBefore,metadataAfter,tree);
  const licenses=directoryFiles(join(root,'licenses'),{maximumFile:2*1024*1024,maximumTotal:32*1024*1024})
    .map(file=>({...file,path:`licenses/${file.path}`}));
  return {record,binary,materials,closure,tree:parseCanonical(tree),licenses};
}
export function compareReplicas(firstRoot,secondRoot,admission) {
  const first=loadReplica(firstRoot,1,admission),second=loadReplica(secondRoot,2,admission);
  requireValue(first.record.job.id!==second.record.job.id && first.record.job.runnerId!==second.record.job.runnerId,
    'compiler distinct actual hosted replica jobs/runners');
  requireValue(first.binary.equals(second.binary) && first.materials.equals(second.materials) &&
    equivalentClosure(first.closure,second.closure) && first.tree.identitySetSha256===second.tree.identitySetSha256 &&
    bytes(first.tree.packages).equals(bytes(second.tree.packages)) && bytes(first.record.inputs).equals(bytes(second.record.inputs)) &&
    bytes(inventory(first.licenses)).equals(bytes(inventory(second.licenses))),
  'compiler actual byte-identical independent builds');
  return {first,second,reproduction:{status:'byte-identical',firstBuildSha256:sha256(first.binary),
    secondBuildSha256:sha256(second.binary)}};
}

function equivalentClosure(first,second) {
  const left=parseCanonical(first),right=parseCanonical(second);
  // Raw Cargo metadata retains independent absolute roots; both raw identities stay in receipts.
  delete left.metadataCanonicalSha256;delete right.metadataCanonicalSha256;
  return bytes(left).equals(bytes(right));
}
