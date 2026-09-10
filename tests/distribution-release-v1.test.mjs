import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { spawnSync } from 'node:child_process';
import test from 'node:test';
import {
  canonical,
  canonicalBounded,
  MAX_ENVELOPE_BYTES,
  MAX_ENVELOPE_CONTAINERS,
  MAX_ENVELOPE_DEPTH,
  MAX_ENVELOPE_VALUES,
  parseCanonical,
} from '../scripts/distribution-release/canonical.mjs';
import { readEnvelopeFile } from '../scripts/distribution-release/input.mjs';
import { validateRelease, validateReleaseText } from '../scripts/distribution-release/validate.mjs';

const digest = (index) => index.toString(16).padStart(64, '0');
const artifact = (path, index) => ({ path, size: 100 + index, sha256: digest(index) });
const VERSION = '0.2.0';
const TAG = `v${VERSION}`;
const COMMIT = 'a'.repeat(40);
const compare = (left, right) => left < right ? -1 : left > right ? 1 : 0;

function subject(target, offset) {
  const extension = target === 'x86_64-pc-windows-msvc' ? 'zip' : 'tar.gz';
  const archive = artifact(`zryna-${VERSION}-${target}.${extension}`, offset);
  const prefix = `zryna-${VERSION}-${target}`;
  return {
    target,
    archive,
    buildReceipt: artifact(`${prefix}.build-receipt.json`, offset + 1),
    sbom: artifact(`${prefix}.spdx.json`, offset + 2),
    provenance: artifact(`${prefix}.intoto.jsonl`, offset + 3),
    attestation: artifact(`${prefix}.attestation.sigstore.json`, offset + 4),
    platformBaseline: target === 'x86_64-pc-windows-msvc'
      ? {
          os: 'windows', product: 'windows-server', version: '2022',
          architecture: 'x86_64', runtime: 'operating-system-ucrt',
        }
      : {
          os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64',
        },
    reproduction: {
      firstBuildSha256: archive.sha256,
      secondBuildSha256: archive.sha256,
      comparison: 'byte-identical',
    },
  };
}

function fixture() {
  const document = {
    format: 'zryna.distribution-release.v1',
    version: VERSION,
    tag: TAG,
    channel: 'beta',
    publication: 'prerelease',
    source: {
      repository: 'https://github.com/zryna/zryna',
      ref: `refs/tags/${TAG}`,
      tagType: 'annotated',
      tagObject: 'c'.repeat(40),
      commit: COMMIT,
      tree: 'b'.repeat(40),
      sourceDateEpoch: 1_789_081_200,
    },
    workflow: {
      repository: 'zryna/zryna',
      path: '.github/workflows/release.yml',
      commit: COMMIT,
      runId: '123456789',
      runAttempt: 1,
      environment: 'binary-release',
      requiredJobs: [
        { name: 'adapter', conclusion: 'success', sourceCommit: COMMIT },
        { name: 'm0', conclusion: 'success', sourceCommit: COMMIT },
        { name: 'm2', conclusion: 'success', sourceCommit: COMMIT },
        { name: 'm3', conclusion: 'success', sourceCommit: COMMIT },
        { name: 'rust (ubuntu-latest)', conclusion: 'success', sourceCommit: COMMIT },
        { name: 'rust (windows-latest)', conclusion: 'success', sourceCommit: COMMIT },
      ],
    },
    subjects: [
      subject('x86_64-pc-windows-msvc', 1),
      subject('x86_64-unknown-linux-gnu', 10),
    ],
    releaseNotes: artifact('RELEASE_NOTES.md', 20),
    checksums: {
      document: artifact('SHA256SUMS', 21),
      signature: artifact('SHA256SUMS.sigstore.json', 22),
      entries: [],
    },
    signing: {
      issuer: 'https://token.actions.githubusercontent.com',
      certificateIdentity:
        `https://github.com/zryna/zryna/.github/workflows/release.yml@refs/tags/${TAG}`,
      releaseNotesSignature: artifact('RELEASE_NOTES.md.sigstore.json', 23),
      envelopeSignaturePath: 'zryna-release-envelope-v1.sigstore.json',
    },
    revocation: {
      policy: artifact('REVOCATION.md', 24),
      mode: 'preserve-and-mark-affected',
      replacement: 'new-version-and-tag',
    },
    assetAllowlist: [],
  };
  const core = [
    ...document.subjects.flatMap((entry) => [
      entry.archive, entry.buildReceipt, entry.sbom, entry.provenance, entry.attestation,
    ]),
    document.releaseNotes,
    document.revocation.policy,
  ];
  document.checksums.entries = structuredClone(core).sort((left, right) =>
    compare(left.path, right.path));
  document.assetAllowlist = [
    'zryna-release-envelope-v1.json',
    document.signing.envelopeSignaturePath,
    ...core.map(({ path }) => path),
    document.checksums.document.path,
    document.checksums.signature.path,
    document.signing.releaseNotesSignature.path,
  ].sort(compare);
  return document;
}

