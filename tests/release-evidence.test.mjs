import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  canonicalizeEnvelopeSignature,
} from '../scripts/distribution-release/canonicalize-envelope-signature.mjs';
import { prepareReleaseEvidence } from '../scripts/distribution-release/prepare-release-evidence.mjs';
import { publishDraftRelease } from '../scripts/distribution-release/publish-draft-release.mjs';
import { releaseSpdxBytes } from '../scripts/distribution-release/release-sbom.mjs';
import { verifySignedRelease } from '../scripts/distribution-release/verify-signed-release.mjs';
import { githubServer } from './release-publisher-mock.mjs';

const VERSION = '0.2.2';
const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const TAG_OBJECT = 'a'.repeat(40);
const RECIPE = 'd'.repeat(64);
const TARGETS = [
  { key: 'windows', triple: 'x86_64-pc-windows-msvc', extension: 'zip' },
  { key: 'linux', triple: 'x86_64-unknown-linux-gnu', extension: 'tar.gz' },
];
const CANDIDATE_JOBS = [
  'admit protected production candidate',
  'accept installed candidate x86_64-pc-windows-msvc',
  'accept installed candidate x86_64-unknown-linux-gnu',
  'build candidate x86_64-pc-windows-msvc replica 1',
  'build candidate x86_64-pc-windows-msvc replica 2',
  'build candidate x86_64-unknown-linux-gnu replica 1',
  'build candidate x86_64-unknown-linux-gnu replica 2',
  'reproduce candidate x86_64-pc-windows-msvc',
  'reproduce candidate x86_64-unknown-linux-gnu',
];

function source(tagType = false) {
  return {
    repository: 'https://github.com/zryna/zryna',
    ref: 'refs/tags/v0.2.2',
    ...(tagType ? { tagType: 'annotated' } : {}),
    tagObject: TAG_OBJECT,
    commit: COMMIT,
    tree: TREE,
    sourceDateEpoch: 1_789_081_200,
  };
}

function writeCanonical(path, value) {
  writeFileSync(path, `${canonicalBounded(value)}\n`);
}

function archiveFiles(target) {
  const cliPath = target === 'x86_64-pc-windows-msvc' ? 'bin/zryna.exe' : 'bin/zryna';
  const indexed = [
    { path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n'), role: 'license',
      material: 'source', licenses: ['LICENSE'] },
    { path: cliPath, mode: 0o755, data: Buffer.from(`${target} cli`), role: 'cli',
      material: 'source', licenses: ['LICENSE'] },
  ].sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0)
    .map((file) => ({ ...file, size: file.data.length, sha256: sha256(file.data) }));
  const inventory = Buffer.from(`${canonicalBounded({
    format: 'zryna.distribution-inventory.v1',
    files: indexed.map(({ data, ...entry }) => entry),
  })}\n`);
  return [
    ...indexed.map(({ role, material, licenses, size, sha256: digest, ...file }) => file),
    { path: 'metadata/inventory.json', mode: 0o644, data: inventory },
    { path: 'metadata/checksums.sha256', mode: 0o644, data: Buffer.from('checksums\n') },
  ].sort((left, right) => left.path < right.path ? -1 : left.path > right.path ? 1 : 0);
}

