import { spawnSync } from 'node:child_process';
import {
  closeSync, existsSync, lstatSync, mkdirSync, mkdtempSync, openSync, renameSync, rmSync,
  writeFileSync,
} from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded, parseCanonical, sha256 } from './canonical.mjs';
import { validateReproductionText } from './compare-release-builds.mjs';
import {
  validateProductionCandidateReproduction,
} from './compare-production-candidate-builds.mjs';
import {
  createReleaseFile, exactReleaseNames, MAX_RELEASE_DOCUMENT, readReleaseFile,
} from './release-files.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const VERSION = '0.2.0';
const MAX_OUTPUT = 256 * 1024;
const TIMEOUT = 5 * 60_000;
const DEFAULT_SYSTEM = Object.freeze({
  close: closeSync, exists: existsSync, lstat: lstatSync, make: mkdirSync,
  makeTemp: mkdtempSync, move: renameSync, open: openSync, remove: rmSync, write: writeFileSync,
});
const TARGETS = Object.freeze({
  'x86_64-unknown-linux-gnu': {
    filename: `zryna-${VERSION}-x86_64-unknown-linux-gnu.tar.gz`,
    cli: 'bin/zryna', runtime: 'runtime/node/bin/node',
    archiveFormat: 'tar-gzip',
    platformBaseline: { os: 'linux', distribution: 'ubuntu', version: '24.04', architecture: 'x86_64' },
  },
  'x86_64-pc-windows-msvc': {
    filename: `zryna-${VERSION}-x86_64-pc-windows-msvc.zip`,
    cli: 'bin/zryna.exe', runtime: 'runtime/node/node.exe',
    archiveFormat: 'zip',
    platformBaseline: { os: 'windows', product: 'windows-server', version: '2022',
      architecture: 'x86_64', runtime: 'operating-system-ucrt' },
  },
});

function reject(message) {
  throw new Error(`R406-INSTALLED-ACCEPTANCE: ${message}`);
}

function safePath(path) {
  if (typeof path !== 'string' || path.length < 1 || path.length > 240
    || path.startsWith('qualification/') || path.includes('\\')) return false;
  const parts = path.split('/');
  return /^[A-Za-z0-9][A-Za-z0-9._@+-]*$/.test(parts[0])
    && parts.slice(1).every((part) => /^[A-Za-z0-9@][A-Za-z0-9._@+-]*$/.test(part)
      && part !== '.' && part !== '..');
}

function requireDirectory(system, path) {
  const entry = system.lstat(path, { throwIfNoEntry: false });
  if (!entry?.isDirectory() || entry.isSymbolicLink()) reject('installation ancestor is unsafe');
}

function createAncestors(system, root, path) {
  let current = root;
  for (const part of dirname(path).split('/')) {
    if (part === '.') continue;
    current = join(current, part);
    const entry = system.lstat(current, { throwIfNoEntry: false });
    if (entry === undefined) system.make(current, { recursive: false, mode: 0o700 });
    requireDirectory(system, current);
  }
}

export function extractVerifiedProductionFiles(root, files, system = DEFAULT_SYSTEM) {
  if (!isAbsolute(root) || resolve(root) !== root || !Array.isArray(files) || files.length < 1) {
    reject('an absolute isolated root and verified files are required');
  }
  system.make(root, { recursive: false, mode: 0o700 });
  requireDirectory(system, root);
  const identities = new Set();
  for (const file of files) {
    if (!safePath(file?.path) || !Buffer.isBuffer(file?.data)
      || ![0o644, 0o755].includes(file?.mode)) reject('verified file tuple is unsafe');
    const identity = file.path.toLowerCase();
    if (identities.has(identity)) reject('verified file paths collide');
    identities.add(identity);
    createAncestors(system, root, file.path);
    const destination = join(root, ...file.path.split('/'));
    if (!resolve(destination).startsWith(`${root}${process.platform === 'win32' ? '\\' : '/'}`)) {
      reject('verified file escaped the installation root');
    }
    let descriptor;
    try {
      descriptor = system.open(destination, 'wx', file.mode);
      system.write(descriptor, file.data);
    } finally {
      if (descriptor !== undefined) system.close(descriptor);
    }
    const installed = system.lstat(destination);
    if (!installed.isFile() || installed.isSymbolicLink() || installed.size !== file.data.length) {
      reject('installed entry is not the verified ordinary file');
    }
  }
  return root;
}

function command(spawn, executable, args, cwd) {
  const result = spawn(executable, args, {
    cwd, encoding: null, maxBuffer: MAX_OUTPUT + 1, timeout: TIMEOUT,
    windowsHide: true, shell: false,
  });
  const stdout = Buffer.isBuffer(result.stdout) ? result.stdout : Buffer.alloc(0);
  const stderr = Buffer.isBuffer(result.stderr) ? result.stderr : Buffer.alloc(0);
  if (result.error || result.signal !== null || stdout.length + stderr.length > MAX_OUTPUT) {
    reject(`installed command ${args[0]} did not complete within its bounds`);
  }
  return { status: result.status, stdout, stderr };
}

