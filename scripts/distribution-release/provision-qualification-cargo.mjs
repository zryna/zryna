import { spawnSync } from 'node:child_process';
import {
  lstatSync, mkdirSync, readFileSync, readdirSync, realpathSync, writeFileSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { canonical, sha256 } from './canonical.mjs';

const MAX_TOOL = 536870912;
const MAX_CRATE = 64 * 1024 * 1024;
const MAX_OUTPUT = 16 * 1024 * 1024;
const MAX_INDEX_FILE = 4 * 1024 * 1024;
const MAX_INDEX_TOTAL = 64 * 1024 * 1024;
const MAX_INDEX_ENTRIES = 4096;

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
  }
  const expectedNames = [...names].map((identity) => `${identity}.crate`).sort();
  const actualNames = system.list(root).sort();
  if (canonical(actualNames) !== canonical(expectedNames)) {
    reject('Cargo registry cache inventory differs from its captures');
  }
  for (const capture of rustCaptures) {
    const filename = `${capture.identity}.crate`;
    if (basename(filename) !== filename) reject('Cargo crate filename differs');
    const cached = regular(join(root, filename), {
      size: capture.archive.length, sha256: sha256(capture.archive), maximum: MAX_CRATE,
    }, system);
    if (!cached.equals(capture.archive)) reject('Cargo cached archive differs from its capture');
  }
}

function copyIndexTree(source, target, state) {
  const sourceMetadata = lstatSync(source);
  if (!sourceMetadata.isDirectory() || sourceMetadata.isSymbolicLink()
    || !samePath(realpathSync.native(source), source)) reject('Cargo index directory differs');
  mkdirSync(target, { recursive: false, mode: 0o700 });
  for (const entry of readdirSync(source, { withFileTypes: true })) {
    state.entries += 1;
    if (state.entries > MAX_INDEX_ENTRIES || !/^[A-Za-z0-9._+-]{1,160}$/.test(entry.name)) {
      reject('Cargo index metadata inventory differs');
    }
    const sourcePath = join(source, entry.name);
    const targetPath = join(target, entry.name);
    const metadata = lstatSync(sourcePath);
    if (entry.isDirectory() && metadata.isDirectory() && !metadata.isSymbolicLink()) {
      copyIndexTree(sourcePath, targetPath, state);
    } else if (entry.isFile() && metadata.isFile() && !metadata.isSymbolicLink()
      && metadata.size > 0 && metadata.size <= MAX_INDEX_FILE
      && samePath(realpathSync.native(sourcePath), sourcePath)) {
      const bytes = readFileSync(sourcePath);
      state.total += bytes.length;
      if (bytes.length !== metadata.size || state.total > MAX_INDEX_TOTAL) {
        reject('Cargo index metadata exceeds its bound');
      }
      writeFileSync(targetPath, bytes, { flag: 'wx', mode: 0o600 });
    } else {
      reject('Cargo index metadata file identity differs');
    }
  }
}