function fixture(t, mutateStatement = () => {}, mutateSbom = (bytes) => bytes) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-release-evidence-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const inputRoot = join(parent, 'reproduced');
  const admissionRoot = join(parent, 'admission');
  const outputRoot = join(parent, 'publication');
  const attestations = join(parent, 'attestations');
  mkdirSync(inputRoot);
  mkdirSync(admissionRoot);
  mkdirSync(attestations);
  const attestationPaths = {};
  const verifiedFiles = {};
  for (const target of TARGETS) {
    const root = join(inputRoot, target.key);
    mkdirSync(root);
    const base = `zryna-${VERSION}-${target.triple}`;
    const paths = {
      archive: `${base}.${target.extension}`,
      buildReceipt: `${base}.build-receipt.json`,
      sbom: `${base}.spdx.json`,
    };
    const archive = Buffer.from(`${target.triple} archive`);
    const files = archiveFiles(target.triple);
    verifiedFiles[target.triple] = files;
    const bytes = {
      archive,
      buildReceipt: Buffer.from(`{"target":"${target.triple}"}\n`),
      sbom: mutateSbom(releaseSpdxBytes({
        files,
        archive: { filename: paths.archive, size: archive.length, sha256: sha256(archive) },
        source: source(),
        target: target.triple,
      }), { target, archive }),
    };
    const artifacts = Object.fromEntries(Object.entries(paths).map(([key, path]) => [key, {
      path, size: bytes[key].length, sha256: sha256(bytes[key]),
    }]));
    for (const [key, path] of Object.entries(paths)) writeFileSync(join(root, path), bytes[key]);
    writeCanonical(join(root, 'reproduction.json'), {
      format: 'zryna.release-reproduction.v1',
      version: VERSION,
      target: target.triple,
      source: source(),
      recipe: { format: 'zryna.distribution-recipe.v1', sha256: RECIPE },
      artifacts,
      builds: [
        { replica: 1, manifestSha256: sha256(`${target.triple}-first`) },
        { replica: 2, manifestSha256: sha256(`${target.triple}-second`) },
      ],
      comparison: 'byte-identical',
    });
    const run = 'https://github.com/zryna/zryna/actions/runs/987654321/attempts/2';
    const statement = {
      _type: 'https://in-toto.io/Statement/v1',
      subject: [{ name: artifacts.archive.path, digest: { sha256: artifacts.archive.sha256 } }],
      predicateType: 'https://slsa.dev/provenance/v1',
      predicate: {
        buildDefinition: {
          buildType: 'https://actions.github.io/buildtypes/workflow/v1',
          externalParameters: { workflow: {
            path: '.github/workflows/release.yml', ref: 'refs/tags/v0.2.2',
            repository: 'https://github.com/zryna/zryna',
          } },
          internalParameters: { github: {
            event_name: 'push', repository_id: '123', repository_owner_id: '456',
            runner_environment: 'github-hosted',
          } },
          resolvedDependencies: [{
            uri: 'git+https://github.com/zryna/zryna@refs/tags/v0.2.2',
            digest: { gitCommit: COMMIT },
          }],
        },
        runDetails: {
          builder: {
            id: 'https://github.com/zryna/zryna/.github/workflows/release.yml@refs/tags/v0.2.2',
          },
          metadata: { invocationId: run },
        },
      },
    };
    mutateStatement(statement, target);
    const bundle = {
      mediaType: 'application/vnd.dev.sigstore.bundle.v0.3+json',
      dsseEnvelope: { payload: Buffer.from(JSON.stringify(statement)).toString('base64'),
        payloadType: 'application/vnd.in-toto+json', signatures: [{ sig: 'AA==' }] },
    };
    const attestationPath = join(attestations, `${target.key}.jsonl`);
    writeFileSync(attestationPath, `${JSON.stringify(bundle)}\n`);
    attestationPaths[target.key] = resolve(attestationPath);
  }
  writeCanonical(join(admissionRoot, 'tag-receipt.json'), {
    format: 'zryna.release-tag-receipt.v1', version: VERSION, source: source(true),
    workflow: { path: '.github/workflows/release.yml', size: 100, sha256: 'e'.repeat(64) },
  });
  const names = ['adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)'];
  writeCanonical(join(admissionRoot, 'preassembly-gates.json'), {
    format: 'zryna.preassembly-gates.v1', repository: 'https://github.com/zryna/zryna',
    workflow: '.github/workflows/ci.yml', runId: '123456789', runAttempt: 1,
    runUrl: 'https://github.com/zryna/zryna/actions/runs/123456789', sourceCommit: COMMIT,
    requiredContexts: names,
    requiredJobs: names.map((name, index) => ({
      name, conclusion: 'success', sourceCommit: COMMIT, runId: '123456789', runAttempt: 1,
      jobId: String(200 + index), checkRunId: String(300 + index),
    })),
  });
  writeCanonical(join(admissionRoot, 'production-candidate-receipt.json'), {
    format: 'zryna.production-candidate-receipt.v1', status: 'production-candidate-passed',
    productionAdmission: 'candidate-prerequisite-only',
    workflow: '.github/workflows/release-production-candidate.yml', sourceCommit: COMMIT,
    recipeSha256: RECIPE, runId: '876543210', runAttempt: 1,
    runUrl: 'https://github.com/zryna/zryna/actions/runs/876543210',
    requiredJobs: CANDIDATE_JOBS.map((name, index) => ({
      name, conclusion: 'success', jobId: String(500 + index), checkRunId: String(600 + index),
    })),
  });
  return {
    inputRoot: resolve(inputRoot), admissionRoot: resolve(admissionRoot), outputRoot: resolve(outputRoot),
    verifyArchiveImpl: async (archive, expected) => {
      assert.equal(sha256(archive), expected.sha256);
      return { archiveSha256: expected.sha256, files: verifiedFiles[expected.target.triple] };
    },
    environment: {
      GITHUB_TOKEN: 'test-token', GITHUB_SERVER_URL: 'https://github.com',
      GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_REF: 'refs/tags/v0.2.2',
      GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
      GITHUB_RUN_ID: '987654321', GITHUB_RUN_ATTEMPT: '2',
      ZRYNA_LINUX_ATTESTATION: attestationPaths.linux,
      ZRYNA_WINDOWS_ATTESTATION: attestationPaths.windows,
    },
  };
}