function success(spawn, executable, args, cwd) {
  const result = command(spawn, executable, args, cwd);
  if (result.status !== 0) reject(`installed command ${args[0]} failed`);
  return result;
}

function mutate(system, path) {
  const entry = system.lstat(path);
  if (!entry.isFile() || entry.isSymbolicLink() || entry.size < 1) reject('tamper subject is unsafe');
  const bytes = Buffer.alloc(entry.size, 0x78);
  system.write(path, bytes, { flag: 'r+' });
}

function install(system, root, label, files) {
  return extractVerifiedProductionFiles(join(root, label), files, system);
}

export async function runInstalledAcceptance({
  archive, descriptor, workRoot, verifyArchiveImpl, verifyOptions,
  spawn = spawnSync, system = DEFAULT_SYSTEM,
}) {
  if (!Buffer.isBuffer(archive) || !isAbsolute(workRoot) || resolve(workRoot) !== workRoot
    || descriptor?.version !== VERSION || !Object.hasOwn(TARGETS, descriptor?.target?.triple)
    || descriptor.filename !== TARGETS[descriptor.target.triple].filename
    || descriptor.size !== archive.length || descriptor.sha256 !== sha256(archive)
    || typeof verifyArchiveImpl !== 'function') {
    reject('authenticated production archive descriptor is invalid');
  }
  const verified = await verifyArchiveImpl(archive, descriptor, verifyOptions);
  if (verified?.archiveSha256 !== descriptor.sha256 || !Array.isArray(verified.files)
    || verified.files.some(({ path }) => path.startsWith('qualification/'))) {
    reject('production archive verification did not close the expected bytes');
  }
  const target = TARGETS[descriptor.target.triple];
  const paths = new Set(verified.files.map(({ path }) => path));
  const provider = verified.distribution?.files?.find(({ role }) => role === 'provider')?.path;
  for (const path of [target.cli, target.runtime, 'metadata/distribution.json', provider]) {
    if (!safePath(path) || !paths.has(path)) reject('production execution inventory is incomplete');
  }

  requireDirectory(system, workRoot);
  const root = system.makeTemp(join(workRoot, 'zryna-installed-acceptance-'));
  try {
    const staged = install(system, root, 'staged', verified.files);
    const relocated = join(root, 'relocated');
    system.move(staged, relocated);
    requireDirectory(system, relocated);
    const executable = join(relocated, ...target.cli.split('/'));
    const projectParent = join(root, 'projects');
    system.make(projectParent, { recursive: false, mode: 0o700 });
    const version = success(spawn, executable, ['--version'], projectParent);
    if (![Buffer.from(`zryna ${VERSION}\n`), Buffer.from(`zryna ${VERSION}\r\n`)]
      .some((expected) => version.stdout.equals(expected)) || version.stderr.length !== 0) {
      reject('installed version output differs');
    }
    success(spawn, executable, ['new', 'hello'], projectParent);
    for (const [operation, targetName, name] of [
      ['build', 'javascript', 'js-build'], ['run', 'javascript', 'js-run'],
      ['build', 'webassembly', 'wasm-build'], ['run', 'webassembly', 'wasm-run'],
    ]) {
      const args = [operation, 'src/main.zry', '--project-root', 'hello', '--target', targetName,
        '--name', name];
      if (operation === 'run') args.push('--export', 'main');
      const result = success(spawn, executable, args, projectParent);
      if (operation === 'run' && ![Buffer.from('42\n'), Buffer.from('42\r\n')]
        .some((expected) => result.stdout.equals(expected))) reject(`${targetName} result differs`);
    }

    const tamperSubjects = [provider, target.runtime, 'metadata/distribution.json'];
    for (let index = 0; index < tamperSubjects.length; index += 1) {
      const changed = install(system, root, `tampered-${index}`, verified.files);
      mutate(system, join(changed, ...tamperSubjects[index].split('/')));
      const changedCli = join(changed, ...target.cli.split('/'));
      const name = `tamper-${index}`;
      const result = command(spawn, changedCli, [
        'build', 'src/main.zry', '--project-root', 'hello', '--target', 'javascript', '--name', name,
      ], projectParent);
      if (result.status === 0 || !result.stderr.includes(Buffer.from('ZRYNA-C4220'))
        || system.exists(join(projectParent, 'hello', '.zryna', 'out', `${name}.build`))) {
        reject('tampered installation reached artifact execution');
      }
    }
    return Object.freeze({
      format: 'zryna.installed-acceptance.v1', version: VERSION,
      target: descriptor.target.triple, archiveSha256: descriptor.sha256,
      checks: Object.freeze({ relocated: true, version: true, project: true,
        javascript: true, webassembly: true, tamperSubjects: 3 }),
    });
  } finally {
    system.remove(root, { recursive: true, force: true });
  }
}

async function defaultVerifyArchive(archive, descriptor) {
  const { verifyArchive } = await import('../distribution/verify.mjs');
  return verifyArchive(archive, descriptor);
}