test('accepts one canonical exact-identity Windows and Linux envelope', () => {
  const value = fixture();
  assert.equal(validateRelease(value), value);
  const wire = `${canonical(value)}\n`;
  assert.deepEqual(validateReleaseText(wire), value);
  assert.deepEqual(parseCanonical(wire), value);
});

test('rejects drift at source, workflow, signing, target, and platform boundaries', () => {
  for (const [code, mutate] of [
    ['R406-SOURCE', (value) => { value.tag = 'v0.2.1'; }],
    ['R406-SOURCE', (value) => { value.source.ref = 'refs/tags/v0.2.1'; }],
    ['R406-SOURCE', (value) => { value.source.tagObject = value.source.commit; }],
    ['R406-WORKFLOW', (value) => { value.workflow.commit = 'c'.repeat(40); }],
    ['R406-WORKFLOW', (value) => { value.workflow.requiredJobs[0].sourceCommit = 'c'.repeat(40); }],
    ['R406-SIGNATURE', (value) => { value.signing.certificateIdentity = value.signing.certificateIdentity.replace(TAG, 'v0.2.1'); }],
    ['R406-TARGETS', (value) => { value.subjects.reverse(); }],
    ['R406-TARGETS', (value) => { value.subjects[0].target = 'x86_64-unknown-linux-gnu'; }],
    ['R406-PLATFORM', (value) => { value.subjects[0].platformBaseline = structuredClone(value.subjects[1].platformBaseline); }],
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateRelease(value), new RegExp(`^${code}:`));
  }
});

test('rejects missing protected checks and version-disconnected asset names', () => {
  const missingCheck = fixture();
  missingCheck.workflow.requiredJobs.pop();
  assert.throws(() => validateRelease(missingCheck), /^R406-WORKFLOW:/);

  const archive = fixture();
  archive.subjects[1].archive.path = archive.subjects[1].archive.path.replace(VERSION, '0.2.1');
  archive.assetAllowlist = archive.assetAllowlist
    .filter((path) => path !== `zryna-${VERSION}-x86_64-unknown-linux-gnu.tar.gz`);
  archive.assetAllowlist.push(archive.subjects[1].archive.path);
  archive.assetAllowlist.sort(compare);
  archive.checksums.entries.find((entry) => entry.sha256 === archive.subjects[1].archive.sha256).path
    = archive.subjects[1].archive.path;
  archive.checksums.entries.sort((left, right) => compare(left.path, right.path));
  assert.throws(() => validateRelease(archive), /^R406-ASSETS:/);

  const global = fixture();
  global.releaseNotes.path = 'NOTES.md';
  assert.throws(() => validateRelease(global), /^R406-ASSETS:/);
});

