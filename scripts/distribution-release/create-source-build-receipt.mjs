import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import {
  closeSync, fstatSync, lstatSync, openSync, readSync, realpathSync, statSync,
} from 'node:fs';
import {
  dirname, isAbsolute, join, relative, resolve, sep,
} from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { validateReleaseQualificationArchitecture } from './validate-release-qualification-architecture.mjs';
import { validateSourceBuildReceipt } from './validate-source-build-receipt.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPOSITORY = 'https://github.com/zryna/zryna';
const INPUT_PATHS = ['Cargo.lock', 'Cargo.toml', 'rust-toolchain.toml', 'zryna.workspace.json'];
const COMMAND = ['cargo', 'run', '--locked', '-p', 'zryna', '--', 'architecture', 'check', '--json'];
const MAX_OUTPUT = 256 * 1024;
const MAX_EXECUTABLE = 256 * 1024 * 1024;
const DIGEST = /^[0-9a-f]{64}$/;
const CARGO_VERSION = 'cargo 1.97.1 (c980f4866 2026-06-30)';
const RUSTC_VERSION = 'rustc 1.97.1 (8bab26f4f 2026-07-14)';

function reject(message) {
  throw new Error(`R406-ARCH-PRODUCER: ${message}`);
}

function run(spawn, executable, args, cwd, encoding = 'utf8', environment) {
  const result = spawn(executable, args, {
    cwd, encoding, env: environment, maxBuffer: MAX_OUTPUT + 1, shell: false, windowsHide: true,
  });
  if (result.error || result.status !== 0) {
    reject(`${executable} ${args.join(' ')} failed with status ${result.status ?? 'unknown'}`);
  }
  const output = result.stdout;
  if ((Buffer.isBuffer(output) ? output.length : Buffer.byteLength(output, 'utf8')) > MAX_OUTPUT) {
    reject(`${executable} output exceeds ${MAX_OUTPUT} bytes`);
  }
  return output;
}

function hashRegularFile(path) {
  const metadata = statSync(path);
  if (!metadata.isFile() || metadata.size < 1 || metadata.size > MAX_EXECUTABLE) {
    reject(`tool executable must be a regular file of at most ${MAX_EXECUTABLE} bytes`);
  }
  const descriptor = openSync(path, 'r');
  const hash = createHash('sha256');
  const buffer = Buffer.allocUnsafe(64 * 1024);
  let offset = 0;
  try {
    while (offset < metadata.size) {
      const count = readSync(descriptor, buffer, 0, Math.min(buffer.length, metadata.size - offset), offset);
      if (count === 0) reject('tool executable changed during hashing');
      hash.update(buffer.subarray(0, count));
      offset += count;
    }
    const after = fstatSync(descriptor);
    if (after.size !== metadata.size || after.mtimeMs !== metadata.mtimeMs) {
      reject('tool executable changed during hashing');
    }
  } finally {
    closeSync(descriptor);
  }
  return hash.digest('hex');
}

function samePath(left, right) {
  const normalize = (path) => (process.platform === 'win32' ? resolve(path).toLowerCase() : resolve(path));
  return normalize(left) === normalize(right);
}

function inspectActualExecutable(path) {
  const metadata = lstatSync(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || !samePath(realpathSync.native(path), path)) {
    reject('tool path must name a regular non-link executable without reparse aliases');
  }
}

