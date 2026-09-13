import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { parseDocument } from 'yaml';
import { canonicalBounded, parseCanonical, sha256 } from '../scripts/distribution-release/canonical.mjs';
import {
  compareProductionCandidateBuilds, validateProductionCandidateReproduction,
} from '../scripts/distribution-release/compare-production-candidate-builds.mjs';
import { validateReproduction } from '../scripts/distribution-release/compare-release-builds.mjs';
import {
  validateProductionCandidateAuthority,
} from '../scripts/distribution-release/production-candidate-authority.mjs';
import {
  acceptProductionCandidate,
} from '../scripts/distribution-release/run-installed-acceptance.mjs';
import { MAX_RELEASE_DOCUMENT } from '../scripts/distribution-release/release-files.mjs';
import { runProductionCandidateBuild } from '../scripts/distribution-release/run-production-candidate-build.mjs';
import { validateReleaseTagReceipt } from '../scripts/distribution-release/validate-release-tag-receipt.mjs';

const TARGET = 'x86_64-unknown-linux-gnu';
const COMMIT = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const RECIPE = { format: 'zryna.distribution-recipe.v1', sha256: 'c'.repeat(64) };
const FILES = {
  archive: `zryna-0.2.0-${TARGET}.tar.gz`,
  buildReceipt: `zryna-0.2.0-${TARGET}.build-receipt.json`,
  sbom: `zryna-0.2.0-${TARGET}.spdx.json`,
};

function authority() {
  const runId = '34752906563';
  const requiredContexts = [
    'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
  ];
  return {
    format: 'zryna.release-production-candidate-authority.v1',
    status: 'production-candidate', productionAdmission: 'forbidden', versionCandidate: '0.2.0',
    observedSourceRef: 'refs/heads/main',
    intendedRelease: { ref: 'refs/tags/v0.2.0', tagProvenance: 'not-observed' },
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: TREE, sourceDateEpoch: 1_789_081_200 },
    workflow: { path: '.github/workflows/release-production-candidate.yml',
      size: 100, sha256: 'd'.repeat(64) },
    gates: {
      format: 'zryna.preassembly-gates.v1', repository: 'https://github.com/zryna/zryna',
      workflow: '.github/workflows/ci.yml', runId, runAttempt: 1,
      runUrl: `https://github.com/zryna/zryna/actions/runs/${runId}`, sourceCommit: COMMIT,
      requiredContexts,
      requiredJobs: requiredContexts.map((name, index) => ({ name, conclusion: 'success',
        sourceCommit: COMMIT, runId, runAttempt: 1,
        jobId: String(index + 1), checkRunId: String(index + 11) })),
    },
  };
}

function roots(t) {
  const root = resolve(mkdtempSync(join(tmpdir(), 'zryna-production-candidate-test-')));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  return root;
}

function build(root, replica, archive = Buffer.from('candidate archive')) {
  mkdirSync(root);
  const bytes = {
    archive, buildReceipt: Buffer.from('{}'),
    sbom: Buffer.from('{"spdxVersion":"SPDX-2.3"}'),
  };
  const artifacts = Object.fromEntries(Object.entries(FILES).map(([key, path]) => [key, {
    path, size: bytes[key].length, sha256: sha256(bytes[key]),
  }]));
  const value = {
    format: 'zryna.production-candidate-build-result.v1', status: 'production-candidate',
    productionAdmission: 'forbidden', version: '0.2.0', target: TARGET, replica,
    observedSource: authority().source, intendedRelease: authority().intendedRelease,
    recipe: RECIPE, artifacts,
  };
  for (const [key, path] of Object.entries(FILES)) writeFileSync(join(root, path), bytes[key]);
  writeFileSync(join(root, 'production-candidate-build-result.json'),
    `${canonicalBounded(value)}\n`);
  return value;
}

test('candidate authority distinguishes observed main from an unobserved intended tag', () => {
  const value = authority();
  assert.equal(validateProductionCandidateAuthority(value), value);
  for (const mutate of [
    (copy) => { copy.source.ref = 'refs/tags/v0.2.0'; },
    (copy) => { copy.intendedRelease.tagProvenance = 'observed'; },
    (copy) => { copy.productionAdmission = 'allowed'; },
    (copy) => { copy.format = 'zryna.release-qualification-source.v1'; },
  ]) {
    const copy = structuredClone(value);
    mutate(copy);
    assert.throws(() => validateProductionCandidateAuthority(copy),
      /R406-PRODUCTION-CANDIDATE-AUTHORITY/);
  }
  assert.throws(() => validateReleaseTagReceipt(value), /R406-TAG-SCHEMA/);
});

