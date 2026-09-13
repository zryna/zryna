import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { join, resolve } from 'node:path';
import test from 'node:test';
import {
  canonicalBounded, parseCanonical, sha256,
} from '../scripts/distribution-release/canonical.mjs';
import { validateReleaseQualificationArchitecture } from
  '../scripts/distribution-release/validate-release-qualification-architecture.mjs';
import { createProductionProvisioner } from '../scripts/distribution/provision-release.mjs';

const COMMIT = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const TARGET = 'x86_64-unknown-linux-gnu';
const DIGEST = 'c'.repeat(64);

function wire(value) {
  return Buffer.from(`${canonicalBounded(value)}\n`);
}

function acceptedRecipe(overrides = {}) {
  const proposal = parseCanonical(readFileSync(new URL(
    '../scripts/distribution/release-recipe-v1.json', import.meta.url,
  ), 'utf8'));
  return {
    ...proposal,
    productionAdmission: 'allowed',
    status: 'production-accepted',
    ...overrides,
  };
}

function architectureReceipt() {
  return wire({
    format: 'zryna.source-build-receipt.v1',
    source: {
      repository: 'https://github.com/zryna/zryna', commit: COMMIT, tree: TREE,
    },
    command: ['cargo', 'run', '--locked', '-p', 'zryna', '--',
      'architecture', 'check', '--json'],
    toolchain: {
      channel: '1.97.1',
      cargoVersion: 'cargo 1.97.1 (c980f4866 2026-06-30)',
      cargoSha256: sha256(Buffer.from('cargo')),
      rustcVersion: 'rustc 1.97.1 (8bab26f4f 2026-07-14)',
      rustcSha256: sha256(Buffer.from('rustc')),
    },
    inputs: ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json']
      .map((logicalPath) => ({ logicalPath, size: 1, sha256: DIGEST })),
    report: { diagnostics: [] },
  });
}

function fixture() {
  const sourceRoot = resolve('source');
  const runnerTemp = resolve('runner-temp');
  const cargoHome = resolve('bootstrap-cargo-home');
  const source = {
    repository: 'https://github.com/zryna/zryna',
    ref: 'refs/tags/v0.2.1',
    commit: COMMIT,
    tree: TREE,
    sourceDateEpoch: 1234,
  };
  const target = {
    triple: TARGET,
    archiveFormat: 'tar-gzip',
    platformBaseline: {
      os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64',
    },
  };
  const recipe = acceptedRecipe();
  const recipeBytes = wire(recipe);
  const receipt = architectureReceipt();
  const expectedPrepared = {
    format: 'zryna.distribution.v1',
    logicalPath: 'prepared/metadata/distribution.json',
    archiveFileCount: 8,
    size: 101,
    sha256: DIGEST,
  };
  const paths = Object.fromEntries(['cargo', 'node', 'rustc', 'linker', 'inspector', 'xz']
    .map((name) => [name, resolve(`tools/${name}`)]));
  const contents = new Map(Object.entries({
    [paths.cargo]: Buffer.from('cargo'),
    [paths.node]: Buffer.from('node'),
    [paths.rustc]: Buffer.from('rustc'),
    [paths.linker]: Buffer.from('linker'),
    [paths.inspector]: Buffer.from('inspector'),
    [paths.xz]: Buffer.from('xz'),
  }));
  const record = (name, origin, evidence = DIGEST) => ({
    name,
    version: name === 'node' ? '22.22.1' : name === 'cargo' || name === 'rustc'
      ? '1.97.1' : `${name} version`,
    origin,
    size: contents.get(paths[name]).length,
    sha256: sha256(contents.get(paths[name])),
    observationEvidenceSha256: evidence,
  });
  const observed = {
    toolchains: [
      record('cargo', 'repository:rust-toolchain.toml'),
      record('node', 'https://nodejs.org/dist/v22.22.1/', 'd'.repeat(64)),
      record('rustc', 'repository:rust-toolchain.toml'),
    ],
    nativeTools: [
      record('inspector', 'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-readelf'),
      record('linker', 'ubuntu-24.04:/usr/bin/x86_64-linux-gnu-gcc-13'),
      record('xz', 'ubuntu-24.04:/usr/bin/xz'),
    ],
    paths,
    hostEnvironment: {},
  };
  const environment = {
    CARGO_HOME: cargoHome,
    GITHUB_RUN_ID: '77',
    GITHUB_SHA: COMMIT,
    RUNNER_TEMP: runnerTemp,
    ZRYNA_REPLICA: '1',
    ZRYNA_TARGET: TARGET,
  };
  const calls = { acquisition: 0, archive: 0, audit: 0, seed: 0, spawn: 0 };
  const executable = join(runnerTemp, `zryna-production-${TARGET}-77-1`,
    'target', TARGET, 'release', 'zryna');
  contents.set(executable, Buffer.alloc(64, 7));
  const system = {
    platform: 'linux',
    inspect(path) {
      const bytes = contents.get(path);
      return {
        isFile: () => bytes !== undefined,
        isDirectory: () => bytes === undefined,
        isSymbolicLink: () => false,
        size: bytes?.length ?? 0,
      };
    },
    realPath: (path) => path,
    read: (path) => Buffer.from(contents.get(path)),
    list: () => ['cargo-home'],
    make: () => {},
  };
  const provisioner = createProductionProvisioner({
    environment,
    system,
    observeTools({ architectureBytes }) {
      calls.architecture = validateReleaseQualificationArchitecture(
        parseCanonical(architectureBytes.toString('utf8')),
      );
      return observed;
    },
    createArchiveCapability() {
      calls.archive += 1;
      return { kind: 'authenticated-archive-capability' };
    },
    async acquireMaterials({ archiveCapability }) {
      calls.acquisition += 1;
      assert.equal(archiveCapability.kind, 'authenticated-archive-capability');
      return {
        capturedMaterials: [{ path: 'LICENSE', mode: 0o644, data: Buffer.from('license') }],
        rustCaptures: [{ identity: 'crate-1.0.0', archive: Buffer.from('crate') }],
      };
    },
    auditCompileSources(seededCargoHome, rustCaptures) {
      calls.audit += 1;
      assert.match(seededCargoHome, /cargo-home$/);
      assert.equal(rustCaptures[0].archive.toString(), 'crate');
    },
    preparePayload(identity, captured, observedReceipt) {
      assert.equal(identity.recipe.sha256, sha256(recipeBytes));
      assert.equal(captured[0].data.toString(), 'license');
      assert(observedReceipt.equals(receipt));
      return { preparedDistribution: expectedPrepared };
    },
    seedCargoHome({ bootstrapCargoHome, workRoot, rustCaptures }) {
      calls.seed += 1;
      assert.equal(bootstrapCargoHome, cargoHome);
      assert.equal(rustCaptures[0].archive.toString(), 'crate');
      return { workRoot, cargoHome: join(workRoot, 'cargo-home') };
    },
    spawn(command, args, options) {
      calls.spawn += 1;
      assert.equal(command, paths.cargo);
      assert.deepEqual(args.slice(0, 5), ['rustc', '--locked', '--release', '--target', TARGET]);
      assert.equal(options.shell, false);
      assert.equal(options.env.CARGO_NET_OFFLINE, 'true');
      assert.equal(options.env.ZRYNA_DISTRIBUTION_SHA256, DIGEST);
      assert.match(options.env.CARGO_ENCODED_RUSTFLAGS, /--remap-path-prefix=/);
      return { status: 0, signal: null, stdout: Buffer.alloc(0), stderr: Buffer.alloc(0) };
    },
  });
  return {
    calls, environment, expectedPrepared, provisioner, receipt, recipe, recipeBytes,
    source, sourceRoot, target,
  };
}

