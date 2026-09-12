import assert from 'node:assert/strict';
import { resolve } from 'node:path';
import test from 'node:test';
import {
  createPreassemblyGates, createReleaseQualificationGates,
} from '../scripts/distribution-release/create-preassembly-gates.mjs';
import {
  createQualificationArchitectureReceipt,
  createSourceBuildReceipt,
} from '../scripts/distribution-release/create-source-build-receipt.mjs';

const COMMIT = 'a'.repeat(40);
const TREE = 'b'.repeat(40);
const SOURCE_ROOT = resolve('test-source');
const RUSTUP_HOME = resolve('test-toolchain', 'rustup-home');
const TOOLCHAIN_ROOT = resolve(RUSTUP_HOME, 'toolchains', '1.97.1-test-host');
const suffix = process.platform === 'win32' ? '.exe' : '';
const CARGO = resolve(TOOLCHAIN_ROOT, 'bin', `cargo${suffix}`);
const RUSTC = resolve(TOOLCHAIN_ROOT, 'bin', `rustc${suffix}`);
const RUSTUP = resolve('test-toolchain', `rustup${suffix}`);
const CARGO_HOME = resolve('test-toolchain', 'cargo-home');
const CARGO_SHA256 = 'c'.repeat(64);
const RUSTC_SHA256 = 'd'.repeat(64);
const REQUIRED = [
  'adapter', 'm0', 'm2', 'm3', 'rust (ubuntu-latest)', 'rust (windows-latest)',
];
const environment = {
  GITHUB_REPOSITORY: 'zryna/zryna',
  GITHUB_SERVER_URL: 'https://github.com',
  GITHUB_REF: 'refs/tags/v0.2.0',
  GITHUB_SHA: COMMIT,
  GITHUB_TOKEN: 'test-token',
  CARGO_HOME,
  RUSTUP_HOME,
  ZRYNA_SOURCE_ROOT: SOURCE_ROOT,
  ZRYNA_CARGO_PATH: CARGO,
  ZRYNA_CARGO_SHA256: CARGO_SHA256,
  ZRYNA_RUSTC_PATH: RUSTC,
  ZRYNA_RUSTC_SHA256: RUSTC_SHA256,
  ZRYNA_RUSTUP_PATH: RUSTUP,
};

function spawnFixture(executable, args, options) {
  let stdout;
  if (executable === 'git' && args[0] === 'rev-parse' && args[1] === 'HEAD') stdout = `${COMMIT}\n`;
  else if (executable === 'git' && args[0] === 'rev-parse') stdout = `${SOURCE_ROOT}\n`;
  else if (executable === 'git' && args[0] === 'show' && args[1] === '-s') stdout = `${TREE}\n`;
  else if (executable === 'git' && args[0] === 'status') {
    assert.deepEqual(args, [
      'status', '--porcelain=v1', '--untracked-files=all', '--ignored=matching',
    ]);
    stdout = '';
  }
  else if (executable === 'git' && args[0] === 'show') stdout = Buffer.from(args[1]);
  else if (executable === RUSTUP && args[0] === 'which') {
    stdout = `${args.at(-1) === 'cargo' ? CARGO : RUSTC}\n`;
  } else if (executable === CARGO && args[0] === '--version') {
    stdout = 'cargo 1.97.1 (c980f4866 2026-06-30)\n';
  } else if (executable === RUSTC) {
    stdout = 'rustc 1.97.1 (8bab26f4f 2026-07-14)\n';
  } else if (executable === CARGO && args[0] === 'run') stdout = '{"diagnostics":[]}\n';
  else throw new Error(`unexpected command ${executable} ${args.join(' ')}`);
  if (options.encoding === null && !Buffer.isBuffer(stdout)) stdout = Buffer.from(stdout);
  return { status: 0, stdout, stderr: options.encoding === null ? Buffer.alloc(0) : '' };
}

function response(value, status = 200, {
  declaredLength, chunkSize = 64, contentEncoding,
} = {}) {
  const bytes = new TextEncoder().encode(JSON.stringify(value));
  let offset = 0;
  return {
    ok: status >= 200 && status < 300,
    status,
    headers: {
      get: (name) => {
        if (name === 'content-length') return String(declaredLength ?? bytes.byteLength);
        if (name === 'content-encoding') return contentEncoding ?? null;
        return null;
      },
    },
    body: {
      getReader: () => ({
        read: async () => {
          if (offset === bytes.byteLength) return { done: true };
          const valueBytes = bytes.subarray(offset, offset + chunkSize);
          offset += valueBytes.byteLength;
          return { done: false, value: valueBytes };
        },
        cancel: async () => {},
      }),
    },
  };
}

