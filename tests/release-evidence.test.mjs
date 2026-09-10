import assert from 'node:assert/strict';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { prepareReleaseEvidence } from '../scripts/distribution-release/prepare-release-evidence.mjs';
import { publishDraftRelease } from '../scripts/distribution-release/publish-draft-release.mjs';
import { verifySignedRelease } from '../scripts/distribution-release/verify-signed-release.mjs';

const VERSION = '0.2.0';
const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const TAG_OBJECT = 'a'.repeat(40);
const RECIPE = 'd'.repeat(64);
const TARGETS = [
  { key: 'windows', triple: 'x86_64-pc-windows-msvc', extension: 'zip' },
  { key: 'linux', triple: 'x86_64-unknown-linux-gnu', extension: 'tar.gz' },
];

function source(tagType = false) {
  return {
    repository: 'https://github.com/zryna/zryna',
    ref: 'refs/tags/v0.2.0',
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

function fixture(t) {
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
  for (const target of TARGETS) {
    const root = join(inputRoot, target.key);
    mkdirSync(root);
    const base = `zryna-${VERSION}-${target.triple}`;
    const bytes = {
      archive: Buffer.from(`${target.triple} archive`),
      buildReceipt: Buffer.from(`{"target":"${target.triple}"}\n`),
      sbom: Buffer.from(`{"spdxVersion":"SPDX-2.3","name":"${target.triple}"}\n`),
    };
    const paths = {
      archive: `${base}.${target.extension}`,
      buildReceipt: `${base}.build-receipt.json`,
      sbom: `${base}.spdx.json`,
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
    const statement = {
      _type: 'https://in-toto.io/Statement/v1',
      subject: [{ name: artifacts.archive.path, digest: { sha256: artifacts.archive.sha256 } }],
      predicateType: 'https://slsa.dev/provenance/v1',
      predicate: { buildDefinition: { buildType: 'https://actions.github.io/buildtypes/workflow/v1' } },
    };
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
  return {
    inputRoot: resolve(inputRoot), admissionRoot: resolve(admissionRoot), outputRoot: resolve(outputRoot),
    environment: {
      GITHUB_TOKEN: 'test-token', GITHUB_SERVER_URL: 'https://github.com',
      GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_REF: 'refs/tags/v0.2.0',
      GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
      GITHUB_RUN_ID: '987654321', GITHUB_RUN_ATTEMPT: '2',
      ZRYNA_LINUX_ATTESTATION: attestationPaths.linux,
      ZRYNA_WINDOWS_ATTESTATION: attestationPaths.windows,
    },
  };
}

function githubServer() {
  let release = null;
  let mutations = 0;
  const response = (status, value) => {
    const body = Buffer.from(JSON.stringify(value));
    return new Response(body, { status, headers: {
      'content-length': String(body.length), 'content-type': 'application/json',
    } });
  };
  const fetchImpl = async (url, options) => {
    const parsed = new URL(url);
    if (options.method === 'GET' && parsed.pathname.endsWith('/releases/tags/v0.2.0')) {
      return release === null ? response(404, { message: 'Not Found' }) : response(200, release);
    }
    if (options.method === 'POST' && parsed.pathname.endsWith('/releases')) {
      mutations += 1;
      const input = JSON.parse(options.body);
      release = {
        id: 41, url: 'https://api.github.com/repos/zryna/zryna/releases/41',
        tag_name: input.tag_name, name: input.name, body: input.body,
        draft: input.draft, prerelease: input.prerelease, assets: [],
      };
      return response(201, release);
    }
    if (options.method === 'POST' && parsed.hostname === 'uploads.github.com') {
      mutations += 1;
      const name = parsed.searchParams.get('name');
      const bytes = Buffer.from(await new Response(options.body).arrayBuffer());
      const asset = {
        id: 100 + release.assets.length,
        name,
        state: 'uploaded',
        size: bytes.length,
        digest: `sha256:${sha256(bytes)}`,
        browser_download_url: `https://github.com/zryna/zryna/releases/download/v0.2.0/${encodeURIComponent(name)}`,
      };
      release.assets.push(asset);
      return response(201, asset);
    }
    if (options.method === 'PATCH' && parsed.pathname.endsWith('/releases/41')) {
      mutations += 1;
      const input = JSON.parse(options.body);
      release = { ...release, draft: input.draft, prerelease: input.prerelease };
      return response(200, release);
    }
    throw new Error(`unexpected request ${options.method} ${url}`);
  };
  return { fetchImpl, get release() { return release; }, get mutations() { return mutations; } };
}

function prepareAll(paths) {
  for (const phase of ['subjects', 'documents']) {
    prepareReleaseEvidence({ ...paths, phase });
  }
  writeFileSync(join(paths.outputRoot, 'SHA256SUMS.sigstore.json'), '{}\n');
  writeFileSync(join(paths.outputRoot, 'RELEASE_NOTES.md.sigstore.json'), '{}\n');
  prepareReleaseEvidence({ ...paths, phase: 'envelope' });
  writeFileSync(join(paths.outputRoot, 'zryna-release-envelope-v1.sigstore.json'), '{}\n');
}

test('binds two reproduced archives, admission, provenance, checksums, and signed envelope', (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const calls = [];
  const envelope = verifySignedRelease({
    directory: paths.outputRoot,
    spawn(executable, args) {
      calls.push([executable, ...args]);
      return { status: 0, stdout: 'Verified OK', stderr: '' };
    },
  });
  assert.equal(envelope.workflow.runId, '987654321');
  assert.equal(envelope.subjects.length, 2);
  assert.equal(calls.filter((call) => call[1] === 'verify-blob-attestation').length, 2);
  assert.equal(calls.filter((call) => call[1] === 'verify-blob').length, 3);
  assert.equal(readFileSync(join(paths.outputRoot, 'SHA256SUMS'), 'utf8').split('\n').length, 13);
});

test('rejects provenance drift and cryptographic verifier failure', (t) => {
  const drift = fixture(t);
  prepareAll(drift);
  const provenance = join(drift.outputRoot,
    'zryna-0.2.0-x86_64-unknown-linux-gnu.intoto.jsonl');
  writeFileSync(provenance, '{}\n');
  assert.throws(() => verifySignedRelease({
    directory: drift.outputRoot,
    spawn: () => ({ status: 0, stdout: '', stderr: '' }),
  }), /R406-SIGNED-RELEASE:.*signed descriptor/);

  const crypto = fixture(t);
  prepareAll(crypto);
  assert.throws(() => verifySignedRelease({
    directory: crypto.outputRoot,
    spawn: () => ({ status: 1, stdout: '', stderr: 'invalid signature' }),
  }), /R406-SIGNED-RELEASE: cosign verify-blob-attestation failed/);
});

test('creates, fills, verifies, and publishes only the exact server-digested asset set', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const server = githubServer();
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

test('refuses an existing release before mutation', async (t) => {
  const paths = fixture(t);
  prepareAll(paths);
  const server = githubServer();
  await publishDraftRelease({
    directory: paths.outputRoot, phase: 'create-draft', environment: paths.environment,
    fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
  });
  const mutations = server.mutations;
  await assert.rejects(() => publishDraftRelease({
    directory: paths.outputRoot, phase: 'create-draft', environment: paths.environment,
    fetchImpl: server.fetchImpl, requestTimeoutMs: 5_000, verifyImpl: () => {},
  }), /R406-PUBLISHER: a release already exists/);
  assert.equal(server.mutations, mutations);
});
