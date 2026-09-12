import { spawnSync } from 'node:child_process';
import {
  lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { sha256 } from './canonical.mjs';

const MAX_TOOL = 536870912;
const MAX_CRATE = 64 * 1024 * 1024;
const MAX_OUTPUT = 16 * 1024 * 1024;

function reject(message) {
  throw new Error(`R406-QUALIFICATION-CARGO: ${message}`);
}

function samePath(left, right) {
  const normalize = (value) => process.platform === 'win32'
    ? resolve(value).toLowerCase() : resolve(value);
  return normalize(left) === normalize(right);
}

function regular(path, expected, system) {
  const metadata = system.inspect(path);
  if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size < 1
    || metadata.size > (expected.maximum ?? MAX_CRATE)
    || !samePath(system.realPath(path), path)) reject('Cargo material file identity differs');
  const bytes = system.read(path);
  if (bytes.length !== metadata.size || (expected.size !== undefined && bytes.length !== expected.size)
    || sha256(bytes) !== expected.sha256) reject('Cargo material bytes differ');
  return bytes;
}

export function provisionQualificationCargoHome({
  sourceRoot, workRoot, target, environment = process.env, spawn = spawnSync,
  system = { inspect: lstatSync, realPath: realpathSync.native, read: readFileSync,
    list: readdirSync, make: mkdirSync },
}) {
  const cargo = environment.ZRYNA_CARGO_PATH;
  const cargoDigest = environment.ZRYNA_CARGO_SHA256;
  const expectedPlatform = target === 'x86_64-pc-windows-msvc' ? 'win32' : 'linux';
  if (![sourceRoot, workRoot, cargo].every(isAbsolute) || samePath(sourceRoot, workRoot)
    || !['x86_64-unknown-linux-gnu', 'x86_64-pc-windows-msvc'].includes(target)
    || process.platform !== expectedPlatform
    || !/^[0-9a-f]{64}$/.test(cargoDigest ?? '')) reject('Cargo provision inputs differ');
  const source = system.inspect(sourceRoot);
  if (!source.isDirectory() || source.isSymbolicLink()
    || !samePath(system.realPath(sourceRoot), sourceRoot)) reject('Cargo source root differs');
  regular(cargo, { sha256: cargoDigest, maximum: MAX_TOOL }, system);
  system.make(workRoot, { recursive: false, mode: 0o700 });
  const cargoHome = join(workRoot, 'cargo-home');
  system.make(cargoHome, { recursive: false, mode: 0o700 });
  const fetchEnvironment = {
    CARGO_HOME: cargoHome,
    CARGO_HTTP_TIMEOUT: '30',
    CARGO_NET_OFFLINE: 'false',
    CARGO_NET_RETRY: '0',
    LANG: 'C',
    LC_ALL: 'C',
    PATH: environment.ZRYNA_QUALIFICATION_FETCH_PATH,
    RUSTC: environment.ZRYNA_RUSTC_PATH,
    SOURCE_DATE_EPOCH: environment.SOURCE_DATE_EPOCH,
    TZ: 'UTC',
  };
  if (process.platform === 'win32') {
    delete fetchEnvironment.LC_ALL;
    fetchEnvironment.SystemRoot = environment.SystemRoot;
  }
  if (Object.values(fetchEnvironment).some((value) => typeof value !== 'string'
    || value.length < 1 || /[\r\n\0]/.test(value))) reject('Cargo fetch environment differs');
  const result = spawn(cargo, ['fetch', '--locked', '--target', target], {
    cwd: sourceRoot, env: fetchEnvironment, encoding: null, maxBuffer: MAX_OUTPUT + 1,
    timeout: 10 * 60_000, windowsHide: true, shell: false,
  });
  regular(cargo, { sha256: cargoDigest, maximum: MAX_TOOL }, system);
  if (result.error || result.status !== 0 || result.signal !== null
    || !Buffer.isBuffer(result.stdout) || !Buffer.isBuffer(result.stderr)
    || result.stdout.length + result.stderr.length > MAX_OUTPUT) {
    reject(`Cargo fetch failed with status ${result.status ?? 'unknown'}`);
  }
  return { workRoot, cargoHome };
}

export function auditQualificationCargoCache(cargoHome, rustCaptures, {
  system = { inspect: lstatSync, realPath: realpathSync.native, read: readFileSync,
    list: readdirSync },
} = {}) {
  if (!isAbsolute(cargoHome ?? '') || !Array.isArray(rustCaptures) || rustCaptures.length < 1) {
    reject('Cargo cache audit inputs differ');
  }
  const cacheRoot = join(cargoHome, 'registry', 'cache');
  const entries = system.list(cacheRoot);
  const roots = entries.filter((name) => /^index\.crates\.io-[0-9a-f]{16}$/.test(name));
  if (roots.length !== 1 || entries.length !== 1) {
    reject('Cargo registry cache root is missing or ambiguous');
  }
  const root = join(cacheRoot, roots[0]);
  const metadata = system.inspect(root);
  if (!metadata.isDirectory() || metadata.isSymbolicLink() || !samePath(system.realPath(root), root)) {
    reject('Cargo registry cache directory differs');
  }
  const names = new Set();
  for (const capture of rustCaptures) {
    if (!capture || !/^[a-z0-9_-]+-[0-9]+\.[0-9]+\.[0-9]+(?:[A-Za-z0-9.+-]*)$/.test(capture.identity)
      || !Buffer.isBuffer(capture.archive) || capture.archive.length < 1
      || capture.archive.length > MAX_CRATE || names.has(capture.identity)) {
      reject('Cargo crate capture differs');
    }
    names.add(capture.identity);
    const filename = `${capture.identity}.crate`;
    if (basename(filename) !== filename) reject('Cargo crate filename differs');
    const cached = regular(join(root, filename), {
      size: capture.archive.length, sha256: sha256(capture.archive), maximum: MAX_CRATE,
    }, system);
    if (!cached.equals(capture.archive)) reject('Cargo cached archive differs from its capture');
  }
}