test('candidate runner rejects qualification admission before any production build', async (t) => {
  const root = roots(t);
  const admissionRoot = join(root, 'admission');
  const outputRoot = join(root, 'output');
  const sourceRoot = join(root, 'source');
  mkdirSync(admissionRoot);
  mkdirSync(sourceRoot);
  writeFileSync(join(admissionRoot, 'qualification-source.json'), '{}');
  await assert.rejects(() => runProductionCandidateBuild({
    admissionRoot, outputRoot, sourceRoot, target: TARGET, replica: 1,
    environment: {
      GITHUB_EVENT_NAME: 'workflow_dispatch', GITHUB_REPOSITORY: 'zryna/zryna',
      GITHUB_REF: 'refs/heads/main', GITHUB_REF_TYPE: 'branch', GITHUB_REF_NAME: 'main',
      GITHUB_REF_PROTECTED: 'true', GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
      GITHUB_WORKFLOW_REF:
        'zryna/zryna/.github/workflows/release-production-candidate.yml@refs/heads/main',
    },
  }), /R406-RELEASE-FILE: release file inventory differs/);
});

test('candidate runner fails closed while the production recipe digest is unaccepted', async (t) => {
  const root = roots(t);
  const admissionRoot = join(root, 'admission');
  const outputRoot = join(root, 'output');
  const sourceRoot = join(root, 'source');
  mkdirSync(admissionRoot);
  mkdirSync(sourceRoot);
  writeFileSync(join(admissionRoot, 'candidate-authority.json'),
    `${canonicalBounded(authority())}\n`);
  const recipe = readFileSync(resolve(import.meta.dirname, '..', 'scripts', 'distribution',
    'release-recipe-v1.json'));
  const spawn = (_executable, args) => ({ status: 0, stdout: args[0] === 'ls-tree'
    ? Buffer.from(`100644 blob ${'e'.repeat(40)}\tscripts/distribution/release-recipe-v1.json\0`)
    : recipe });
  await assert.rejects(() => runProductionCandidateBuild({
    admissionRoot, outputRoot, sourceRoot, target: TARGET, replica: 1, spawn,
    acceptedRecipeSha256: null,
    environment: {
      GITHUB_EVENT_NAME: 'workflow_dispatch', GITHUB_REPOSITORY: 'zryna/zryna',
      GITHUB_REF: 'refs/heads/main', GITHUB_REF_TYPE: 'branch', GITHUB_REF_NAME: 'main',
      GITHUB_REF_PROTECTED: 'true', GITHUB_SHA: COMMIT, GITHUB_WORKFLOW_SHA: COMMIT,
      GITHUB_WORKFLOW_REF:
        'zryna/zryna/.github/workflows/release-production-candidate.yml@refs/heads/main',
      ZRYNA_TARGET: TARGET, ZRYNA_REPLICA: '1',
    },
  }), /recipe bytes differ from the independently accepted digest/);
});

test('candidate replicas reproduce and installed acceptance remains publication-forbidden', async (t) => {
  const root = roots(t);
  const firstRoot = join(root, 'first');
  const secondRoot = join(root, 'second');
  const reproducedRoot = join(root, 'reproduced');
  const acceptanceRoot = join(root, 'acceptance');
  const workRoot = join(root, 'work');
  build(firstRoot, 1);
  build(secondRoot, 2);
  mkdirSync(workRoot);
  const reproduced = compareProductionCandidateBuilds({
    firstRoot, secondRoot, outputRoot: reproducedRoot, target: TARGET,
  });
  assert.throws(() => validateReproduction(reproduced, TARGET),
    /R406-REPRODUCTION: reproduction result schema differs/);
  const archiveSha256 = reproduced.artifacts.archive.sha256;
  const files = [
    { path: 'VERSION', mode: 0o644, data: Buffer.from('0.2.0\n') },
    { path: 'bin/zryna', mode: 0o755, data: Buffer.from('cli') },
    { path: 'runtime/node/bin/node', mode: 0o755, data: Buffer.from('runtime') },
    { path: 'lib/zryna/bootstrap/worker.mjs', mode: 0o644, data: Buffer.from('provider') },
    { path: 'metadata/distribution.json', mode: 0o644, data: Buffer.from('{}\n') },
  ];
  const spawn = (executable, args) => {
    const tampered = executable.includes('tampered-');
    return { status: tampered ? 2 : 0, signal: null,
      stdout: Buffer.from(args[0] === '--version' ? 'zryna 0.2.0\n'
        : args[0] === 'run' ? `${args[args.indexOf('--target') + 1]}: i32 42\n` : ''),
      stderr: Buffer.from(tampered ? 'error[ZRYNA-C4220]: changed installation\n' : '') };
  };
  const receipt = await acceptProductionCandidate({
    inputRoot: reproducedRoot, outputRoot: acceptanceRoot, workRoot, target: TARGET, spawn,
    verifyArchiveImpl: async (_archive, _descriptor, options) => {
      assert.deepEqual(options, { productionCandidate: true });
      return { archiveSha256, files, distribution: { files: [
        { path: 'lib/zryna/bootstrap/worker.mjs', role: 'provider' },
        { path: 'runtime/node/bin/node', role: 'runtime' },
      ] } };
    },
  });
  assert.equal(receipt.productionAdmission, 'forbidden');
  assert.equal(receipt.intendedRelease.tagProvenance, 'not-observed');
  assert.equal(parseCanonical(readFileSync(join(acceptanceRoot,
    'production-candidate-installed-acceptance.json'), 'utf8')).archiveSha256, archiveSha256);
});

