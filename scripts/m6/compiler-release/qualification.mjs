import { sourceRegistry } from '../source-registry.mjs';
import { decodeTar, encodeTar } from '../../distribution/archive-tar.mjs';
import { bytes, describe, exactKeys, OID, orderedPaths, parseCanonical, receiptSource,
  requireValue, SHA, sha256 } from './common.mjs';
const groups=new Set(['playground','transport','containment','browser','accessibility']);
export const qualificationRegistry=Object.freeze({format:'zryna.playground-compiler-qualification-registry.v1',version:1,
  scope:'prepublication-runtime-candidate-only',obligations:Object.freeze(sourceRegistry.gates.filter(g=>groups.has(g.id))
    .flatMap(g=>g.platforms.flatMap(platform=>g.cases.map(name=>`${g.id}-${platform}/${name}`))).sort())});
export const qualificationRegistrySha256=sha256(bytes(qualificationRegistry));
export function validateQualification(record, review, files, {source,selection,compiler}={}) {
  exactKeys(record,['format','version','status','source','compiler','registrySha256','obligations',
    'publishedInputs','materials','files']);
  exactKeys(review,['format','version','status','source','compiler','registrySha256','qualificationSha256',
    'rawInventorySha256','reviewer','confirmation']);
  requireValue(record.format==='zryna.playground-compiler-candidate.v1' && record.version===1 &&
    record.status==='candidate-qualified-not-published' && review.format==='zryna.playground-compiler-candidate-review.v1' &&
    review.version===1 && review.status==='independently-reviewed-actual-positive' &&
    typeof review.reviewer==='string' && /^[A-Za-z0-9@._/-]{1,128}$/.test(review.reviewer), 'compiler actual candidate review');
  exactKeys(record.source,['repository','commit','tree']);
  requireValue(OID(record.source.commit) && OID(record.source.tree) &&
    bytes(record.source).equals(bytes(receiptSource(source))) && bytes(review.source).equals(bytes(record.source)),
  'compiler candidate exact source');
  exactKeys(record.compiler,['bytes','sha256']);
  requireValue(Number.isSafeInteger(record.compiler.bytes) && record.compiler.bytes>0 &&
    record.compiler.bytes<=128*1024*1024 && SHA(record.compiler.sha256) &&
    bytes(review.compiler).equals(bytes(record.compiler)) &&
    (!compiler || bytes(compiler).equals(bytes(record.compiler))), 'compiler candidate exact release binary');
  requireValue(record.registrySha256===qualificationRegistrySha256 && review.registrySha256===qualificationRegistrySha256 &&
    review.qualificationSha256===sha256(bytes(record)) && sha256(bytes(review))===selection.reviewSha256 &&
    review.rawInventorySha256===sha256(bytes(record.files)), 'compiler candidate independently selected review');
  exactKeys(record.publishedInputs,['envelopeSha256','archiveSha256','verificationReceiptSha256']);
  requireValue(record.publishedInputs.envelopeSha256==='49cbe33ae33d8c926b2ced151435d5e347e42bbe293eef539643a70fccea3ad6' &&
    record.publishedInputs.archiveSha256==='b258d45e1eb58a701026ced7c74d03dc74c911c90a5ec7da93c656884a97159f' &&
    SHA(record.publishedInputs.verificationReceiptSha256),'compiler candidate actual published input identity');
  exactKeys(record.materials,['compositionSha256','compilerManifestSha256','evaluatorManifestSha256',
    'compilerImageSha256','evaluatorImageSha256','operatorApprovalSha256']);
  requireValue(Object.values(record.materials).every(SHA),'compiler candidate exact approved runtime materials');
  requireValue(Array.isArray(record.files) && record.files.length>0 && record.files.length<=480,
    'compiler candidate finite raw inventory');
  orderedPaths(record.files.map(f=>f.path));
  const retained=new Map(files.map(f=>[f.path,f]));let total=0;
  for(const file of record.files){
    exactKeys(file,['path','bytes','sha256']);const actual=retained.get(file.path);
    requireValue(file.path.startsWith('raw/') && Number.isSafeInteger(file.bytes) && file.bytes>0 &&
      file.bytes<=2*1024*1024 && SHA(file.sha256) && actual?.mode===0o644 &&
      file.bytes===actual.data.length && sha256(actual.data)===file.sha256, 'compiler candidate actual raw receipt');
    total+=file.bytes;
  }
  requireValue(total<=48*1024*1024 && retained.size===record.files.length,'compiler candidate complete raw coverage');
  requireValue(Array.isArray(record.obligations) &&
    bytes(record.obligations.map(o=>o.id)).equals(bytes(qualificationRegistry.obligations)),
  'compiler candidate all owning runtime cases');
  for(const item of record.obligations){
    exactKeys(item,['id','status','receipts']);
    requireValue(item.status==='pass' && Array.isArray(item.receipts) && item.receipts.length>0 &&
      item.receipts.length<=16 && new Set(item.receipts).size===item.receipts.length &&
      item.receipts.every(path=>retained.has(path)), 'compiler candidate actual positive case receipts');
  }
  exactKeys(review.confirmation,['path','bytes','sha256']);
  const confirmation=record.files.find(file=>file.path===review.confirmation.path);
  requireValue(confirmation && bytes(confirmation).equals(bytes(review.confirmation)), 'compiler candidate readonly confirmation');
  return record;
}
export async function openQualification(packet, {source,selection,compiler}={}) {
  requireValue(Buffer.isBuffer(packet) && packet.length>0 && packet.length<=64*1024*1024 &&
    sha256(packet)===selection.qualificationSha256,'compiler candidate protected-tag packet pin');
  const files=await decodeTar(packet,'m6-compiler-qualification',0);
  const record=files.find(f=>f.path==='qualification.json'), review=files.find(f=>f.path==='review.json');
  requireValue(record?.mode===0o644 && review?.mode===0o644,'compiler candidate packet records');
  const actual=files.filter(f=>f!==record && f!==review);
  const document=validateQualification(parseCanonical(record.data),parseCanonical(review.data),actual,{source,selection,compiler});
  return {record:document,review:parseCanonical(review.data),files};
}
export async function createQualificationPacket(record,review,rawFiles) {
  const files=[...rawFiles,{path:'qualification.json',mode:0o644,data:bytes(record)},
    {path:'review.json',mode:0o644,data:bytes(review)}].sort((a,b)=>a.path<b.path?-1:1);
  return encodeTar('m6-compiler-qualification',files,0);
}