function exactExecutable({
  environment, name, rustupPath, rustupHome, spawn, cwd, hashFile, inspectFile,
}) {
  const variable = `ZRYNA_${name.toUpperCase()}_PATH`;
  const digestVariable = `ZRYNA_${name.toUpperCase()}_SHA256`;
  const path = environment[variable];
  const expected = environment[digestVariable];
  if (!isAbsolute(path ?? '') || !DIGEST.test(expected ?? '')) {
    reject(`${variable} and ${digestVariable} must bind one absolute executable`);
  }
  const selected = run(
    spawn,
    rustupPath,
    ['which', '--toolchain', '1.97.1', name],
    cwd,
    'utf8',
    environment,
  ).trim();
  if (!isAbsolute(selected) || !samePath(selected, path)) {
    reject(`${name} path differs from rustup's exact 1.97.1 selection`);
  }
  const toolchainRelative = relative(join(rustupHome, 'toolchains'), resolve(path));
  const segments = toolchainRelative.split(sep);
  const executableName = `${name}${process.platform === 'win32' ? '.exe' : ''}`;
  if (segments.length !== 3 || !segments[0].startsWith('1.97.1-')
    || segments[1] !== 'bin' || segments[2] !== executableName) {
    reject(`${name} path is not the actual 1.97.1 toolchain executable`);
  }
  let observed;
  try {
    inspectFile(path);
    observed = hashFile(path);
  } catch (error) {
    if (error?.message?.startsWith('R406-ARCH-PRODUCER:')) throw error;
    reject(`${name} executable could not be read`);
  }
  if (observed !== expected) reject(`${name} executable differs from its admitted digest`);
  return { path, sha256: observed };
}

function rejectCargoConfig(directory, label) {
  for (const filename of ['config', 'config.toml']) {
    try {
      if (statSync(join(directory, filename)).isFile()) {
        reject(`${label} must not contain Cargo configuration`);
      }
    } catch (error) {
      if (error?.code !== 'ENOENT') throw error;
    }
  }
}

function createArchitectureObservation({
  environment = process.env,
  cwd = environment.ZRYNA_SOURCE_ROOT,
  spawn = spawnSync,
  hashFile = hashRegularFile,
  inspectFile = inspectActualExecutable,
  inspectCargoConfig = rejectCargoConfig,
} = {}) {
  if (!isAbsolute(cwd ?? '')) reject('ZRYNA_SOURCE_ROOT must name an absolute isolated checkout');
  if (!isAbsolute(environment.ZRYNA_RUSTUP_PATH ?? '')
    || !isAbsolute(environment.RUSTUP_HOME ?? '')) {
    reject('ZRYNA_RUSTUP_PATH and RUSTUP_HOME must be absolute controlled paths');
  }
  const commit = run(spawn, 'git', ['rev-parse', 'HEAD'], cwd).trim();
  const tree = run(spawn, 'git', ['show', '-s', '--format=%T', 'HEAD'], cwd).trim();
  const topLevel = run(spawn, 'git', ['rev-parse', '--show-toplevel'], cwd).trim();
  if (commit !== environment.GITHUB_SHA || !/^[0-9a-f]{40}$/.test(tree)) {
    reject('checked-out commit or tree differs from the workflow source');
  }
  if (!samePath(topLevel, cwd)) reject('ZRYNA_SOURCE_ROOT must be the checkout root');
  if (run(spawn, 'git', [
    'status', '--porcelain=v1', '--untracked-files=all', '--ignored=matching',
  ], cwd).length !== 0) {
    reject('checkout contains tracked, untracked, or ignored files outside the source tree');
  }
  const cargoHome = environment.CARGO_HOME;
  if (!isAbsolute(cargoHome ?? '')) reject('CARGO_HOME must be an absolute controlled directory');
  inspectCargoConfig(cargoHome, 'controlled CARGO_HOME');
  let ancestor = dirname(resolve(cwd));
  while (true) {
    inspectCargoConfig(join(ancestor, '.cargo'), 'checkout ancestor');
    const parent = dirname(ancestor);
    if (parent === ancestor) break;
    ancestor = parent;
  }
  const toolOptions = {
    environment,
    rustupPath: environment.ZRYNA_RUSTUP_PATH,
    rustupHome: environment.RUSTUP_HOME,
    spawn,
    cwd,
    hashFile,
    inspectFile,
  };
  const cargo = exactExecutable({ ...toolOptions, name: 'cargo' });
  const rustc = exactExecutable({ ...toolOptions, name: 'rustc' });
  if (cargo.sha256 === rustc.sha256) reject('cargo and rustc executable identities must differ');
  const inputs = INPUT_PATHS.map((logicalPath) => {
    const bytes = run(spawn, 'git', ['show', `${commit}:${logicalPath}`], cwd, null);
    return { logicalPath, size: bytes.length, sha256: sha256(bytes) };
  });
  const buildEnvironment = Object.fromEntries(Object.entries(environment).filter(([name]) => (
    !name.startsWith('CARGO_')
    && !name.startsWith('RUST')
  )));
  Object.assign(buildEnvironment, {
    CARGO_BUILD_JOBS: '3',
    CARGO_HOME: cargoHome,
    CARGO_NET_OFFLINE: 'true',
    RUSTC: rustc.path,
    RUSTUP_TOOLCHAIN: '1.97.1',
  });
  const cargoVersion = run(spawn, cargo.path, ['--version'], cwd, 'utf8', buildEnvironment).trim();
  const rustcVersion = run(spawn, rustc.path, ['--version'], cwd, 'utf8', buildEnvironment).trim();
  if (cargoVersion !== CARGO_VERSION || rustcVersion !== RUSTC_VERSION) {
    reject('cargo and rustc full version outputs differ from the release pin');
  }
  const reportText = run(spawn, cargo.path, COMMAND.slice(1), cwd, 'utf8', buildEnvironment);
  let report;
  try {
    report = JSON.parse(reportText);
  } catch {
    reject('architecture command did not emit JSON');
  }
  if (JSON.stringify(report) !== '{"diagnostics":[]}') {
    reject('architecture command did not emit the exact successful report');
  }
  try {
    inspectFile(cargo.path);
    inspectFile(rustc.path);
    if (hashFile(cargo.path) !== cargo.sha256 || hashFile(rustc.path) !== rustc.sha256) {
      reject('tool executable changed during the architecture command');
    }
  } catch (error) {
    if (error?.message?.startsWith('R406-ARCH-PRODUCER:')) throw error;
    reject('tool executable could not be reread after the architecture command');
  }
  return {
    commit,
    tree,
    command: COMMAND,
    toolchain: {
      channel: '1.97.1',
      cargoVersion,
      cargoSha256: cargo.sha256,
      rustcVersion,
      rustcSha256: rustc.sha256,
    },
    inputs,
    report,
  };
}