test('captures authenticated pinned materials and compiles offline with production identity',
  async () => {
    const value = fixture();
    const captured = await value.provisioner.captureReleaseMaterials({
      recipe: value.recipe,
      recipeBytes: value.recipeBytes,
      sourceRoot: value.sourceRoot,
      source: value.source,
      target: value.target,
      architectureReceipt: value.receipt,
    });
    assert.equal(captured[0].data.toString(), 'license');
    captured[0].data.fill(0);
    const compiled = await value.provisioner.compileReleaseCli({
      recipe: value.recipe,
      recipeBytes: value.recipeBytes,
      sourceRoot: value.sourceRoot,
      source: value.source,
      target: value.target,
      replica: 1,
      preparedDistribution: value.expectedPrepared,
    });
    assert.equal(compiled.cli.length, 64);
    assert.deepEqual(compiled.toolchains.map(({ name }) => name), ['cargo', 'node', 'rustc']);
    assert.equal(compiled.toolchains[0].signatureEvidenceSha256, sha256(value.receipt));
    assert.equal(compiled.toolchains[1].signatureEvidenceSha256, 'd'.repeat(64));
    assert.deepEqual(value.calls, {
      acquisition: 1,
      archive: 1,
      architecture: value.calls.architecture,
      audit: 1,
      seed: 1,
      spawn: 1,
    });
  });

test('rejects changed prepared distribution before creating a compile root', async () => {
  const value = fixture();
  await value.provisioner.captureReleaseMaterials({
    recipe: value.recipe,
    recipeBytes: value.recipeBytes,
    sourceRoot: value.sourceRoot,
    source: value.source,
    target: value.target,
    architectureReceipt: value.receipt,
  });
  await assert.rejects(() => value.provisioner.compileReleaseCli({
    recipe: value.recipe,
    recipeBytes: value.recipeBytes,
    sourceRoot: value.sourceRoot,
    source: value.source,
    target: value.target,
    replica: 1,
    preparedDistribution: { ...value.expectedPrepared, sha256: 'e'.repeat(64) },
  }), /D422-PROVISION: prepared distribution differs from authenticated materials/);
  assert.equal(value.calls.seed, 0);
  assert.equal(value.calls.spawn, 0);
});

test('rejects duplicate or cross-source use of a pending authenticated capture', async () => {
  const value = fixture();
  const options = {
    recipe: value.recipe,
    recipeBytes: value.recipeBytes,
    sourceRoot: value.sourceRoot,
    source: value.source,
    target: value.target,
    architectureReceipt: value.receipt,
  };
  await value.provisioner.captureReleaseMaterials(options);
  await assert.rejects(() => value.provisioner.captureReleaseMaterials(options),
    /D422-PROVISION: production material capture is already pending/);
  await assert.rejects(() => value.provisioner.compileReleaseCli({
    ...options,
    source: { ...value.source, tree: 'f'.repeat(40) },
    replica: 1,
    preparedDistribution: value.expectedPrepared,
  }), /D422-PROVISION: compile does not match one authenticated material capture/);
});
