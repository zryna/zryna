// Fixed authenticated acquisition path. No injectable verifier and no exported capability mint.
import { LIMITS, bytes, certificateIdentity, exactKeys, issuer, requireValue, sha256,
  sourceRef, verifierPins } from './compiler-publication-constants.mjs';
import { validateCompilerPublicationEnvelope, validateCompilerPublicationPolicy } from './compiler-publication-schema.mjs';
import { verifyCompilerPublicationArchive } from './compiler-publication-archive.mjs';
import { verifyCompilerPublicationSignature } from './compiler-publication-verifier.mjs';

export async function verifyCompilerPublication(input) {
  exactKeys(input, ['envelope', 'bundle', 'archive', 'policy', 'verifierPolicy', 'trustedRoot']);
  for (const [name, maximum] of [['envelope', LIMITS.record], ['bundle', 1048576],
    ['archive', LIMITS.archive], ['trustedRoot', 262144]]) {
    requireValue(input[name] instanceof Uint8Array && input[name].length > 0 &&
      input[name].length <= maximum, 'compiler publication bounded acquisition carrier');
  }
  validateCompilerPublicationPolicy(input.policy);
  const policy = structuredClone(input.policy), verifierPolicy = structuredClone(input.verifierPolicy);
  requireValue(input.archive.length === policy.archive.bytes &&
    input.trustedRoot.length === verifierPins.trustedRootBytes, 'compiler publication selected carrier sizes');
  // Own every caller carrier/expectation before the first asynchronous archive operation.
  const envelope = Buffer.from(input.envelope), bundle = Buffer.from(input.bundle);
  const document = validateCompilerPublicationEnvelope(envelope, policy);
  const archive = Buffer.from(input.archive), root = Buffer.from(input.trustedRoot);
  requireValue(archive.length === document.archive.bytes && sha256(archive) === document.archive.sha256,
    'compiler publication externally selected archive');
  const verification = verifyCompilerPublicationSignature(envelope, bundle, verifierPolicy, root, document.source.commit);
  let verified;
  try { verified = await verifyCompilerPublicationArchive(archive, document); }
  catch (error) { error.verification = verification; throw error; }
  const receipt = { type: 'zryna.playground-compiler-publication.v1', status: 'authenticated',
    product: document.product, productVersion: document.productVersion, target: document.target,
    profile: document.profile, source: document.source, archive: document.archive, compiler: document.binary,
    envelopeSha256: sha256(envelope), provenanceSha256: sha256(bytes(document.provenance)),
    verification: { certificateIdentity, issuer, sourceCommit: document.source.commit,
      repository: 'zryna/zryna', ref: sourceRef, event: 'push', offline: true,
      transparencyAndSctChecksRetained: true, verifier: { ...verifierPins, ownerUid: verifierPolicy.ownerUid },
      result: verification.result, resultSha256: sha256(bytes(verification.result)),
      commandSha256: sha256(bytes(verification.command)), trustedRootSha256: sha256(root) } };
  return { receipt, verification, binary: Buffer.from(verified.binary),
    files: verified.files.map(file => ({ path: file.path, mode: file.mode, data: Buffer.from(file.data) })) };
}
