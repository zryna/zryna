import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical } from '../scripts/distribution-release/canonical.mjs';
import {
  validateBuildInput,
  validateBuildInputText,
} from '../scripts/distribution-release/validate-build-input.mjs';

const COMMIT = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const VERSION = '0.2.0';
const digest = (index) => index.toString(16).padStart(64, '0');

function fixture(target = 'x86_64-unknown-linux-gnu') {
  const windows = target === 'x86_64-pc-windows-msvc';
  return {
    format: 'zryna.distribution-build-input.v1',
    version: VERSION,
    tag: `v${VERSION}`,
    channel: 'beta',
    source: {
      repository: 'https://github.com/zryna/zryna',
      ref: `refs/tags/v${VERSION}`,
      commit: COMMIT,
      tree: TREE,
      sourceDateEpoch: 1_789_081_200,
    },
    target: {
      triple: target,
      archiveFormat: windows ? 'zip' : 'tar-gzip',
      platformBaseline: windows
        ? {
            os: 'windows', product: 'windows-server', version: '2022',
            architecture: 'x86_64', runtime: 'operating-system-ucrt',
          }
        : {
            os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64',
          },
    },
    toolchains: [
      {
        name: 'cargo', version: '1.97.1', origin: 'repository:rust-toolchain.toml',
        sha256: digest(1), signatureEvidenceSha256: digest(2),
      },
      {
        name: 'node', version: '22.22.1', origin: 'https://nodejs.org/dist/v22.22.1/',
        sha256: digest(3), signatureEvidenceSha256: digest(4),
      },
      {
        name: 'rustc', version: '1.97.1', origin: 'repository:rust-toolchain.toml',
        sha256: digest(5), signatureEvidenceSha256: digest(6),
      },
    ],
    materials: {
      format: 'zryna.distribution-materials.v1',
      logicalPath: 'prepared/metadata/materials.json',
      fileCount: 300,
      size: 32768,
      sha256: digest(49),
    },
    preparedDistribution: {
      format: 'zryna.distribution.v1',
      logicalPath: 'prepared/metadata/distribution.json',
      archiveFileCount: 306,
      size: 4096,
      sha256: digest(50),
    },
    compiledCli: {
      logicalPath: `inputs/compiled-cli/zryna${windows ? '.exe' : ''}`,
      size: 8192,
      sha256: digest(51),
    },
    architectureReceipt: {
      format: 'zryna.source-build-receipt.v1',
      logicalPath: 'inputs/source-build-receipt.json',
      size: 1024,
      sha256: digest(52),
      sourceCommit: COMMIT,
      sourceTree: TREE,
    },
    gateReceipt: {
      format: 'zryna.preassembly-gates.v1',
      logicalPath: 'inputs/preassembly-gates.json',
      size: 1024,
      sha256: digest(53),
      workflow: '.github/workflows/ci.yml',
      runId: '123456789',
      runAttempt: 1,
      runUrl: 'https://github.com/zryna/zryna/actions/runs/123456789',
      sourceCommit: COMMIT,
      requiredContexts: [
        'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
      ],
      requiredJobs: [
        'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
      ].map((name, index) => ({
        name, conclusion: 'success', sourceCommit: COMMIT, runId: '123456789', runAttempt: 1,
        jobId: `${200 + index}`, checkRunId: `${300 + index}`,
      })),
    },
    recipe: { format: 'zryna.distribution-recipe.v1', sha256: digest(54) },
  };
}

test('accepts canonical Linux and Windows build inputs', () => {
  for (const target of ['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc']) {
    const value = fixture(target);
    assert.equal(validateBuildInput(value), value);
    assert.deepEqual(validateBuildInputText(`${canonical(value)}\n`), value);
  }
});

test('rejects source, receipt, target, and fixed-path drift', () => {
  for (const [code, mutate] of [
    ['R406-BUILD-SOURCE', (value) => { value.tag = 'v0.2.1'; }],
    ['R406-BUILD-SOURCE', (value) => { value.architectureReceipt.sourceTree = COMMIT; }],
    ['R406-BUILD-SOURCE', (value) => { value.gateReceipt.sourceCommit = TREE; }],
    ['R406-BUILD-TARGET', (value) => { value.target.archiveFormat = 'zip'; }],
    ['R406-BUILD-PATH', (value) => { value.materials.logicalPath = 'materials.json'; }],
    ['R406-BUILD-PATH', (value) => { value.compiledCli.logicalPath = 'zryna'; }],
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateBuildInput(value), new RegExp(`${code}:`));
  }
});

test('rejects missing, stale, duplicate, and unsorted authenticated inputs', () => {
  for (const index of [0, 2, 5]) {
    const missing = fixture();
    missing.gateReceipt.requiredJobs.splice(index, 1);
    assert.throws(() => validateBuildInput(missing), /R406-BUILD-SCHEMA:/);
  }

  const stale = fixture();
  stale.gateReceipt.requiredJobs[0].sourceCommit = TREE;
  assert.throws(() => validateBuildInput(stale), /R406-BUILD-GATES:/);

  const wrongRun = fixture();
  wrongRun.gateReceipt.runUrl = 'https://github.com/zryna/zryna/actions/runs/987654321';
  assert.throws(() => validateBuildInput(wrongRun), /R406-BUILD-GATES:/);

  const duplicate = fixture();
  duplicate.toolchains[1].name = duplicate.toolchains[0].name;
  assert.throws(() => validateBuildInput(duplicate), /R406-BUILD-ORDER:/);

  const unsorted = fixture();
  unsorted.toolchains.reverse();
  assert.throws(() => validateBuildInput(unsorted), /R406-BUILD-ORDER:/);

  const arbitraryTool = fixture();
  arbitraryTool.toolchains[2].version = '1.89.0';
  assert.throws(() => validateBuildInput(arbitraryTool), /R406-BUILD-TOOLCHAINS:/);

  const extraContext = fixture();
  extraContext.gateReceipt.requiredContexts[5] = 'new-required-check';
  assert.throws(() => validateBuildInput(extraContext), /R406-BUILD-GATES:/);

  const mixedAttempt = fixture();
  mixedAttempt.gateReceipt.requiredJobs[0].runAttempt = 2;
  assert.throws(() => validateBuildInput(mixedAttempt), /R406-BUILD-GATES:/);

  const conflictingCount = fixture();
  conflictingCount.materials.fileCount = conflictingCount.preparedDistribution.archiveFileCount;
  assert.throws(() => validateBuildInput(conflictingCount), /R406-BUILD-MATERIALS:/);
});

test('schema and canonical parser reject open or ambiguous inputs', () => {
  for (const mutate of [
    (value) => { value.channel = 'stable'; },
    (value) => { value.version = '0.2.0-beta.1'; },
    (value) => { value.compiledCli.logicalPath = '../zryna'; },
    (value) => { value.privatePath = 'C:\\private'; },
    (value) => { value.target.platformBaseline.version = '22.04'; },
    (value) => { value.materials.fileCount = 513; },
  ]) {
    const value = fixture();
    mutate(value);
    assert.throws(() => validateBuildInput(value), /R406-BUILD-SCHEMA:/);
  }

  assert.throws(() => validateBuildInputText(JSON.stringify(fixture())), /R406-CANONICAL:/);
});