function prepareAll(paths) {
  for (const phase of ['subjects', 'documents']) {
    prepareReleaseEvidence({ ...paths, phase });
  }
  writeFileSync(join(paths.outputRoot, 'SHA256SUMS.sigstore.json'), '{}\n');
  writeFileSync(join(paths.outputRoot, 'RELEASE_NOTES.md.sigstore.json'), '{}\n');
  prepareReleaseEvidence({ ...paths, phase: 'envelope' });
  writeFileSync(join(paths.outputRoot,
    'zryna-release-envelope-v1.json.sigstore.json'), '{}\n');
  canonicalizeEnvelopeSignature(paths.outputRoot);
}

function prepareForEnvelopeSignature(paths) {
  for (const phase of ['subjects', 'documents']) {
    prepareReleaseEvidence({ ...paths, phase });
  }
  writeFileSync(join(paths.outputRoot, 'SHA256SUMS.sigstore.json'), '{}\n');
  writeFileSync(join(paths.outputRoot, 'RELEASE_NOTES.md.sigstore.json'), '{}\n');
  prepareReleaseEvidence({ ...paths, phase: 'envelope' });
}

test('canonicalizes the signer envelope bundle before independent verification', async (t) => {
  const paths = fixture(t);
  prepareForEnvelopeSignature(paths);
  const actionPath = join(paths.outputRoot,
    'zryna-release-envelope-v1.json.sigstore.json');
  const canonicalPath = join(paths.outputRoot, 'zryna-release-envelope-v1.sigstore.json');
  const signature = Buffer.from('{"mediaType":"application/vnd.dev.sigstore.bundle.v0.3+json"}\n');
  writeFileSync(actionPath, signature);
  const result = spawnSync(process.execPath, [
    resolve(import.meta.dirname, '..', 'scripts', 'distribution-release',
      'canonicalize-envelope-signature.mjs'),
    '--directory', paths.outputRoot,
  ], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  assert.equal(existsSync(actionPath), false);
  assert.deepEqual(readFileSync(canonicalPath), signature);
  await verifySignedRelease({
    directory: paths.outputRoot,
    verifyArchiveImpl: paths.verifyArchiveImpl,
    spawn: () => ({ status: 0, stdout: 'Verified OK', stderr: '' }),
  });
});

test('canonicalizer rejects overwrite and extra-entry layouts without deleting the source', (t) => {
  for (const scenario of ['overwrite', 'extra']) {
    const paths = fixture(t);
    prepareForEnvelopeSignature(paths);
    const actionPath = join(paths.outputRoot,
      'zryna-release-envelope-v1.json.sigstore.json');
    const canonicalPath = join(paths.outputRoot, 'zryna-release-envelope-v1.sigstore.json');
    const signature = Buffer.from(`${scenario} signature\n`);
    writeFileSync(actionPath, signature);
    if (scenario === 'overwrite') writeFileSync(canonicalPath, 'existing canonical\n');
    else writeFileSync(join(paths.outputRoot, 'unexpected'), 'extra\n');

    assert.throws(() => canonicalizeEnvelopeSignature(paths.outputRoot),
      /R406-RELEASE-FILE: release file inventory differs/);
    assert.deepEqual(readFileSync(actionPath), signature);
    if (scenario === 'overwrite') {
      assert.equal(readFileSync(canonicalPath, 'utf8'), 'existing canonical\n');
    } else {
      assert.equal(existsSync(canonicalPath), false);
    }
  }
});

test('canonicalizer rejects a linked signer output without creating the canonical path', (t) => {
  const paths = fixture(t);
  prepareForEnvelopeSignature(paths);
  const outside = join(paths.outputRoot, '..', 'linked-signature');
  mkdirSync(outside);
  const actionPath = join(paths.outputRoot,
    'zryna-release-envelope-v1.json.sigstore.json');
  const canonicalPath = join(paths.outputRoot, 'zryna-release-envelope-v1.sigstore.json');
  symlinkSync(outside, actionPath, process.platform === 'win32' ? 'junction' : 'dir');

  assert.throws(() => canonicalizeEnvelopeSignature(paths.outputRoot),
    /R406-RELEASE-FILE: release root contains a non-file entry/);
  assert.equal(existsSync(actionPath), true);
  assert.equal(existsSync(canonicalPath), false);
});

test('binds two reproduced archives, admission, provenance, checksums, and signed envelope', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const calls = [];
  const envelope = await verifySignedRelease({
    directory: paths.outputRoot,
    verifyArchiveImpl: paths.verifyArchiveImpl,
    spawn(executable, args) {
      calls.push([executable, ...args]);
      return { status: 0, stdout: 'Verified OK', stderr: '' };
    },
  });
  assert.equal(envelope.workflow.runId, '987654321');
  assert.equal(envelope.subjects.length, 2);
  const attestationCalls = calls.filter((call) => call[1] === 'verify-blob-attestation');
  assert.equal(attestationCalls.length, 2);
  assert(attestationCalls.every((call) => call.includes('https://slsa.dev/provenance/v1')
    && call.includes('--check-claims=true')));
  assert.equal(calls.filter((call) => call[1] === 'verify-blob').length, 3);
  assert.equal(readFileSync(join(paths.outputRoot, 'SHA256SUMS'), 'utf8').split('\n').length, 13);
});