function githubFixture({ wrongRun = false } = {}) {
  const run = {
    id: 123456789, run_attempt: 2, repository: { full_name: 'zryna/zryna' },
    path: '.github/workflows/ci.yml', event: 'workflow_dispatch', head_branch: 'main',
    head_sha: COMMIT, status: 'completed', conclusion: 'success',
    html_url: 'https://github.com/zryna/zryna/actions/runs/123456789',
  };
  const jobs = REQUIRED.map((name, index) => ({
    id: 200 + index,
    run_id: wrongRun && index === 0 ? 123456788 : 123456789,
    head_sha: COMMIT,
    status: 'completed',
    conclusion: 'success',
    name,
    check_run_url: `https://api.github.com/repos/zryna/zryna/check-runs/${300 + index}`,
    workflow_name: 'CI',
  }));
  return async (url, options) => {
    assert.equal(options.headers.Authorization, 'Bearer test-token');
    assert.equal(options.headers['Accept-Encoding'], 'identity');
    if (url.includes('/actions/workflows/ci.yml/runs?')) {
      return response({ total_count: 1, workflow_runs: [run] });
    }
    if (url.endsWith('/branches/main/protection/required_status_checks')) {
      return response({ contexts: REQUIRED, checks: [] });
    }
    if (url.includes('/actions/runs/123456789/attempts/2/jobs?')) {
      return response({ total_count: jobs.length, jobs });
    }
    throw new Error(`unexpected URL ${url}`);
  };
}

test('architecture producer runs the exact command with digest-bound tools on exact clean source', () => {
  const hashFile = (path) => ({ [CARGO]: CARGO_SHA256, [RUSTC]: RUSTC_SHA256 })[path];
  const receipt = createSourceBuildReceipt({
    environment,
    spawn: spawnFixture,
    hashFile,
    inspectFile: () => {},
    inspectCargoConfig: () => {},
    cwd: SOURCE_ROOT,
  });
  assert.deepEqual(receipt.source, {
    repository: 'https://github.com/zryna/zryna', commit: COMMIT, tree: TREE,
  });
  assert.deepEqual(receipt.report, { diagnostics: [] });
  assert.equal(receipt.inputs.length, 4);
  assert.equal(receipt.toolchain.cargoSha256, CARGO_SHA256);
  assert.equal(receipt.toolchain.rustcSha256, RUSTC_SHA256);
});

test('architecture producer rejects untracked or ignored checkout material', () => {
  const dirtySpawn = (executable, args, options) => {
    if (executable === 'git' && args[0] === 'status') {
      return { status: 0, stdout: '!! .cargo/config.toml\n', stderr: '' };
    }
    return spawnFixture(executable, args, options);
  };
  assert.throws(
    () => createSourceBuildReceipt({ environment, spawn: dirtySpawn, cwd: SOURCE_ROOT }),
    /R406-ARCH-PRODUCER: checkout contains tracked, untracked, or ignored files/,
  );
});

test('architecture producer rejects a tool path outside rustup exact selection', () => {
  const proxy = resolve(CARGO_HOME, `cargo${suffix}`);
  assert.throws(
    () => createSourceBuildReceipt({
      environment: { ...environment, ZRYNA_CARGO_PATH: proxy },
      spawn: spawnFixture,
      hashFile: () => CARGO_SHA256,
      inspectFile: () => {},
      inspectCargoConfig: () => {},
      cwd: SOURCE_ROOT,
    }),
    /R406-ARCH-PRODUCER: cargo path differs from rustup's exact 1.97.1 selection/,
  );
});

test('qualification architecture producer reuses the observation under a closed non-production identity', () => {
  const qualificationEnvironment = {
    ...environment,
    GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_REF: 'refs/heads/main',
    GITHUB_REF_TYPE: 'branch',
    GITHUB_REF_NAME: 'main',
    GITHUB_REF_PROTECTED: 'true',
    GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main',
  };
  const hashFile = (path) => ({ [CARGO]: CARGO_SHA256, [RUSTC]: RUSTC_SHA256 })[path];
  const receipt = createQualificationArchitectureReceipt({
    environment: qualificationEnvironment,
    spawn: spawnFixture,
    hashFile,
    inspectFile: () => {},
    inspectCargoConfig: () => {},
    cwd: SOURCE_ROOT,
  });
  assert.equal(receipt.format, 'zryna.release-qualification-architecture.v1');
  assert.equal(receipt.status, 'provisional-candidate');
  assert.equal(receipt.productionAdmission, 'forbidden');
  assert.deepEqual(receipt.source, {
    repository: 'https://github.com/zryna/zryna', ref: 'refs/heads/main',
    commit: COMMIT, tree: TREE,
  });
  assert.deepEqual(receipt.report, { diagnostics: [] });
});