test('rejects non-identical reproduction and incomplete or forged checksum coverage', () => {
  const mismatch = fixture();
  mismatch.subjects[1].reproduction.secondBuildSha256 = digest(60);
  assert.throws(() => validateRelease(mismatch), /^R406-REPRODUCTION:/);

  const missing = fixture();
  missing.checksums.entries.pop();
  assert.throws(() => validateRelease(missing), /^R406-CHECKSUMS:/);

  const forged = fixture();
  forged.checksums.entries[0].size += 1;
  assert.throws(() => validateRelease(forged), /^R406-CHECKSUMS:/);

  const signatureCycle = fixture();
  signatureCycle.checksums.entries.push(structuredClone(signatureCycle.checksums.signature));
  signatureCycle.checksums.entries.sort((left, right) => compare(left.path, right.path));
  assert.throws(() => validateRelease(signatureCycle), /^R406-CHECKSUMS:/);

  const missingAttestation = fixture();
  missingAttestation.checksums.entries = missingAttestation.checksums.entries
    .filter(({ path }) => path !== missingAttestation.subjects[0].attestation.path);
  assert.throws(() => validateRelease(missingAttestation), /^R406-CHECKSUMS:/);

  const forgedAttestation = fixture();
  forgedAttestation.checksums.entries
    .find(({ path }) => path === forgedAttestation.subjects[0].attestation.path).sha256 = digest(61);
  assert.throws(() => validateRelease(forgedAttestation), /^R406-CHECKSUMS:/);
});

test('rejects duplicate, extra, missing, and unsorted public assets', () => {
  const duplicate = fixture();
  duplicate.releaseNotes.path = duplicate.subjects[0].archive.path;
  assert.throws(() => validateRelease(duplicate), /^R406-ASSETS:/);

  const extra = fixture();
  extra.assetAllowlist.push('undeclared.exe');
  extra.assetAllowlist.sort(compare);
  assert.throws(() => validateRelease(extra), /^R406-ASSETS:/);

  const missing = fixture();
  missing.assetAllowlist.shift();
  assert.throws(() => validateRelease(missing), /^R406-ASSETS:/);

  const order = fixture();
  order.assetAllowlist.reverse();
  assert.throws(() => validateRelease(order), /^R406-ORDER:/);
});

test('schema rejects false success, qualified equivalence, paths, and extra fields', () => {
  for (const mutate of [
    (value) => { value.workflow.requiredJobs[0].conclusion = 'skipped'; },
    (value) => { value.subjects[0].reproduction.comparison = 'normalized'; },
    (value) => { value.subjects[0].archive.path = '../escape.zip'; },
    (value) => { value.source.privatePath = 'C:\\private'; },
    (value) => { value.workflow.runId = '0'; },
    (value) => { value.signing.envelopeSignaturePath = 'other.sigstore.json'; },
    (value) => { value.channel = 'stable'; },
    (value) => { value.publication = 'release'; },
    (value) => { value.version = '0.2.0-beta.1'; },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateRelease(value), /^R406-SCHEMA:/);
  }
});

test('wire parser rejects whitespace and non-canonical object order', () => {
  const value = fixture();
  assert.throws(() => validateReleaseText(JSON.stringify(value)), /^R406-CANONICAL:/);
  assert.throws(() => validateReleaseText(`${canonical(value)}\n\n`), /^R406-CANONICAL:/);
  assert.throws(() => validateReleaseText('{"broken":]\n'),
    /^R406-CANONICAL: document is not valid JSON$/);
});

