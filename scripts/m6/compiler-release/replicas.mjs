import { join } from 'node:path';
import { bytes, exactKeys, ordinary, parseCanonical, requireValue, sha256, id } from './common.mjs';
import { directoryFiles, inventory } from './files.mjs';
export function loadReplica(root, expectedReplica, admission) {
  const record=parseCanonical(ordinary(join(root,'replica.json')));
  exactKeys(record,['format','version','source','replica','run','job','runner','compiler','inputs','materials',
    'closure','tree','toolchain','tools','recipe','commands']);
  requireValue(record.format==='zryna.playground-compiler-replica.v1' && record.version===1 &&
    record.replica===expectedReplica && bytes(record.source).equals(bytes(admission.source)) &&
    bytes(record.run).equals(bytes(admission.run)) && record.job?.key==='build' &&
    record.job.name===`independent compiler replica ${expectedReplica}` &&
    id(record.job.id)===record.job.id && id(record.job.runnerId)===record.job.runnerId &&
    typeof record.runner==='string' && record.runner.length>0 && record.runner.length<=128,
  'compiler actual independent replica identity');
  const binary=ordinary(join(root,'compiler.bin'),128*1024*1024);
  const materials=ordinary(join(root,'rust-materials.json')),closure=ordinary(join(root,'material-closure.json'));
  const tree=ordinary(join(root,'tree-identities.json'));
  for(const [descriptor,data,path] of [[record.compiler,binary,'compiler.bin'],
    [record.materials,materials,'rust-materials.json'],[record.closure,closure,'material-closure.json'],[record.tree,tree,'tree-identities.json']]) {
    exactKeys(descriptor,['path','bytes','sha256']);
    requireValue(descriptor.path===path && descriptor.bytes===data.length && descriptor.sha256===sha256(data),
      'compiler replica exact bytes');
  }
  requireValue(bytes(record.toolchain).equals(bytes({rust:'1.97.1',node:'22.22.1',pnpm:'11.18.0'})) &&
    bytes(record.recipe).equals(bytes({target:'x86_64-unknown-linux-gnu',package:'zryna-playground-compiler',
      profile:'release',locked:true,incremental:false,sourceCommit:admission.source.commit})),
  'compiler replica fixed build recipe');
  requireValue(Array.isArray(record.commands) && record.commands.length===5 && record.commands.every(command=>
    command.status===0 && command.signal===null && command.errorCode===null),'compiler actual build command proof');
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
