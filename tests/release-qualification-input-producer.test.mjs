import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { canonicalBounded, sha256 } from '../scripts/distribution-release/canonical.mjs';
import { createReleaseQualificationInput } from '../scripts/distribution-release/create-release-qualification-input.mjs';

const COMMIT = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const RECIPE = readFileSync(new URL('../scripts/distribution/release-recipe-v1.json', import.meta.url));

function bytes(value) {
  return Buffer.from(`${canonicalBounded(value)}\n`);
}

function tool(name, version, origin, digit) {
  return { name, version, origin, size: 100 + digit, sha256: digit.toString(16).repeat(64),
    observationEvidenceSha256: ((digit + 1) % 16).toString(16).repeat(64) };
}

function fixture() {
  const sourceBytes = bytes({
    format: 'zryna.release-qualification-source.v1', status: 'provisional-candidate',
    productionAdmission: 'forbidden', versionCandidate: '0.2.0',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: TREE, sourceDateEpoch: 1_789_081_200 },
    workflow: { path: '.github/workflows/release-qualification.yml', size: 100,
      sha256: 'c'.repeat(64) },
  });
  const architectureBytes = bytes({
    format: 'zryna.release-qualification-architecture.v1', status: 'provisional-candidate',
    productionAdmission: 'forbidden',
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
      commit: COMMIT, tree: TREE },
    command: ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'],
    toolchain: { channel: '1.97.1', cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)',
      cargoSha256: 'd'.repeat(64), rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)',
      rustcSha256: 'e'.repeat(64) },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map((logicalPath, index) => ({ logicalPath, size: 10 + index,
        sha256: String(index + 1).repeat(64) })),
    report: { diagnostics: [] },
  });
  const jobs = ['adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)'];
  const gateBytes = bytes({
    format: 'zryna.release-qualification-gates.v1', status: 'provisional-candidate',
    productionAdmission: 'forbidden', repository: 'https://github.com/zryna/zryna',
    sourceRef: 'refs/heads/main', workflow: '.github/workflows/ci.yml', runId: '123456789',
    runAttempt: 1, runUrl: 'https://github.com/zryna/zryna/actions/runs/123456789',
    sourceCommit: COMMIT, requiredContexts: jobs,
    requiredJobs: jobs.map((name, index) => ({ name, conclusion: 'success', sourceCommit: COMMIT,
      runId: '123456789', runAttempt: 1, jobId: String(100 + index),
      checkRunId: String(200 + index) })),
  });
  return { sourceBytes, architectureBytes, gateBytes, recipeBytes: RECIPE,
    target: 'x86_64-unknown-linux-gnu',
    capturedMaterials: [{ path: 'LICENSE', mode: 0o644, data: Buffer.from('license\n') }],
    tools: {
      toolchains: [
        tool('cargo', '1.97.1', 'repository:rust-toolchain.toml', 1),
        tool('node', '22.22.1', 'https://nodejs.org/dist/v22.22.1/', 3),
        tool('rustc', '1.97.1', 'repository:rust-toolchain.toml', 5),
      ],
      nativeTools: [tool('inspector', 'GNU readelf 2.42',
        'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-readelf', 7),
        tool('linker', 'gcc 13.2.0', 'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-gcc-13', 9),
        tool('xz', 'xz 5.6.1', 'ubuntu-24.04:/usr/bin/xz', 1)],
    } };
}

test('creates one closed provisional input from authenticated receipts, materials, and observations', () => {
  const value = createReleaseQualificationInput(fixture());
  assert.equal(value.productionAdmission, 'forbidden');
  assert.equal(value.recipeProposal.sha256, sha256(RECIPE));
  assert.equal(value.sourceReceipt.logicalPath, 'qualification/source.json');
  assert.equal(value.materials.files[0].material, 'source');
  assert.deepEqual(value.compile.encodedLinkerFlags, ['-Wl,--build-id=none']);
});

test('rejects receipt drift and production recipe admission', () => {
  const drift = fixture();
  const gates = JSON.parse(drift.gateBytes.toString('utf8'));
  gates.sourceCommit = 'f'.repeat(40);
  drift.gateBytes = bytes(gates);
  assert.throws(() => createReleaseQualificationInput(drift));

  const production = fixture();
  const recipe = JSON.parse(production.recipeBytes.toString('utf8'));
  recipe.productionAdmission = 'allowed';
  production.recipeBytes = bytes(recipe);
  assert.throws(() => createReleaseQualificationInput(production), /recipe proposal differs/);
});
