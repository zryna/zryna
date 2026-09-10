import { mkdirSync, readFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { TextDecoder } from 'node:util';
import { canonical, canonicalBounded, parseCanonical } from './canonical.mjs';
import { validateReproductionText } from './compare-release-builds.mjs';
import {
  copyReleaseFile, createReleaseFile, exactReleaseDirectories, exactReleaseNames, hashReleaseFile,
  MAX_RELEASE_DOCUMENT,
  openReleaseFile, closeReleaseFile, readReleaseFile,
} from './release-files.mjs';
import { validatePreassemblyGatesShape } from './validate-preassembly-gates.mjs';
import { validateReleaseTagReceiptText } from './validate-release-tag-receipt.mjs';
import { validateRelease } from './validate.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const VERSION = '0.2.0';
const TARGETS = Object.freeze([
  { key: 'windows', triple: 'x86_64-pc-windows-msvc', extension: 'zip' },
  { key: 'linux', triple: 'x86_64-unknown-linux-gnu', extension: 'tar.gz' },
]);
const ISSUER = 'https://token.actions.githubusercontent.com';
const ENVELOPE = 'zryna-release-envelope-v1.json';

function reject(message) {
  throw new Error(`R406-EVIDENCE: ${message}`);
}

function utf8(bytes, label) {
  try {
    return new TextDecoder('utf-8', { fatal: true }).decode(bytes);
  } catch {
    reject(`${label} is not UTF-8`);
  }
}

function descriptor(root, name, maximum) {
  const file = openReleaseFile(root, name, maximum);
  try {
    return hashReleaseFile(file);
  } finally {
    closeReleaseFile(file);
  }
}

function loadReproductions(inputRoot) {
  if (!isAbsolute(inputRoot) || resolve(inputRoot) !== inputRoot) {
    reject('input root must be absolute and normalized');
  }
  exactReleaseDirectories(inputRoot, TARGETS.map(({ key }) => key));
  const results = [];
  for (const target of TARGETS) {
    const root = join(inputRoot, target.key);
    const reproduction = validateReproductionText(
      utf8(readReleaseFile(root, 'reproduction.json', MAX_RELEASE_DOCUMENT), 'reproduction result'),
      target.triple,
    );
    const names = ['reproduction.json', ...Object.values(reproduction.artifacts).map(({ path }) => path)];
    exactReleaseNames(root, names);
    for (const artifact of Object.values(reproduction.artifacts)) {
      if (canonical(descriptor(root, artifact.path)) !== canonical(artifact)) {
        reject(`${artifact.path} differs from its reproduction descriptor`);
      }
    }
    results.push({ ...target, root, reproduction });
  }
  const identity = ({ target, artifacts, builds, comparison, ...value }) => value;
  if (canonical(identity(results[0].reproduction)) !== canonical(identity(results[1].reproduction))) {
    reject('platform reproduction identities differ');
  }
  return results;
}

function loadAdmission(admissionRoot, reproductions) {
  if (!isAbsolute(admissionRoot) || resolve(admissionRoot) !== admissionRoot) {
    reject('admission root must be absolute and normalized');
  }
  exactReleaseNames(admissionRoot, ['preassembly-gates.json', 'tag-receipt.json']);
  const tag = validateReleaseTagReceiptText(utf8(
    readReleaseFile(admissionRoot, 'tag-receipt.json', MAX_RELEASE_DOCUMENT), 'tag receipt',
  ));
  const gates = validatePreassemblyGatesShape(parseCanonical(utf8(
    readReleaseFile(admissionRoot, 'preassembly-gates.json', MAX_RELEASE_DOCUMENT), 'gate receipt',
  )));
  const expectedSource = { ...tag.source };
  delete expectedSource.tagType;
  if (tag.version !== VERSION
    || canonical(expectedSource) !== canonical(reproductions[0].reproduction.source)
    || gates.sourceCommit !== tag.source.commit
    || gates.runUrl !== `https://github.com/zryna/zryna/actions/runs/${gates.runId}`) {
    reject('admission and reproduction identities differ');
  }
  return { tag, gates };
}

function decodeBase64(value) {
  if (typeof value !== 'string' || value.length < 4 || value.length > MAX_RELEASE_DOCUMENT * 2
    || !/^(?:[A-Za-z0-9+/]{4})*(?:[A-Za-z0-9+/]{2}==|[A-Za-z0-9+/]{3}=)?$/.test(value)) {
    reject('attestation payload is not strict bounded base64');
  }
  const result = Buffer.from(value, 'base64');
  if (result.toString('base64') !== value) reject('attestation payload base64 is noncanonical');
  return result;
}

export function statementFromAttestation(bundleBytes, expected) {
  const text = utf8(bundleBytes, 'attestation bundle');
  const lines = text.endsWith('\n') ? text.slice(0, -1).split('\n') : text.split('\n');
  if (lines.length !== 1 || lines[0].length === 0) reject('expected one attestation bundle record');
  let bundle;
  try {
    bundle = JSON.parse(lines[0]);
  } catch {
    reject('attestation bundle is not JSON');
  }
  const payload = decodeBase64(bundle?.dsseEnvelope?.payload);
  let statement;
  try {
    statement = JSON.parse(utf8(payload, 'attestation statement'));
  } catch {
    reject('attestation statement is not JSON');
  }
  if (statement?._type !== 'https://in-toto.io/Statement/v1'
    || !Array.isArray(statement.subject) || statement.subject.length !== 1
    || statement.subject[0]?.name !== expected.path
    || canonical(statement.subject[0]?.digest) !== canonical({ sha256: expected.sha256 })) {
    reject('attestation subject differs from the reproduced archive');
  }
  return payload;
}

function subjectNames(reproductions) {
  return reproductions.flatMap(({ reproduction }) => Object.values(reproduction.artifacts)
    .map(({ path }) => path)).sort();
}

function documentNames(reproductions) {
  return [
    ...subjectNames(reproductions),
    ...reproductions.flatMap(({ triple }) => {
      const base = `zryna-${VERSION}-${triple}`;
      return [`${base}.attestation.sigstore.json`, `${base}.intoto.jsonl`];
    }),
    'RELEASE_NOTES.md', 'REVOCATION.md', 'SHA256SUMS',
  ].sort();
}

function signedDocumentNames(reproductions) {
  return [...documentNames(reproductions), 'RELEASE_NOTES.md.sigstore.json',
    'SHA256SUMS.sigstore.json'].sort();
}

function requireWorkflowEnvironment(environment, source) {
  if (environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== source.ref
    || environment.GITHUB_SHA !== source.commit
    || environment.GITHUB_WORKFLOW_SHA !== source.commit
    || !/^[1-9][0-9]{0,19}$/.test(environment.GITHUB_RUN_ID ?? '')
    || !/^(?:[1-9]|[1-9][0-9]|100)$/.test(environment.GITHUB_RUN_ATTEMPT ?? '')) {
    reject('exact release workflow environment is required');
  }
  return {
    repository: 'zryna/zryna',
    path: '.github/workflows/release.yml',
    commit: source.commit,
    runId: environment.GITHUB_RUN_ID,
    runAttempt: Number(environment.GITHUB_RUN_ATTEMPT),
    environment: 'binary-release',
  };
}

function writeSubjects(reproductions, outputRoot) {
  mkdirSync(outputRoot, { recursive: false, mode: 0o700 });
  for (const { root, reproduction } of reproductions) {
    for (const artifact of Object.values(reproduction.artifacts)) {
      copyReleaseFile(root, outputRoot, artifact.path);
    }
  }
  exactReleaseNames(outputRoot, subjectNames(reproductions));
}

function releaseNotes(source, reproductions) {
  const archives = reproductions.map(({ reproduction }) => reproduction.artifacts.archive);
  return Buffer.from([
    '# Zryna 0.2.0 beta',
    '',
    'This prerelease contains the first reproducible downloadable Zryna compiler archives.',
    '',
    `Source: ${source.repository}/tree/${source.commit}`,
    `Tag: v${VERSION} (${source.tagObject})`,
    `Source epoch: ${source.sourceDateEpoch}`,
    '',
    'Archives:',
    ...archives.map(({ path, sha256 }) => `- ${path}: sha256:${sha256}`),
    '',
    'Verify the signed release envelope, checksum document, provenance, SBOM, and archive inventory before installation.',
    '',
  ].join('\n'), 'utf8');
}

function revocationPolicy() {
  return Buffer.from([
    '# Revocation and replacement',
    '',
    'A defective or compromised release is preserved and marked affected. Its tag, assets, attestations, and audit history are not replaced or deleted.',
    '',
    'Use a new version and annotated tag for every correction, then stop recommending the affected release.',
    '',
  ].join('\n'), 'utf8');
}

function writeDocuments(reproductions, outputRoot, environment) {
  exactReleaseNames(outputRoot, subjectNames(reproductions));
  for (const entry of reproductions) {
    const archive = entry.reproduction.artifacts.archive;
    const sourcePath = environment[entry.key === 'linux'
      ? 'ZRYNA_LINUX_ATTESTATION' : 'ZRYNA_WINDOWS_ATTESTATION'];
    if (!isAbsolute(sourcePath ?? '') || resolve(sourcePath) !== sourcePath) {
      reject(`${entry.key} attestation path must be absolute and normalized`);
    }
    const bundleName = `zryna-${VERSION}-${entry.triple}.attestation.sigstore.json`;
    // The action controls its temporary filename; retain and bind those exact bytes under the
    // closed public name without accepting any sibling files from its temporary directory.
    const bundle = readReleaseFile(dirname(sourcePath), basename(sourcePath), MAX_RELEASE_DOCUMENT);
    createReleaseFile(outputRoot, bundleName, bundle);
    const statement = statementFromAttestation(bundle, archive);
    createReleaseFile(outputRoot, `zryna-${VERSION}-${entry.triple}.intoto.jsonl`,
      Buffer.concat([statement, Buffer.from('\n')]));
  }
  const source = reproductions[0].reproduction.source;
  createReleaseFile(outputRoot, 'RELEASE_NOTES.md', releaseNotes(source, reproductions));
  createReleaseFile(outputRoot, 'REVOCATION.md', revocationPolicy());
  const names = documentNames(reproductions).filter((name) => name !== 'SHA256SUMS');
  const entries = names.map((name) => descriptor(outputRoot, name));
  const checksumBytes = Buffer.from(entries.map(({ path, sha256 }) => `${sha256}  ${path}\n`).join(''));
  createReleaseFile(outputRoot, 'SHA256SUMS', checksumBytes);
  exactReleaseNames(outputRoot, documentNames(reproductions));
}

function writeEnvelope(reproductions, admission, outputRoot, environment) {
  exactReleaseNames(outputRoot, signedDocumentNames(reproductions));
  const source = admission.tag.source;
  const workflow = requireWorkflowEnvironment(environment, source);
  workflow.requiredJobs = admission.gates.requiredJobs.map(({ name, conclusion, sourceCommit }) => ({
    name, conclusion, sourceCommit,
  }));
  const subjects = reproductions.map((entry) => {
    const prefix = `zryna-${VERSION}-${entry.triple}`;
    const artifact = (name) => descriptor(outputRoot, name);
    return {
      target: entry.triple,
      archive: artifact(entry.reproduction.artifacts.archive.path),
      buildReceipt: artifact(entry.reproduction.artifacts.buildReceipt.path),
      sbom: artifact(entry.reproduction.artifacts.sbom.path),
      provenance: artifact(`${prefix}.intoto.jsonl`),
      attestation: artifact(`${prefix}.attestation.sigstore.json`),
      platformBaseline: entry.triple === 'x86_64-pc-windows-msvc'
        ? { os: 'windows', product: 'windows-server', version: '2022', architecture: 'x86_64',
          runtime: 'operating-system-ucrt' }
        : { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' },
      reproduction: {
        firstBuildSha256: entry.reproduction.artifacts.archive.sha256,
        secondBuildSha256: entry.reproduction.artifacts.archive.sha256,
        comparison: 'byte-identical',
      },
    };
  });
  const checksumEntries = signedDocumentNames(reproductions)
    .filter((name) => !['SHA256SUMS', 'SHA256SUMS.sigstore.json',
      'RELEASE_NOTES.md.sigstore.json'].includes(name))
    .map((name) => descriptor(outputRoot, name));
  const expectedChecksums = Buffer.from(checksumEntries
    .map(({ path, sha256 }) => `${sha256}  ${path}\n`).join(''));
  if (!readReleaseFile(outputRoot, 'SHA256SUMS', MAX_RELEASE_DOCUMENT).equals(expectedChecksums)) {
    reject('checksum document differs from the complete core inventory');
  }
  const envelope = {
    format: 'zryna.distribution-release.v1',
    version: VERSION,
    tag: `v${VERSION}`,
    channel: 'beta',
    publication: 'prerelease',
    source,
    workflow,
    subjects,
    releaseNotes: descriptor(outputRoot, 'RELEASE_NOTES.md'),
    checksums: {
      document: descriptor(outputRoot, 'SHA256SUMS'),
      signature: descriptor(outputRoot, 'SHA256SUMS.sigstore.json'),
      entries: checksumEntries,
    },
    signing: {
      issuer: ISSUER,
      certificateIdentity: `https://github.com/zryna/zryna/.github/workflows/release.yml@${source.ref}`,
      releaseNotesSignature: descriptor(outputRoot, 'RELEASE_NOTES.md.sigstore.json'),
      envelopeSignaturePath: 'zryna-release-envelope-v1.sigstore.json',
    },
    revocation: {
      policy: descriptor(outputRoot, 'REVOCATION.md'),
      mode: 'preserve-and-mark-affected',
      replacement: 'new-version-and-tag',
    },
    assetAllowlist: [...signedDocumentNames(reproductions), ENVELOPE,
      'zryna-release-envelope-v1.sigstore.json'].sort(),
  };
  validateRelease(envelope);
  createReleaseFile(outputRoot, ENVELOPE, Buffer.from(`${canonicalBounded(envelope)}\n`));
}

export function prepareReleaseEvidence({
  inputRoot, admissionRoot, outputRoot, phase, environment = process.env,
}) {
  if (![inputRoot, admissionRoot, outputRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || !['subjects', 'documents', 'envelope'].includes(phase)) {
    reject('exact absolute roots and a supported phase are required');
  }
  const reproductions = loadReproductions(inputRoot);
  const admission = loadAdmission(admissionRoot, reproductions);
  if (phase === 'subjects') writeSubjects(reproductions, outputRoot);
  else if (phase === 'documents') writeDocuments(reproductions, outputRoot, environment);
  else writeEnvelope(reproductions, admission, outputRoot, environment);
}

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!['--input', '--admission', '--output', '--phase'].includes(argv[index])
      || argv[index + 1] === undefined || values.has(argv[index])) {
      reject('expected --input, --admission, --output, and --phase once');
    }
    values.set(argv[index], argv[index + 1]);
  }
  if (values.size !== 4) reject('expected --input, --admission, --output, and --phase once');
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    prepareReleaseEvidence({
      inputRoot: resolve(values.get('--input')),
      admissionRoot: resolve(values.get('--admission')),
      outputRoot: resolve(values.get('--output')),
      phase: values.get('--phase'),
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