test('rejects provenance drift and cryptographic verifier failure', async (t) => {
  const drift = fixture(t);
  prepareAll(drift);
  const provenance = join(drift.outputRoot,
    'zryna-0.2.2-x86_64-unknown-linux-gnu.intoto.jsonl');
  writeFileSync(provenance, '{}\n');
  await assert.rejects(() => verifySignedRelease({
    directory: drift.outputRoot,
    verifyArchiveImpl: drift.verifyArchiveImpl,
    spawn: () => ({ status: 0, stdout: '', stderr: '' }),
  }), /R406-SIGNED-RELEASE:.*signed descriptor/);

  const crypto = fixture(t);
  prepareAll(crypto);
  await assert.rejects(() => verifySignedRelease({
    directory: crypto.outputRoot,
    verifyArchiveImpl: crypto.verifyArchiveImpl,
    spawn: () => ({ status: 1, stdout: '', stderr: 'invalid signature' }),
  }), /R406-SIGNED-RELEASE: cosign verify-blob-attestation failed/);
});

test('rejects a valid archive subject with wrong predicate, workflow, source, or run identity', (t) => {
  for (const [label, mutate] of [
    ['predicate', (statement) => { statement.predicateType = 'https://example.invalid/predicate'; }],
    ['workflow', (statement) => {
      statement.predicate.buildDefinition.externalParameters.workflow.path = '.github/workflows/ci.yml';
    }],
    ['source', (statement) => {
      statement.predicate.buildDefinition.resolvedDependencies[0].digest.gitCommit = 'f'.repeat(40);
    }],
    ['run', (statement) => {
      statement.predicate.runDetails.metadata.invocationId =
        'https://github.com/zryna/zryna/actions/runs/987654320/attempts/2';
    }],
  ]) {
    const paths = fixture(t, mutate);
    prepareReleaseEvidence({ ...paths, phase: 'subjects' });
    assert.throws(() => prepareReleaseEvidence({ ...paths, phase: 'documents' }),
      /R406-EVIDENCE: attestation provenance policy differs/, label);
  }
});

