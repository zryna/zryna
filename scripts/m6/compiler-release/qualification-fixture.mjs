// Synthetic schema fixtures do not qualify an actual operator runtime or mint authority.
import { bytes, describe, sha256 } from './common.mjs';
import { qualificationRegistry,qualificationRegistrySha256 } from './qualification.mjs';
export function qualificationFixture() {
  const source={repository:'https://github.com/zryna/zryna',commit:'a'.repeat(40),tree:'b'.repeat(40)};
  const compiler={bytes:123,sha256:'c'.repeat(64)};
  const files=[{path:'raw/fixture-readonly-confirmation.txt',mode:0o644,data:Buffer.from('SCHEMA FIXTURE ONLY\n')}];
  const record={format:'zryna.playground-compiler-candidate.v1',version:1,status:'candidate-qualified-not-published',
    source,compiler,registrySha256:qualificationRegistrySha256,
    obligations:qualificationRegistry.obligations.map(id=>({id,status:'pass',receipts:[files[0].path]})),
    publishedInputs:{envelopeSha256:'49cbe33ae33d8c926b2ced151435d5e347e42bbe293eef539643a70fccea3ad6',
      archiveSha256:'b258d45e1eb58a701026ced7c74d03dc74c911c90a5ec7da93c656884a97159f',
      verificationReceiptSha256:'d'.repeat(64)},
    materials:Object.fromEntries(['compositionSha256','compilerManifestSha256','evaluatorManifestSha256',
      'compilerImageSha256','evaluatorImageSha256','operatorApprovalSha256'].map(key=>[key,'e'.repeat(64)])),
    files:files.map(file=>describe(file.path,file.data))};
  const review={format:'zryna.playground-compiler-candidate-review.v1',version:1,
    status:'independently-reviewed-actual-positive',source,compiler,registrySha256:qualificationRegistrySha256,
    qualificationSha256:sha256(bytes(record)),rawInventorySha256:sha256(bytes(record.files)),
    reviewer:'schema-fixture-only',confirmation:record.files[0]};
  const selection={reviewSha256:sha256(bytes(review))};
  return {source,compiler,files,record,review,selection};
}
