import { spawnSync } from 'node:child_process';
import { isAbsolute, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonical } from './canonical.mjs';
import { statementFromAttestation } from './prepare-release-evidence.mjs';
import {
  closeReleaseFile, exactReleaseNames, hashReleaseFile, MAX_RELEASE_DOCUMENT, openReleaseFile,
  readReleaseFile,
} from './release-files.mjs';
import { validateReleaseText } from './validate.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const MAX_VERIFIER_OUTPUT = 1024 * 1024;

function reject(message) {
  throw new Error(`R406-SIGNED-RELEASE: ${message}`);
}

function descriptor(root, name, maximum) {
  const file = openReleaseFile(root, name, maximum);
  try {
    return hashReleaseFile(file);
  } finally {
    closeReleaseFile(file);
  }
}

function artifacts(envelope) {
  return [
    ...envelope.subjects.flatMap((subject) => [
      subject.archive, subject.buildReceipt, subject.sbom, subject.provenance, subject.attestation,
    ]),
    envelope.releaseNotes,
    envelope.checksums.document,
    envelope.checksums.signature,
    envelope.signing.releaseNotesSignature,
    envelope.revocation.policy,
  ];
}

function verifyDescriptor(root, expected) {
  const maximum = expected.path.endsWith('.json') || expected.path.endsWith('.jsonl')
    || expected.path.endsWith('.md') || expected.path === 'SHA256SUMS'
    ? MAX_RELEASE_DOCUMENT : undefined;
  if (canonical(descriptor(root, expected.path, maximum)) !== canonical(expected)) {
    reject(`${expected.path} differs from its signed descriptor`);
  }
}

function invokeCosign(spawn, args) {
  const result = spawn('cosign', args, {
    encoding: 'utf8', maxBuffer: MAX_VERIFIER_OUTPUT + 1, shell: false,
    timeout: 120_000, windowsHide: true,
  });
  const outputBytes = Buffer.byteLength(`${result.stdout ?? ''}${result.stderr ?? ''}`, 'utf8');
  if (result.error || result.status !== 0 || outputBytes > MAX_VERIFIER_OUTPUT) {
    reject(`cosign ${args[0]} failed with status ${result.status ?? 'unknown'}`);
  }
}

export function verifySignedRelease({ directory, spawn = spawnSync }) {
  if (!isAbsolute(directory) || resolve(directory) !== directory) {
    reject('publication directory must be absolute and normalized');
  }
  const envelopeName = 'zryna-release-envelope-v1.json';
  const envelope = validateReleaseText(readReleaseFile(
    directory, envelopeName, MAX_RELEASE_DOCUMENT,
  ).toString('utf8'));
  exactReleaseNames(directory, envelope.assetAllowlist);
  for (const artifact of artifacts(envelope)) verifyDescriptor(directory, artifact);

  const checksumBytes = Buffer.from(envelope.checksums.entries
    .map(({ path, sha256 }) => `${sha256}  ${path}\n`).join(''));
  if (!readReleaseFile(directory, envelope.checksums.document.path,
    MAX_RELEASE_DOCUMENT).equals(checksumBytes)) {
    reject('checksum document differs from the envelope entries');
  }

  for (const subject of envelope.subjects) {
    const bundle = readReleaseFile(directory, subject.attestation.path, MAX_RELEASE_DOCUMENT);
    const statement = statementFromAttestation(bundle, subject.archive);
    const provenance = readReleaseFile(directory, subject.provenance.path, MAX_RELEASE_DOCUMENT);
    if (!provenance.equals(Buffer.concat([statement, Buffer.from('\n')]))) {
      reject(`${subject.target} provenance differs from its attestation payload`);
    }
    invokeCosign(spawn, [
      'verify-blob-attestation',
      '--bundle', resolve(directory, subject.attestation.path),
      '--certificate-identity', envelope.signing.certificateIdentity,
      '--certificate-oidc-issuer', envelope.signing.issuer,
      resolve(directory, subject.archive.path),
    ]);
  }
  for (const [document, signature] of [
    [envelope.checksums.document.path, envelope.checksums.signature.path],
    [envelope.releaseNotes.path, envelope.signing.releaseNotesSignature.path],
    [envelopeName, envelope.signing.envelopeSignaturePath],
  ]) {
    invokeCosign(spawn, [
      'verify-blob',
      '--bundle', resolve(directory, signature),
      '--certificate-identity', envelope.signing.certificateIdentity,
      '--certificate-oidc-issuer', envelope.signing.issuer,
      resolve(directory, document),
    ]);
  }
  return envelope;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 4 || process.argv[2] !== '--directory') {
      reject('expected --directory once');
    }
    verifySignedRelease({ directory: resolve(process.argv[3]) });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