test('wire limits are inclusive and reject the first extra byte, depth, and container', () => {
  const exactBytes = `"${'x'.repeat(MAX_ENVELOPE_BYTES - 3)}"\n`;
  assert.equal(Buffer.byteLength(exactBytes), MAX_ENVELOPE_BYTES);
  assert.equal(parseCanonical(exactBytes).length, MAX_ENVELOPE_BYTES - 3);
  assert.throws(() => parseCanonical(`"${'x'.repeat(MAX_ENVELOPE_BYTES - 2)}"\n`),
    /^R406-RESOURCE: envelope exceeds 262144 bytes$/);

  const exactDepth = `${'['.repeat(MAX_ENVELOPE_DEPTH)}null${']'.repeat(MAX_ENVELOPE_DEPTH)}\n`;
  assert.doesNotThrow(() => parseCanonical(exactDepth));
  const extraDepth = `${'['.repeat(MAX_ENVELOPE_DEPTH + 1)}null${']'.repeat(MAX_ENVELOPE_DEPTH + 1)}\n`;
  assert.throws(() => parseCanonical(extraDepth), /^R406-RESOURCE: envelope exceeds depth 32$/);

  const exactContainers = `${canonical(Array.from({ length: MAX_ENVELOPE_CONTAINERS - 1 }, () => []))}\n`;
  assert.doesNotThrow(() => parseCanonical(exactContainers));
  const extraContainer = `${canonical(Array.from({ length: MAX_ENVELOPE_CONTAINERS }, () => []))}\n`;
  assert.throws(() => parseCanonical(extraContainer),
    /^R406-RESOURCE: envelope exceeds 1024 containers$/);
});

test('in-memory API rejects first-extra values and cycles before canonical recursion', () => {
  const exact = { values: Array.from({ length: MAX_ENVELOPE_VALUES - 2 }, () => null) };
  assert.throws(() => validateRelease(exact), /^R406-SCHEMA:/);
  const extra = { values: Array.from({ length: MAX_ENVELOPE_VALUES - 1 }, () => null) };
  assert.throws(() => validateRelease(extra), /^R406-RESOURCE: envelope exceeds 4096 values$/);
  const cyclic = {};
  cyclic.self = cyclic;
  assert.throws(() => validateRelease(cyclic), /^R406-RESOURCE: envelope contains a cycle$/);

  const lastString = { value: 'x'.repeat(MAX_ENVELOPE_BYTES) };
  assert.throws(() => validateRelease(lastString),
    /^R406-RESOURCE: envelope strings exceed 262144 bytes$/);

  const escapedCount = Math.floor((MAX_ENVELOPE_BYTES - 3) / 6);
  const exactEscaped = `${'\0'.repeat(escapedCount)}x`;
  assert.equal(Buffer.byteLength(`${canonical(exactEscaped)}\n`), MAX_ENVELOPE_BYTES);
  assert.doesNotThrow(() => canonicalBounded(exactEscaped));
  assert.throws(() => canonicalBounded(`${exactEscaped}x`),
    /^R406-RESOURCE: canonical envelope exceeds 262144 bytes$/);
});

test('retained reader enforces the inclusive file bound before parsing', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'zryna-release-envelope-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const path = join(directory, 'envelope.json');
  writeFileSync(path, 'x'.repeat(MAX_ENVELOPE_BYTES));
  assert.equal(readEnvelopeFile(path).length, MAX_ENVELOPE_BYTES);
  writeFileSync(path, 'x'.repeat(MAX_ENVELOPE_BYTES + 1));
  assert.throws(() => readEnvelopeFile(path), /^R406-RESOURCE: envelope exceeds 262144 bytes$/);
});

test('retained reader rejects non-regular inputs without blocking', (t) => {
  const directory = mkdtempSync(join(tmpdir(), 'zryna-release-nonregular-'));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const child = join(directory, 'directory');
  mkdirSync(child);
  assert.throws(() => readEnvelopeFile(child),
    /^R406-RESOURCE: envelope must be a direct regular file$/);

  if (process.platform !== 'win32') {
    const fifo = join(directory, 'fifo');
    const created = spawnSync('mkfifo', [fifo], { shell: false, encoding: 'utf8' });
    assert.equal(created.error, undefined);
    assert.equal(created.status, 0, created.stderr);
    assert.throws(() => readEnvelopeFile(fifo),
      /^R406-RESOURCE: envelope must be a direct regular file$/);
  }
});
