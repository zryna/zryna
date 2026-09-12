import assert from 'node:assert/strict';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { canonicalBounded, parseCanonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { releaseSpdxBytes } from '../scripts/distribution-release/release-sbom.mjs';
import { runProtectedBuild } from '../scripts/distribution-release/run-protected-build.mjs';

const COMMIT = 'b'.repeat(40);
const TREE = 'c'.repeat(40);
const TARGET = 'x86_64-unknown-linux-gnu';
const JOBS = ['adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)'];

function wire(value) { return Buffer.from(`${canonicalBounded(value)}\n`); }

function fixture(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-protected-build-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const admissionRoot = join(parent, 'admission');
  const sourceRoot = join(parent, 'source');
  const outputRoot = join(parent, 'output');
  mkdirSync(admissionRoot);
  mkdirSync(sourceRoot);
  writeFileSync(join(admissionRoot, 'tag-receipt.json'), wire({
    format: 'zryna.release-tag-receipt.v1', version: '0.2.0',
    source: {
      repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/v0.2.0',
      tagType: 'annotated', tagObject: 'a'.repeat(40), commit: COMMIT, tree: TREE,
      sourceDateEpoch: 1_789_081_200,
    },
    workflow: { path: '.github/workflows/release.yml', size: 100, sha256: 'e'.repeat(64) },
  }));
  writeFileSync(join(admissionRoot, 'preassembly-gates.json'), wire({
    format: 'zryna.preassembly-gates.v1', repository: 'https://github.com/zryna/zryna',
    workflow: '.github/workflows/ci.yml', runId: '123456789', runAttempt: 1,
    runUrl: 'https://github.com/zryna/zryna/actions/runs/123456789', sourceCommit: COMMIT,
    requiredContexts: JOBS,
    requiredJobs: JOBS.map((name, index) => ({
      name, conclusion: 'success', sourceCommit: COMMIT, runId: '123456789', runAttempt: 1,
      jobId: String(index + 10), checkRunId: String(index + 20),
    })),
  }));
  const recipeBytes = wire({ format: 'zryna.distribution-recipe.v1', qualification: 'fixture' });
  const spawn = (executable, args, options) => {
    assert.equal(executable, 'git');
    let stdout;
    if (args[0] === 'ls-tree') {
      stdout = Buffer.from(`100644 blob ${'f'.repeat(40)}\tscripts/distribution/release-recipe-v1.json\0`);
    } else if (args[0] === 'cat-file') stdout = recipeBytes;
    else throw new Error(`unexpected git command ${args.join(' ')}`);
    return { status: 0, stdout: options.encoding === null ? stdout : stdout.toString(options.encoding) };
  };
  return {
    admissionRoot: resolve(admissionRoot), sourceRoot: resolve(sourceRoot),
    outputRoot: resolve(outputRoot), recipeBytes, spawn,
    environment: {
      GITHUB_REPOSITORY: 'zryna/zryna', GITHUB_REF: 'refs/tags/v0.2.0',
      GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
      ZRYNA_TARGET: TARGET, ZRYNA_REPLICA: '1',
    },
  };
}

function adapters() {
  let preparedRecord;
  const indexed = [
    { path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n'), material: 'source',
      licenses: ['LICENSE'], role: 'license' },
    { path: 'VERSION', mode: 0o644, data: Buffer.from('0.2.0\n'), material: 'source',
      licenses: ['LICENSE'], role: 'notice' },
  ].map((file) => ({ ...file, size: file.data.length, sha256: sha256(file.data) }));
  const inventory = wire({
    format: 'zryna.distribution-inventory.v1',
    files: indexed.map(({ data, ...entry }) => entry),
  });
  const files = [
    ...indexed.map(({ material, licenses, role, size, sha256: digest, ...file }) => file),
    { path: 'metadata/inventory.json', mode: 0o644, data: inventory },
    { path: 'metadata/checksums.sha256', mode: 0o644, data: Buffer.from('checksums\n') },
  ];
  return {
    createArchitectureReceipt: async () => wire({ receipt: 'architecture' }),
    captureReleaseMaterials: async () => [],
    preparePayload(identity) {
      preparedRecord = { format: 'zryna.distribution.v1', ...identity, files: [] };
      return {
        record: preparedRecord,
        distribution: wire(preparedRecord),
        payload: [],
        materials: {
          format: 'zryna.distribution-materials.v1', logicalPath: 'prepared/metadata/materials.json',
          fileCount: 1, size: 10, sha256: '1'.repeat(64),
        },
        preparedDistribution: {
          format: 'zryna.distribution.v1', logicalPath: 'prepared/metadata/distribution.json',
          archiveFileCount: 2, size: 20, sha256: '2'.repeat(64),
        },
      };
    },
    compileReleaseCli: async () => ({
      cli: Buffer.from('compiled cli'),
      toolchains: [
        { name: 'cargo', version: '1.97.1', origin: 'repository:rust-toolchain.toml',
          sha256: '3'.repeat(64), signatureEvidenceSha256: '4'.repeat(64) },
        { name: 'node', version: '22.22.1', origin: 'https://nodejs.org/dist/v22.22.1/',
          sha256: '5'.repeat(64), signatureEvidenceSha256: '6'.repeat(64) },
        { name: 'rustc', version: '1.97.1', origin: 'repository:rust-toolchain.toml',
          sha256: '7'.repeat(64), signatureEvidenceSha256: '8'.repeat(64) },
      ],
    }),
    assemble: async () => ({
      archive: Buffer.from('archive'), receipt: wire({ receipt: 'build' }),
      filename: 'zryna-0.2.0-x86_64-unknown-linux-gnu.tar.gz', files,
    }),
    verifyArchive: async (archive) => ({
      archiveSha256: sha256(archive), distribution: preparedRecord, files,
    }),
    createReleaseSbom: async (context) => releaseSpdxBytes(context),
  };
}

test('orchestrates authenticated prepare, compile, assemble, verify, and SPDX outputs', async (t) => {
  const paths = fixture(t);
  const result = await runProtectedBuild({
    ...paths, target: TARGET, replica: 1,
    acceptedRecipeSha256: sha256(paths.recipeBytes), adapters: adapters(),
  });
  assert.equal(result.artifacts.archive.sha256, sha256('archive'));
  assert.equal(readFileSync(join(paths.outputRoot, 'build-result.json'), 'utf8'),
    `${canonicalBounded(result)}\n`);
  assert.deepEqual(readFileSync(join(paths.outputRoot, result.artifacts.archive.path)),
    Buffer.from('archive'));
});

test('rejects execution while no exact recipe digest is accepted', async (t) => {
  const paths = fixture(t);
  await assert.rejects(() => runProtectedBuild({
    ...paths, target: TARGET, replica: 1, acceptedRecipeSha256: null, adapters: adapters(),
  }), /R406-PROTECTED-BUILD: recipe bytes differ from the independently accepted digest/);
});

test('rejects a valid-looking SPDX header without complete archive and material coverage', async (t) => {
  const paths = fixture(t);
  const implementation = adapters();
  implementation.createReleaseSbom = async ({ archive }) => wire({
    spdxVersion: 'SPDX-2.3', dataLicense: 'CC0-1.0', SPDXID: 'SPDXRef-DOCUMENT',
    name: `zryna-0.2.0-${TARGET}`,
    documentNamespace: `https://zryna.com/spdx/0.2.0/${TARGET}/${archive.sha256}`,
  });
  await assert.rejects(() => runProtectedBuild({
    ...paths, target: TARGET, replica: 1,
    acceptedRecipeSha256: sha256(paths.recipeBytes), adapters: implementation,
  }), /R406-SPDX: document fields differ from the closed SPDX 2.3 profile/);
});

test('rejects an SPDX document that omits one authenticated archive relationship', async (t) => {
  const paths = fixture(t);
  const implementation = adapters();
  implementation.createReleaseSbom = async (context) => {
    const value = parseCanonical(releaseSpdxBytes(context).toString('utf8'));
    value.relationships.pop();
    return wire(value);
  };
  await assert.rejects(() => runProtectedBuild({
    ...paths, target: TARGET, replica: 1,
    acceptedRecipeSha256: sha256(paths.recipeBytes), adapters: implementation,
  }), /R406-SPDX: SPDX subject, archive files, materials, licenses, or relationships differ/);
});