test('qualification architecture producer rejects a tag or mutable workflow context', () => {
  const base = {
    ...environment,
    GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_REF: 'refs/heads/main',
    GITHUB_REF_TYPE: 'branch',
    GITHUB_REF_NAME: 'main',
    GITHUB_REF_PROTECTED: 'true',
    GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main',
  };
  for (const changed of [
    { GITHUB_REF: 'refs/tags/v0.2.0' },
    { GITHUB_REF_PROTECTED: 'false' },
    { GITHUB_WORKFLOW_SHA: 'f'.repeat(40) },
    { GITHUB_WORKFLOW_REF: 'zryna/zryna/.github/workflows/release.yml@refs/heads/main' },
  ]) {
    assert.throws(() => createQualificationArchitectureReceipt({
      environment: { ...base, ...changed }, spawn: assert.fail, cwd: SOURCE_ROOT,
    }), /R406-ARCH-PRODUCER: exact protected qualification workflow context is required/);
  }
});

test('gate producer selects one live run and binds every job to its attempt', async () => {
  const receipt = await createPreassemblyGates({ environment, fetchImpl: githubFixture() });
  assert.deepEqual(receipt.requiredContexts, REQUIRED);
  assert.equal(receipt.runId, '123456789');
  assert(receipt.requiredJobs.every((job) => job.runId === '123456789' && job.runAttempt === 2));
});

test('qualification gate producer binds the same protected CI observation to main only', async () => {
  const qualificationEnvironment = {
    ...environment,
    GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_REF: 'refs/heads/main',
    GITHUB_REF_TYPE: 'branch',
    GITHUB_REF_NAME: 'main',
    GITHUB_REF_PROTECTED: 'true',
    GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main',
  };
  const receipt = await createReleaseQualificationGates({
    environment: qualificationEnvironment, fetchImpl: githubFixture(),
  });
  assert.equal(receipt.format, 'zryna.release-qualification-gates.v1');
  assert.equal(receipt.status, 'provisional-candidate');
  assert.equal(receipt.productionAdmission, 'forbidden');
  assert.equal(receipt.sourceRef, 'refs/heads/main');
  assert.equal(receipt.sourceCommit, COMMIT);
});

test('qualification gate producer rejects a tag or unprotected main context before fetching', async () => {
  const base = {
    ...environment,
    GITHUB_EVENT_NAME: 'workflow_dispatch',
    GITHUB_REF: 'refs/heads/main',
    GITHUB_REF_TYPE: 'branch',
    GITHUB_REF_NAME: 'main',
    GITHUB_REF_PROTECTED: 'true',
    GITHUB_WORKFLOW_SHA: COMMIT,
    GITHUB_WORKFLOW_REF:
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main',
  };
  for (const changed of [
    { GITHUB_REF: 'refs/tags/v0.2.0' },
    { GITHUB_REF_PROTECTED: 'false' },
    { GITHUB_WORKFLOW_SHA: 'f'.repeat(40) },
  ]) {
    await assert.rejects(() => createReleaseQualificationGates({
      environment: { ...base, ...changed }, fetchImpl: assert.fail,
    }), /R406-GATES-PRODUCER: exact protected main qualification workflow context/);
  }
});

test('gate producer rejects jobs outside the selected attempt endpoint run', async () => {
  await assert.rejects(
    createPreassemblyGates({ environment, fetchImpl: githubFixture({ wrongRun: true }) }),
    /R406-GATES-PRODUCER: required job adapter differs from the selected attempt/,
  );
});

test('gate producer rejects oversized declared and streamed responses', async () => {
  const declared = async () => response({}, 200, { declaredLength: 1024 * 1024 + 1 });
  await assert.rejects(
    createPreassemblyGates({ environment, fetchImpl: declared }),
    /R406-GATES-PRODUCER: GitHub API .* exceeded 1048576 bytes/,
  );

  const streamed = async () => {
    const bytes = new Uint8Array(1024 * 1024 + 1);
    let delivered = false;
    return {
      ok: true,
      status: 200,
      headers: { get: () => null },
      body: { getReader: () => ({
        read: async () => {
          if (delivered) return { done: true };
          delivered = true;
          return { done: false, value: bytes };
        },
        cancel: async () => {},
      }) },
    };
  };
  await assert.rejects(
    createPreassemblyGates({ environment, fetchImpl: streamed }),
    /R406-GATES-PRODUCER: GitHub API .* exceeded 1048576 bytes/,
  );
});

test('gate producer rejects encoded responses before comparing stream length', async () => {
  const encoded = async () => response({}, 200, { contentEncoding: 'gzip' });
  await assert.rejects(
    createPreassemblyGates({ environment, fetchImpl: encoded }),
    /R406-GATES-PRODUCER: GitHub API .* returned encoded content/,
  );
});

test('gate producer bounds request time', async () => {
  const pending = async () => new Promise(() => {});
  await assert.rejects(
    createPreassemblyGates({ environment, fetchImpl: pending, requestTimeoutMs: 5 }),
    /R406-GATES-PRODUCER: GitHub API .* timed out/,
  );
});
