import assert from 'node:assert/strict';
import {
  mkdtempSync, readFileSync, rmSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { writeReleaseQualificationResult } from '../scripts/distribution-release/write-release-qualification-result.mjs';

const COMMIT = 'b'.repeat(40);
const TARGET = 'x86_64-unknown-linux-gnu';

function fixture(t) {
  const parent = mkdtempSync(join(tmpdir(), 'zryna-qualification-result-'));
  t.after(() => rmSync(parent, { recursive: true, force: true }));
  const outputRoot = join(parent, 'output');
  const input = {
    format: 'zryna.release-qualification-input.v1', status: 'provisional-candidate',
    productionAdmission: 'forbidden', versionCandidate: '0.2.0',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: 'c'.repeat(40), sourceDateEpoch: 1_789_081_200 },
    workflow: { path: '.github/workflows/release-qualification.yml', sha256: 'a'.repeat(64) },
    target: { triple: TARGET, platformBaseline: { os: 'linux', distribution: 'ubuntu',
      version: '24.04', architecture: 'x86_64' } },
    recipeProposal: { gitPath: 'scripts/distribution/release-recipe-v1.json',
      size: 100, sha256: 'd'.repeat(64) },
    sourceReceipt: { logicalPath: 'qualification/source.json', size: 10, sha256: '1'.repeat(64) },
    architectureReceipt: { logicalPath: 'qualification/architecture.json', size: 10,
      sha256: '2'.repeat(64), sourceCommit: COMMIT, sourceTree: 'c'.repeat(40) },
    gateReceipt: { logicalPath: 'qualification/gates.json', size: 10,
      sha256: '3'.repeat(64), runId: '123456789', runAttempt: 1, sourceCommit: COMMIT,
      requiredJobs: ['adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)']
        .map((name) => ({ name, conclusion: 'success', sourceCommit: COMMIT })) },
    toolchains: [
      ['cargo', '1.97.1', 'repository:rust-toolchain.toml', '4', '5'],
      ['node', '22.22.1', 'https://nodejs.org/dist/v22.22.1/', '6', '7'],
      ['rustc', '1.97.1', 'repository:rust-toolchain.toml', '8', '9'],
    ].map(([name, version, origin, digest, signature]) => ({ name, version, origin, size: 100,
      sha256: digest.repeat(64), signatureEvidenceSha256: signature.repeat(64) })),
    nativeTools: [{ name: 'linker', version: 'GNU ld 2.42', origin: 'ubuntu:binutils', size: 100,
      sha256: 'a'.repeat(64), signatureEvidenceSha256: 'b'.repeat(64) }],
    materials: { files: [{ path: 'LICENSE', mode: 420, size: 8, sha256: 'c'.repeat(64),
      role: 'license', material: 'source', licenses: ['LICENSE'] }] },
    compile: { argv: ['cargo', 'rustc', '--locked', '--release', '--target', TARGET,
      '-p', 'zryna', '--bin', 'zryna'], environment: [{ name: 'SOURCE_DATE_EPOCH', value: '1789081200' }],
      encodedRustFlags: ['-Cdebuginfo=0'], encodedLinkerFlags: ['--build-id=none'] },
    archive: { format: 'tar-gzip',
      root: `zryna-qualification-0.2.0-${TARGET}-${COMMIT.slice(0, 12)}`, tarFormat: 'ustar',
      uid: 0, gid: 0, owner: '', group: '', entryMtime: 'source-epoch', gzipLevel: 9,
      gzipMtime: 0, gzipOs: 255 },
  };
  const binding = Buffer.from(`${canonicalBounded(input)}\n`);
  const cli = Buffer.alloc(64, 7);
  const assembled = {
    filename: `${input.archive.root}.tar.gz`, archive: Buffer.from('archive'),
    inventory: Buffer.from('{"files":[]}\n'),
  };
  const inspectionValue = {
    format: 'zryna.release-qualification-inspection.v1', status: 'qualification-only',
    productionAdmission: 'forbidden', target: TARGET, sourceCommit: COMMIT,
    bindingSha256: sha256(binding),
    binary: { format: 'ELF', architecture: 'x86_64', size: cli.length, sha256: sha256(cli) },
    checks: { identityMarker: 'qualification-binding-sha256', sourceRootOccurrences: 0,
      workRootOccurrences: 0, dynamicDependencies: ['libc.so.6'] },
  };
  const inspection = Buffer.from(`${canonicalBounded(inspectionValue)}\n`);
  return { outputRoot, replica: 1, binding, cli, assembled, inspection, inspectionValue };
}

test('writes one exact qualification-only replica result', (t) => {
  const value = fixture(t);
  const result = writeReleaseQualificationResult(value);
  assert.equal(result.productionAdmission, 'forbidden');
  assert.equal(readFileSync(join(value.outputRoot, 'qualification-result.json'), 'utf8'),
    `${canonicalBounded(result)}\n`);
  assert.deepEqual(readFileSync(join(value.outputRoot, result.artifacts.cli.path)), value.cli);
});

test('rejects inspection identity drift and an existing output root', (t) => {
  const drift = fixture(t);
  const changed = structuredClone(drift.inspectionValue);
  changed.binary.sha256 = 'f'.repeat(64);
  drift.inspection = Buffer.from(`${canonicalBounded(changed)}\n`);
  assert.throws(() => writeReleaseQualificationResult(drift),
    /R406-QUALIFICATION-INSPECTION: inspection, qualification input, and binary identities differ/);

  const existing = fixture(t);
  writeReleaseQualificationResult(existing);
  assert.throws(() => writeReleaseQualificationResult(existing), /EEXIST/);
});