export function createSourceBuildReceipt(options = {}) {
  const environment = options.environment ?? process.env;
  if (environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== 'refs/tags/v0.2.0'
    || !/^[0-9a-f]{40}$/.test(environment.GITHUB_SHA ?? '')) {
    reject('exact protected repository, tag, and source SHA are required');
  }
  const { commit, tree, ...observation } = createArchitectureObservation({
    ...options, environment,
  });
  return validateSourceBuildReceipt({
    format: 'zryna.source-build-receipt.v1',
    source: { repository: REPOSITORY, commit, tree },
    ...observation,
  });
}

export function createQualificationArchitectureReceipt(options = {}) {
  const environment = options.environment ?? process.env;
  if (environment.GITHUB_EVENT_NAME !== 'workflow_dispatch'
    || environment.GITHUB_REPOSITORY !== 'zryna/zryna'
    || environment.GITHUB_REF !== 'refs/heads/main'
    || environment.GITHUB_REF_TYPE !== 'branch'
    || environment.GITHUB_REF_NAME !== 'main'
    || environment.GITHUB_REF_PROTECTED !== 'true'
    || !/^[0-9a-f]{40}$/.test(environment.GITHUB_SHA ?? '')
    || environment.GITHUB_WORKFLOW_SHA !== environment.GITHUB_SHA
    || environment.GITHUB_WORKFLOW_REF !==
      'zryna/zryna/.github/workflows/release-qualification.yml@refs/heads/main') {
    reject('exact protected qualification workflow context is required');
  }
  const { commit, tree, ...observation } = createArchitectureObservation({
    ...options, environment,
  });
  return validateReleaseQualificationArchitecture({
    format: 'zryna.release-qualification-architecture.v1',
    status: 'provisional-candidate',
    productionAdmission: 'forbidden',
    source: {
      repository: REPOSITORY, ref: 'refs/heads/main', commit, tree,
    },
    ...observation,
  });
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    process.stdout.write(`${canonicalBounded(createSourceBuildReceipt())}\n`);
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
