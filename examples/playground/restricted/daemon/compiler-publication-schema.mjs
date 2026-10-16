// Pure closed-record validation creates no executable or publication capability.
import { orderedPaths, parseCanonical } from '../../../../scripts/distribution/canonical.mjs';
import { LIMITS, archivePath, binaryPath, bytes, certificateIdentity, descriptor, equal,
  exactKeys, hash, inputPaths, issuer, metadataPaths, positive, product,
  productVersion, profile, requireValue, sha256, sourceRef, target, validateSource,
  workflowPath } from './compiler-publication-constants.mjs';

export function validateCompilerPublicationPolicy(policy) {
  exactKeys(policy, ['version', 'source', 'envelopeSha256', 'archive', 'binary', 'provenanceSha256']);
  validateSource(policy.source);
  descriptor(policy.archive, archivePath, LIMITS.archive);
  descriptor(policy.binary, binaryPath, LIMITS.binary);
  requireValue(policy.version === 1 && hash(policy.envelopeSha256) && hash(policy.provenanceSha256),
    'compiler publication independent policy');
}

export function validateProvenance(provenance, document) {
  exactKeys(provenance, ['workflow', 'toolchain', 'inputs', 'build', 'buildReceipt',
    'rustMaterials', 'sbom', 'requiredCi', 'reproduction']);
  exactKeys(provenance.workflow, ['path', 'ref', 'sha', 'event']);
  requireValue(equal(provenance.workflow, { path: workflowPath, ref: sourceRef,
    sha: document.source.commit, event: 'push' }), 'compiler publication builder');
  exactKeys(provenance.toolchain, ['rust', 'node', 'pnpm']);
  requireValue(equal(provenance.toolchain, { rust: '1.97.1', node: '22.22.1', pnpm: '11.18.0' }),
    'compiler publication toolchain');
  exactKeys(provenance.inputs, Object.keys(inputPaths));
  for (const [name, path] of Object.entries(inputPaths)) descriptor(provenance.inputs[name], path);
  exactKeys(provenance.build, ['package', 'profile', 'target', 'locked', 'sourceCommitEnv']);
  requireValue(equal(provenance.build, { package: product, profile: 'release', target,
    locked: true, sourceCommitEnv: document.source.commit }), 'compiler publication build recipe');
  for (const [name, path] of Object.entries(metadataPaths)) descriptor(provenance[name], path);
  exactKeys(provenance.requiredCi, ['workflowPath', 'runId', 'runAttempt', 'headSha', 'headTree', 'conclusion']);
  const ci = provenance.requiredCi;
  requireValue(ci.workflowPath === '.github/workflows/ci.yml' && positive(ci.runId) && positive(ci.runAttempt) &&
    ci.headSha === document.source.commit && ci.headTree === document.source.tree && ci.conclusion === 'success',
  'compiler publication required CI claim');
  exactKeys(provenance.reproduction, ['status', 'firstBuildSha256', 'secondBuildSha256']);
  const reproduction = provenance.reproduction;
  requireValue(reproduction.status === 'byte-identical' &&
    reproduction.firstBuildSha256 === document.binary.sha256 &&
    reproduction.secondBuildSha256 === document.binary.sha256, 'compiler publication two-build reproduction');
}

function validateFiles(document) {
  requireValue(Array.isArray(document.files) && document.files.length >= 7 &&
    document.files.length <= LIMITS.files, 'compiler publication complete inventory');
  orderedPaths(document.files.map(file => file.path));
  let expanded = 0;
  for (const file of document.files) {
    exactKeys(file, ['path', 'bytes', 'sha256', 'mode']);
    const binary = file.path === binaryPath;
    requireValue(positive(file.bytes, binary ? LIMITS.binary : LIMITS.text) && hash(file.sha256) &&
      file.mode === (binary ? 0o755 : 0o644), 'compiler publication file identity');
    requireValue(binary || ['LICENSE', 'NOTICE', 'README.md', ...Object.values(metadataPaths)].includes(file.path) ||
      file.path.startsWith('licenses/'), 'compiler publication unknown file');
    expanded += file.bytes;
    requireValue(expanded <= LIMITS.expanded, 'compiler publication expanded file budget');
  }
  const files = new Map(document.files.map(file => [file.path, file]));
  for (const path of [binaryPath, 'LICENSE', 'NOTICE', 'README.md', ...Object.values(metadataPaths)]) {
    requireValue(files.has(path), 'compiler publication required file');
  }
  const binary = files.get(binaryPath);
  requireValue(equal({ path: binary.path, bytes: binary.bytes, sha256: binary.sha256 }, document.binary),
    'compiler publication binary inventory');
  for (const name of Object.keys(metadataPaths)) {
    const file = files.get(metadataPaths[name]);
    requireValue(equal({ path: file.path, bytes: file.bytes, sha256: file.sha256 }, document.provenance[name]),
      'compiler publication metadata inventory');
  }
}

export function validateCompilerPublicationEnvelope(input, policy) {
  validateCompilerPublicationPolicy(policy);
  requireValue(input instanceof Uint8Array && input.length > 0 && input.length <= LIMITS.record,
    'compiler publication envelope byte budget');
  const wire = Buffer.from(input);
  requireValue(sha256(wire) === policy.envelopeSha256, 'compiler publication independently pinned envelope');
  const document = parseCanonical(wire);
  exactKeys(document, ['format', 'version', 'product', 'productVersion', 'target', 'profile',
    'source', 'signing', 'archive', 'binary', 'provenance', 'files']);
  requireValue(document.format === 'zryna.playground-compiler-release.v1' && document.version === 1 &&
    document.product === product && document.productVersion === productVersion &&
    document.target === target && document.profile === profile, 'compiler publication product');
  validateSource(document.source);
  requireValue(equal(document.source, policy.source), 'compiler publication independently selected source');
  exactKeys(document.signing, ['issuer', 'certificateIdentity']);
  requireValue(equal(document.signing, { issuer, certificateIdentity }), 'compiler publication signer');
  descriptor(document.archive, archivePath, LIMITS.archive);
  descriptor(document.binary, binaryPath, LIMITS.binary);
  requireValue(equal(document.archive, policy.archive) && equal(document.binary, policy.binary),
    'compiler publication independently selected artifact');
  validateProvenance(document.provenance, document);
  requireValue(sha256(bytes(document.provenance)) === policy.provenanceSha256,
    'compiler publication independently selected provenance');
  validateFiles(document);
  return document;
}