export async function acceptReproducedRelease({
  inputRoot, outputRoot, workRoot, target,
  verifyArchiveImpl = defaultVerifyArchive, spawn = spawnSync, system = DEFAULT_SYSTEM,
}) {
  if (![inputRoot, outputRoot, workRoot].every((path) => isAbsolute(path) && resolve(path) === path)
    || new Set([inputRoot, outputRoot, workRoot]).size !== 3 || !Object.hasOwn(TARGETS, target)) {
    reject('distinct absolute input, output, and work roots and a supported target are required');
  }
  const reproduction = validateReproductionText(
    readReleaseFile(inputRoot, 'reproduction.json', MAX_RELEASE_DOCUMENT).toString('utf8'), target,
  );
  exactReleaseNames(inputRoot, [
    'reproduction.json', ...Object.values(reproduction.artifacts).map(({ path }) => path),
  ]);
  const archiveDescriptor = reproduction.artifacts.archive;
  const { tagObject: _tagObject, ...source } = reproduction.source;
  const targetDefinition = TARGETS[target];
  const descriptor = {
    ...archiveDescriptor, filename: archiveDescriptor.path, version: reproduction.version,
    source,
    target: { triple: target, archiveFormat: targetDefinition.archiveFormat,
      platformBaseline: targetDefinition.platformBaseline },
    recipe: reproduction.recipe,
  };
  const acceptance = await runInstalledAcceptance({
    archive: readReleaseFile(inputRoot, archiveDescriptor.path), descriptor, workRoot,
    verifyArchiveImpl, spawn, system,
  });
  system.make(outputRoot, { recursive: false, mode: 0o700 });
  createReleaseFile(outputRoot, 'installed-acceptance.json',
    Buffer.from(`${canonicalBounded(acceptance)}\n`));
  exactReleaseNames(outputRoot, ['installed-acceptance.json']);
  return acceptance;
}

export async function acceptProductionCandidate({
  inputRoot, outputRoot, workRoot, target,
  verifyArchiveImpl = defaultVerifyArchive, spawn = spawnSync, system = DEFAULT_SYSTEM,
}) {
  if (![inputRoot, outputRoot, workRoot].every((path) => isAbsolute(path) && resolve(path) === path)
    || new Set([inputRoot, outputRoot, workRoot]).size !== 3 || !Object.hasOwn(TARGETS, target)) {
    reject('distinct absolute candidate roots and a supported target are required');
  }
  const reproduction = validateProductionCandidateReproduction(parseCanonical(
    readReleaseFile(inputRoot,
      'production-candidate-reproduction.json', MAX_RELEASE_DOCUMENT).toString('utf8'),
  ), target);
  exactReleaseNames(inputRoot, [
    'production-candidate-reproduction.json',
    ...Object.values(reproduction.artifacts).map(({ path }) => path),
  ]);
  const archiveDescriptor = reproduction.artifacts.archive;
  const definition = TARGETS[target];
  const acceptance = await runInstalledAcceptance({
    archive: readReleaseFile(inputRoot, archiveDescriptor.path),
    descriptor: {
      ...archiveDescriptor, filename: archiveDescriptor.path, version: reproduction.version,
      source: reproduction.observedSource,
      target: { triple: target, archiveFormat: definition.archiveFormat,
        platformBaseline: definition.platformBaseline },
      recipe: reproduction.recipe,
    },
    workRoot, verifyArchiveImpl, verifyOptions: { productionCandidate: true }, spawn, system,
  });
  const receipt = {
    format: 'zryna.production-candidate-installed-acceptance.v1',
    status: 'production-candidate', productionAdmission: 'forbidden',
    observedSource: reproduction.observedSource, intendedRelease: reproduction.intendedRelease,
    target, archiveSha256: acceptance.archiveSha256, checks: acceptance.checks,
  };
  system.make(outputRoot, { recursive: false, mode: 0o700 });
  createReleaseFile(outputRoot, 'production-candidate-installed-acceptance.json',
    Buffer.from(`${canonicalBounded(receipt)}\n`));
  exactReleaseNames(outputRoot, ['production-candidate-installed-acceptance.json']);
  return receipt;
}

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!['--input', '--output', '--work', '--candidate'].includes(argv[index])
      || argv[index + 1] === undefined
      || values.has(argv[index])) reject('expected --input, --output, and --work once');
    values.set(argv[index], argv[index] === '--candidate'
      ? argv[index + 1] : resolve(argv[index + 1]));
  }
  const candidate = values.get('--candidate');
  if (values.size !== (candidate === undefined ? 3 : 4) || (candidate !== undefined && candidate !== 'true')) {
    reject('expected --input, --output, and --work once with optional --candidate true');
  }
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    const operation = values.get('--candidate') === 'true'
      ? acceptProductionCandidate : acceptReproducedRelease;
    await operation({
      inputRoot: values.get('--input'), outputRoot: values.get('--output'),
      workRoot: values.get('--work'), target: process.env.ZRYNA_TARGET,
    });
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