export function seedQualificationCompileCargoHome({ bootstrapCargoHome, workRoot, rustCaptures }) {
  if (![bootstrapCargoHome, workRoot].every((path) => isAbsolute(path ?? ''))
    || samePath(bootstrapCargoHome, workRoot)) reject('Cargo seed roots differ');
  const bootstrap = lstatSync(bootstrapCargoHome);
  if (!bootstrap.isDirectory() || bootstrap.isSymbolicLink()
    || !samePath(realpathSync.native(bootstrapCargoHome), bootstrapCargoHome)) {
    reject('bootstrap Cargo home differs');
  }
  const bootstrapIndex = join(bootstrapCargoHome, 'registry', 'index');
  const roots = readdirSync(bootstrapIndex);
  if (roots.length !== 1 || !/^index\.crates\.io-[0-9a-f]{16}$/.test(roots[0])) {
    reject('bootstrap Cargo index root differs');
  }
  mkdirSync(workRoot, { recursive: false, mode: 0o700 });
  const cargoHome = join(workRoot, 'cargo-home');
  const registry = join(cargoHome, 'registry');
  const index = join(registry, 'index');
  const cache = join(registry, 'cache');
  mkdirSync(cargoHome, { recursive: false, mode: 0o700 });
  mkdirSync(registry, { recursive: false, mode: 0o700 });
  mkdirSync(index, { recursive: false, mode: 0o700 });
  copyIndexTree(join(bootstrapIndex, roots[0]), join(index, roots[0]), { entries: 0, total: 0 });
  mkdirSync(cache, { recursive: false, mode: 0o700 });
  const cacheRoot = join(cache, roots[0]);
  mkdirSync(cacheRoot, { recursive: false, mode: 0o700 });
  const identities = new Set();
  for (const capture of rustCaptures) {
    if (!capture || !/^[a-z0-9_-]+-[0-9]+\.[0-9]+\.[0-9]+(?:[A-Za-z0-9.+-]*)$/.test(
      capture.identity ?? '',
    ) || identities.has(capture.identity) || !Buffer.isBuffer(capture.archive)
      || capture.archive.length < 1 || capture.archive.length > MAX_CRATE) {
      reject('Cargo seed capture differs');
    }
    identities.add(capture.identity);
    writeFileSync(join(cacheRoot, `${capture.identity}.crate`), capture.archive,
      { flag: 'wx', mode: 0o600 });
  }
  auditQualificationCargoCache(cargoHome, rustCaptures);
  if (canonical(readdirSync(registry).sort()) !== canonical(['cache', 'index'])) {
    reject('compile Cargo registry contains consumable material outside its closure');
  }
  return { workRoot, cargoHome, registryRoot: roots[0] };
}

function inspectSourceTree(path) {
  const metadata = lstatSync(path);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()
    || !samePath(realpathSync.native(path), path)) reject('Cargo unpacked source directory differs');
  for (const entry of readdirSync(path, { withFileTypes: true })) {
    const child = join(path, entry.name);
    const childMetadata = lstatSync(child);
    if (entry.isDirectory() && childMetadata.isDirectory() && !childMetadata.isSymbolicLink()) {
      inspectSourceTree(child);
    } else if (!(entry.isFile() && childMetadata.isFile() && !childMetadata.isSymbolicLink()
      && samePath(realpathSync.native(child), child))) {
      reject('Cargo unpacked source file identity differs');
    }
  }
}

export function auditQualificationCompileSources(cargoHome, rustCaptures) {
  auditQualificationCargoCache(cargoHome, rustCaptures);
  const registry = join(cargoHome, 'registry');
  const registryEntries = readdirSync(registry).sort();
  const allowedRegistryEntries = registryEntries.includes('CACHEDIR.TAG')
    ? ['CACHEDIR.TAG', 'cache', 'index', 'src'] : ['cache', 'index', 'src'];
  if (canonical(registryEntries) !== canonical(allowedRegistryEntries)) {
    reject('compile Cargo registry inventory differs');
  }
  if (registryEntries.includes('CACHEDIR.TAG')) {
    const tag = join(registry, 'CACHEDIR.TAG');
    const metadata = lstatSync(tag);
    if (!metadata.isFile() || metadata.isSymbolicLink() || metadata.size < 1
      || metadata.size > 1024 || !samePath(realpathSync.native(tag), tag)) {
      reject('Cargo registry cache tag metadata differs');
    }
  }
  const source = join(cargoHome, 'registry', 'src');
  const roots = readdirSync(source);
  const cacheRoots = readdirSync(join(registry, 'cache'));
  if (roots.length !== 1 || cacheRoots.length !== 1 || roots[0] !== cacheRoots[0]
    || !/^index\.crates\.io-[0-9a-f]{16}$/.test(roots[0])) {
    reject('Cargo unpacked source root differs');
  }
  const root = join(source, roots[0]);
  const expected = rustCaptures.map(({ identity }) => identity).sort();
  const actual = readdirSync(root).sort();
  if (canonical(actual) !== canonical(expected)) {
    reject('Cargo unpacked source inventory differs from its captures');
  }
  for (const identity of actual) inspectSourceTree(join(root, identity));
}
