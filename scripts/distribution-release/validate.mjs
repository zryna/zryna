import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import Ajv from 'ajv';
import { canonical, canonicalBounded, parseCanonical } from './canonical.mjs';
import { readEnvelopeFile } from './input.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const SCHEMA_PATH = fileURLToPath(new URL('../../schemas/zryna-distribution-release-v1.schema.json', import.meta.url));
const ajv = new Ajv({ allErrors: true, strict: true });
const validateSchema = ajv.compile(JSON.parse(readFileSync(SCHEMA_PATH, 'utf8')));
const REQUIRED_JOBS = Object.freeze([
  'adapter',
  'm0',
  'm2',
  'm3',
  'rust (ubuntu-latest)',
  'rust (windows-latest)',
]);

function reject(code, message) {
  throw new Error(`${code}: ${message}`);
}

function strictlySorted(values, project = (value) => value) {
  for (let index = 1; index < values.length; index += 1) {
    if (project(values[index - 1]) >= project(values[index])) return false;
  }
  return true;
}

function artifacts(document) {
  return [
    ...document.subjects.flatMap((subject) => [
      subject.archive,
      subject.buildReceipt,
      subject.sbom,
      subject.provenance,
      subject.attestation,
    ]),
    document.releaseNotes,
    document.checksums.document,
    document.checksums.signature,
    document.signing.releaseNotesSignature,
    document.revocation.policy,
  ];
}

export function validateRelease(document) {
  canonicalBounded(document);
  if (!validateSchema(document)) {
    reject('R406-SCHEMA', ajv.errorsText(validateSchema.errors, { separator: '; ' }));
  }
  if (document.tag !== `v${document.version}` || document.source.ref !== `refs/tags/${document.tag}`) {
    reject('R406-SOURCE', 'version, tag, and source ref differ');
  }
  if (document.source.tagObject === document.source.commit) {
    reject('R406-SOURCE', 'annotated tag object must differ from its peeled commit');
  }
  if (document.workflow.commit !== document.source.commit) {
    reject('R406-WORKFLOW', 'workflow and source commits differ');
  }
  const expectedIdentity = `https://github.com/zryna/zryna/${document.workflow.path}@${document.source.ref}`;
  if (document.signing.certificateIdentity !== expectedIdentity) {
    reject('R406-SIGNATURE', 'certificate identity does not bind the exact workflow and tag');
  }
  const targets = document.subjects.map(({ target }) => target);
  if (!strictlySorted(targets)
    || targets.join(',') !== 'x86_64-pc-windows-msvc,x86_64-unknown-linux-gnu') {
    reject('R406-TARGETS', 'exact sorted Windows and Linux subjects are required');
  }
  for (const subject of document.subjects) {
    const expectedOs = subject.target === 'x86_64-pc-windows-msvc' ? 'windows' : 'linux';
    if (subject.platformBaseline.os !== expectedOs) {
      reject('R406-PLATFORM', `${subject.target} has the wrong platform baseline`);
    }
    if (subject.reproduction.firstBuildSha256 !== subject.archive.sha256
      || subject.reproduction.secondBuildSha256 !== subject.archive.sha256) {
      reject('R406-REPRODUCTION', `${subject.target} builds are not byte-identical to the subject`);
    }
  }
  if (!strictlySorted(document.workflow.requiredJobs, ({ name }) => name)) {
    reject('R406-ORDER', 'required jobs must be unique and sorted by name');
  }
  if (document.workflow.requiredJobs.some(({ sourceCommit }) => sourceCommit !== document.source.commit)) {
    reject('R406-WORKFLOW', 'required job belongs to another source commit');
  }
  const jobNames = new Set(document.workflow.requiredJobs.map(({ name }) => name));
  if (REQUIRED_JOBS.some((name) => !jobNames.has(name))) {
    reject('R406-WORKFLOW', 'current protected branch checks are incomplete');
  }
  for (const subject of document.subjects) {
    const base = `zryna-${document.version}-${subject.target}`;
    const expectedPaths = [
      `${base}.${subject.target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz'}`,
      `${base}.build-receipt.json`,
      `${base}.spdx.json`,
      `${base}.intoto.jsonl`,
      `${base}.attestation.sigstore.json`,
    ];
    const observedPaths = [
      subject.archive.path,
      subject.buildReceipt.path,
      subject.sbom.path,
      subject.provenance.path,
      subject.attestation.path,
    ];
    if (canonical(observedPaths) !== canonical(expectedPaths)) {
      reject('R406-ASSETS', `${subject.target} asset names differ from the closed convention`);
    }
  }
  const fixedPaths = [
    [document.releaseNotes.path, 'RELEASE_NOTES.md'],
    [document.checksums.document.path, 'SHA256SUMS'],
    [document.checksums.signature.path, 'SHA256SUMS.sigstore.json'],
    [document.signing.releaseNotesSignature.path, 'RELEASE_NOTES.md.sigstore.json'],
    [document.revocation.policy.path, 'REVOCATION.md'],
  ];
  if (fixedPaths.some(([observed, expected]) => observed !== expected)) {
    reject('R406-ASSETS', 'release-level asset names differ from the closed convention');
  }
  const allArtifacts = artifacts(document);
  const paths = allArtifacts.map(({ path }) => path);
  if (new Set(paths).size !== paths.length) reject('R406-ASSETS', 'artifact paths must be unique');
  if (!strictlySorted(document.checksums.entries, ({ path }) => path)) {
    reject('R406-ORDER', 'checksum entries must be unique and sorted by path');
  }
  const signedPaths = new Set(document.checksums.entries.map(({ path }) => path));
  const excludedFromChecksums = new Set([
    document.checksums.document.path,
    document.checksums.signature.path,
    document.signing.releaseNotesSignature.path,
  ]);
  const expectedSigned = allArtifacts
    .map(({ path }) => path)
    .filter((path) => !excludedFromChecksums.has(path));
  if (expectedSigned.some((path) => !signedPaths.has(path))
    || signedPaths.size !== expectedSigned.length) {
    reject('R406-CHECKSUMS', 'checksum entries must cover each non-signature core asset exactly');
  }
  for (const entry of document.checksums.entries) {
    const artifact = allArtifacts.find(({ path }) => path === entry.path);
    if (!artifact || canonical(entry) !== canonical(artifact)) {
      reject('R406-CHECKSUMS', `checksum entry does not match ${entry.path}`);
    }
  }
  if (!strictlySorted(document.assetAllowlist)) {
    reject('R406-ORDER', 'asset allowlist must be unique and sorted');
  }
  const expectedAllowlist = [...new Set([
    'zryna-release-envelope-v1.json',
    document.signing.envelopeSignaturePath,
    ...paths,
  ])].sort();
  if (canonical(document.assetAllowlist) !== canonical(expectedAllowlist)) {
    reject('R406-ASSETS', 'asset allowlist differs from the closed release inventory');
  }
  return document;
}

export function validateReleaseText(text) {
  return validateRelease(parseCanonical(text));
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    if (process.argv.length !== 3) reject('R406-USAGE', 'expected one release envelope path');
    validateReleaseText(readEnvelopeFile(resolve(process.argv[2])));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