test('candidate reproduction rejects malformed build evidence', (t) => {
  const root = roots(t);
  const firstRoot = join(root, 'first');
  const secondRoot = join(root, 'second');
  const reproducedRoot = join(root, 'reproduced');
  build(firstRoot, 1);
  build(secondRoot, 2);
  const value = compareProductionCandidateBuilds({
    firstRoot, secondRoot, outputRoot: reproducedRoot, target: TARGET,
  });
  for (const mutate of [
    (copy) => { copy.builds[0].unexpected = true; },
    (copy) => { copy.builds[0].manifestSha256 = 'not-a-digest'; },
    (copy) => { copy.builds[1].replica = 3; },
  ]) {
    const copy = structuredClone(value);
    mutate(copy);
    assert.throws(() => validateProductionCandidateReproduction(copy, TARGET),
      /R406-PRODUCTION-CANDIDATE-REPRODUCTION/);
  }
});

test('candidate installed-acceptance CLI recognizes the literal boolean switch', (t) => {
  const root = roots(t);
  const inputRoot = join(root, 'missing-input');
  const outputRoot = join(root, 'output');
  const workRoot = join(root, 'work');
  const result = spawnSync(process.execPath, [
    resolve(import.meta.dirname, '..', 'scripts', 'distribution-release',
      'run-installed-acceptance.mjs'),
    '--input', inputRoot, '--output', outputRoot, '--work', workRoot, '--candidate', 'true',
  ], { encoding: 'utf8', env: { ...process.env, ZRYNA_TARGET: TARGET } });
  assert.equal(result.status, 1);
  assert.match(result.stderr, /production-candidate-reproduction\.json/);
  assert.doesNotMatch(result.stderr, /optional --candidate true/);
});

test('candidate installed-acceptance CLI reads archives beyond the document bound', (t) => {
  const root = roots(t);
  const firstRoot = join(root, 'first');
  const secondRoot = join(root, 'second');
  const inputRoot = join(root, 'input');
  const outputRoot = join(root, 'output');
  const workRoot = join(root, 'work');
  const archive = Buffer.alloc(MAX_RELEASE_DOCUMENT + 1, 0x78);
  build(firstRoot, 1, archive);
  build(secondRoot, 2, archive);
  compareProductionCandidateBuilds({
    firstRoot, secondRoot, outputRoot: inputRoot, target: TARGET,
  });
  mkdirSync(workRoot);
  const result = spawnSync(process.execPath, [
    resolve(import.meta.dirname, '..', 'scripts', 'distribution-release',
      'run-installed-acceptance.mjs'),
    '--input', inputRoot, '--output', outputRoot, '--work', workRoot, '--candidate', 'true',
  ], { encoding: 'utf8', env: { ...process.env, ZRYNA_TARGET: TARGET } });
  assert.equal(result.status, 1);
  const expectedRejection = process.versions.node === '22.22.1'
      && process.versions.zlib === '1.3.1-e00f703'
    ? /incorrect header check/
    : /D422-ADMISSION: archive Node\/zlib recipe mismatch/;
  assert.match(result.stderr, expectedRejection);
  assert.doesNotMatch(result.stderr, /must be one direct regular file within its byte bound/);
});

test('candidate workflow is protected-main, read-only, and has no publisher or signing authority', () => {
  const path = resolve(import.meta.dirname, '..', '.github', 'workflows',
    'release-production-candidate.yml');
  const parsed = parseDocument(readFileSync(path, 'utf8'));
  assert.deepEqual(parsed.errors, []);
  const workflow = parsed.toJS();
  assert.deepEqual(workflow.on, { workflow_dispatch: null });
  assert.deepEqual(Object.keys(workflow.jobs), ['admit', 'build', 'reproduce', 'installed-acceptance']);
  assert.equal(workflow.jobs.build.env.CARGO_HOME,
    '${{ github.workspace }}/zryna-bootstrap-cargo');
  for (const job of Object.values(workflow.jobs)) {
    assert.notEqual(job.permissions?.contents, 'write');
    assert.equal(job.permissions?.['id-token'], undefined);
    assert.equal(job.environment, undefined);
    for (const step of job.steps) {
      if (step.uses) assert.match(step.uses, /^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+@[0-9a-f]{40}$/);
      if (step.uses?.startsWith('actions/checkout@')) {
        assert.equal(step.with['persist-credentials'], false);
      }
    }
  }
  assert(!JSON.stringify(workflow).includes('publish-draft-release'));
  assert(!JSON.stringify(workflow).includes('actions/attest'));
  assert.equal(workflow.jobs['installed-acceptance'].steps
    .filter(({ name }) => name === 'Install workflow dependencies').length, 1);
});