test('signed-release admission rejects an envelope-bound header-only SBOM', async (t) => {
  const paths = fixture(t, () => {}, (_bytes, { target, archive }) => Buffer.from(`${
    canonicalBounded({
      spdxVersion: 'SPDX-2.3', dataLicense: 'CC0-1.0', SPDXID: 'SPDXRef-DOCUMENT',
      name: `zryna-0.2.2-${target.triple}`,
      documentNamespace: `https://zryna.com/spdx/0.2.2/${target.triple}/${sha256(archive)}`,
    })}\n`));
  prepareAll(paths);
  await assert.rejects(() => verifySignedRelease({
    directory: paths.outputRoot,
    verifyArchiveImpl: paths.verifyArchiveImpl,
    spawn: () => ({ status: 0, stdout: '', stderr: '' }),
  }), /R406-SPDX: document fields differ from the closed SPDX 2.3 profile/);
});

test('discovers, fills, verifies, and publishes one authenticated draft', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const server = githubServer({ decoyCount: 100 });
  const call = (phase) => publishDraftRelease({
    directory: paths.outputRoot, phase, environment: paths.environment,
    fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
  });
  await call('create-draft');
  assert.equal(server.release.draft, true);
  assert.equal(server.release.assets.length, 0);
  await call('upload');
  assert.equal(server.release.assets.length, 17);
  await call('verify-draft');
  await call('publish');
  assert.equal(server.release.draft, false);
  assert.equal(server.release.prerelease, true);
  assert.equal(server.mutations, 19);
});

test('resumes an exact partially uploaded draft without replacing assets', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const server = githubServer();
  const call = (phase) => publishDraftRelease({
    directory: paths.outputRoot, phase, environment: paths.environment,
    fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
  });
  await call('create-draft');
  server.interruptUploadsAfter(2);
  await assert.rejects(() => call('upload'), /simulated upload interruption/);
  assert.equal(server.release.assets.length, 2);
  const mutations = server.mutations;
  await call('create-draft');
  assert.equal(server.mutations, mutations);
  server.interruptUploadsAfter(null);
  await call('upload');
  assert.equal(server.release.assets.length, 17);
  assert.equal(new Set(server.uploadNames).size, 17);
  assert.equal(server.uploadNames.length, 17);
  await call('verify-draft');
});

test('rejects wrong or ambiguous draft identities before mutation', async (t) => {
  for (const scenario of ['wrong fields', 'duplicate tag']) {
    const paths = fixture(t);
    prepareAll(paths);
    const server = githubServer();
    const call = (phase) => publishDraftRelease({
      directory: paths.outputRoot, phase, environment: paths.environment,
      fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
    });
    await call('create-draft');
    if (scenario === 'wrong fields') server.mutateRelease((release) => { release.name = 'wrong'; });
    else server.addRelease({ ...server.release, id: 42,
      url: 'https://api.github.com/repos/zryna/zryna/releases/42' });
    const mutations = server.mutations;
    await assert.rejects(() => call('upload'), scenario === 'wrong fields'
      ? /GitHub release identity or state differs/
      : /multiple releases exist for the immutable tag/);
    assert.equal(server.mutations, mutations);
  }
});

test('rejects wrong authenticated draft asset identities and URLs before resume', async (t) => {
  for (const [label, mutate] of [
    ['API identity', (asset) => { asset.url = `${asset.url}-wrong`; }],
    ['published URL', (asset) => {
      asset.browser_download_url = asset.browser_download_url.replace(
        'untagged-0123456789abcdefabcd', 'v0.2.2');
    }],
    ['foreign draft URL', (asset) => {
      asset.browser_download_url = asset.browser_download_url.replace(
        'github.com/zryna/zryna', 'github.com/example/zryna');
    }],
    ['malformed draft slug', (asset) => {
      asset.browser_download_url = asset.browser_download_url.replace(
        'untagged-0123456789abcdefabcd', 'untagged-0123456789abcdefabcg');
    }],
  ]) {
    const paths = fixture(t);
    prepareAll(paths);
    const server = githubServer();
    const call = (phase) => publishDraftRelease({
      directory: paths.outputRoot, phase, environment: paths.environment,
      fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
    });
    await call('create-draft');
    server.interruptUploadsAfter(1);
    await assert.rejects(() => call('upload'), /simulated upload interruption/);
    server.mutateRelease((release) => { mutate(release.assets[0]); });
    const mutations = server.mutations;
    await assert.rejects(() => call('upload'), /server-side identity or digest differs/, label);
    assert.equal(server.mutations, mutations, label);
  }
});

test('refuses an existing published release before mutation', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const server = githubServer();
  const call = (phase) => publishDraftRelease({
    directory: paths.outputRoot, phase, environment: paths.environment,
    fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
  });
  await call('create-draft');
  await call('upload');
  await call('publish');
  const mutations = server.mutations;
  await assert.rejects(() => call('create-draft'), /GitHub release identity or state differs/);
  assert.equal(server.mutations, mutations);
});
