import { spawnSync } from 'node:child_process';
import {
  existsSync, lstatSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { basename, isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { canonicalBounded, sha256 } from './canonical.mjs';
import {
  admitReleaseTransition, extractVerifiedProductionFiles, removeVerifiedProductionFiles,
} from './run-installed-acceptance.mjs';
import { cleanInstalledEnvironment, exactInstalledOutput } from './installed-lifecycle.mjs';
import { createReleaseFile, MAX_RELEASE_FILE, readReleaseFile } from './release-files.mjs';
import { requireCleanHost } from './run-published-acceptance.mjs';
import { verifySignedRelease } from './verify-signed-release.mjs';
import { verifyArchive } from '../distribution/verify.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const REPOSITORY = 'zryna/zryna';
const PREDECESSORS = Object.freeze({
  '0.2.1': Object.freeze({
    version: '0.2.1', tag: 'v0.2.1', channel: 'beta', name: 'Zryna 0.2.1 beta',
    commit: '841c8aee901782c9f7bf442bfe8eb2fe6b7f6446',
  }),
  '0.2.2': Object.freeze({
    version: '0.2.2', tag: 'v0.2.2', channel: 'developer-preview',
    name: 'Zryna 0.2.2 Developer Preview',
    commit: 'c33d76615cafbd4b92e5321c85815bbc3ea4d375',
  }),
});
const CURRENT = Object.freeze({
  version: '0.2.3', tag: 'v0.2.3', channel: 'developer-preview',
  name: 'Zryna 0.2.3 Developer Preview',
});
const TARGETS = Object.freeze({
  'x86_64-unknown-linux-gnu': { cli: 'bin/zryna', extension: 'tar.gz' },
  'x86_64-pc-windows-msvc': { cli: 'bin/zryna.exe', extension: 'zip' },
});
const MAX_OUTPUT = 256 * 1024;
const TIMEOUT = 5 * 60_000;

function reject(message) {
  throw new Error(`R423-PUBLISHED-UPGRADE: ${message}`);
}

function descriptor(envelope, subject) {
  return {
    ...subject.archive, filename: subject.archive.path, version: envelope.version,
    source: {
      repository: envelope.source.repository, ref: envelope.source.ref,
      commit: envelope.source.commit, tree: envelope.source.tree,
      sourceDateEpoch: envelope.source.sourceDateEpoch,
    },
    target: {
      triple: subject.target,
      archiveFormat: subject.target.endsWith('windows-msvc') ? 'zip' : 'tar-gzip',
      platformBaseline: subject.platformBaseline,
    },
    recipe: envelope.recipe,
  };
}

function validateState(state, envelope, directory, expected) {
  if (state?.immutable !== true || state.tag_name !== expected.tag || state.name !== expected.name
    || state.draft !== false || state.prerelease !== true
    || state.html_url !== `https://github.com/${REPOSITORY}/releases/tag/${expected.tag}`
    || typeof state.published_at !== 'string' || !Array.isArray(state.assets)) {
    reject(`${expected.tag} is not the expected immutable prerelease`);
  }
  const assets = [...state.assets].sort((left, right) => left.name.localeCompare(right.name));
  if (canonicalBounded(assets.map(({ name }) => name))
    !== canonicalBounded(envelope.assetAllowlist)) reject(`${expected.tag} asset allowlist differs`);
  for (const asset of assets) {
    const bytes = readReleaseFile(directory, asset.name, MAX_RELEASE_FILE);
    if (asset.state !== 'uploaded' || asset.size !== bytes.length
      || asset.browser_download_url
        !== `https://github.com/${REPOSITORY}/releases/download/${expected.tag}/${asset.name}`) {
      reject(`${expected.tag} asset identity differs`);
    }
  }
}

async function loadRelease({
  cosign, directory, expected, statePath, target, verifyArchiveImpl, verifySignedReleaseImpl,
}) {
  const envelope = await verifySignedReleaseImpl({ directory, cosign });
  if (envelope.version !== expected.version || envelope.tag !== expected.tag
    || envelope.channel !== expected.channel || envelope.source.ref !== `refs/tags/${expected.tag}`
    || (expected.commit && envelope.source.commit !== expected.commit)) {
    reject(`${expected.tag} signed identity differs`);
  }
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  validateState(state, envelope, directory, expected);
  const subject = envelope.subjects.find(({ target: value }) => value === target);
  if (!subject) reject(`${expected.tag} target subject is missing`);
  const archive = readReleaseFile(directory, subject.archive.path, MAX_RELEASE_FILE);
  const archiveDescriptor = descriptor(envelope, subject);
  const verified = await verifyArchiveImpl(archive, archiveDescriptor);
  if (verified?.archiveSha256 !== subject.archive.sha256 || !Array.isArray(verified.files)) {
    reject(`${expected.tag} archive verification differs`);
  }
  return { archive, descriptor: archiveDescriptor, envelope, files: verified.files, state };
}

function command(spawn, executable, args, cwd, environment, expected) {
  const result = spawn(executable, args, {
    cwd, encoding: null, env: environment, maxBuffer: MAX_OUTPUT + 1, shell: false,
    timeout: TIMEOUT, windowsHide: true,
  });
  const stdout = Buffer.isBuffer(result.stdout) ? result.stdout : Buffer.alloc(0);
  const stderr = Buffer.isBuffer(result.stderr) ? result.stderr : Buffer.alloc(0);
  if (result.error || result.signal !== null || result.status !== 0
    || stdout.length + stderr.length > MAX_OUTPUT
    || (expected !== undefined && !exactInstalledOutput({ stdout, stderr }, expected))) {
    reject(`${basename(executable)} ${args[0]} failed`);
  }
  return { stdout, stderr };
}

function runPortableProject(spawn, executable, projectParent, environment, suffix) {
  for (const target of ['javascript', 'webassembly']) {
    for (const operation of ['build', 'run']) {
      const args = [operation, 'src/main.zry', '--project-root', 'hello', '--target', target,
        '--name', `${target}-${suffix}-${operation}`];
      if (operation === 'run') args.push('--export', 'main');
      command(spawn, executable, args, projectParent, environment,
        operation === 'run' ? `${target}: i32 42` : undefined);
    }
  }
}

function retainedProject(projectRoot) {
  return [
    'zryna.package.json', 'zryna.lock.json', 'src/main.zry',
    '.zryna/out/javascript-before-build.build/zryna-manifest-v1.json',
    '.zryna/out/webassembly-before-build.build/zryna-manifest-v1.json',
  ].map((path) => [path, readFileSync(join(projectRoot, ...path.split('/')))]);
}

function assertRetained(projectRoot, retained) {
  if (retained.some(([path, data]) => !readFileSync(join(projectRoot, ...path.split('/'))).equals(data))) {
    reject('upgrade changed retained project source or outputs');
  }
}

export async function acceptPublishedUpgrade({
  cosign = 'cosign', currentDirectory, currentStatePath, outputRoot, previousDirectory,
  previousStatePath, previousVersion, workRoot, target, requireCleanHostImpl = requireCleanHost,
  spawn = spawnSync, verifyArchiveImpl = verifyArchive, verifySignedReleaseImpl = verifySignedRelease,
}) {
  const paths = [currentDirectory, currentStatePath, outputRoot, previousDirectory,
    previousStatePath, workRoot];
  if (!paths.every((path) => isAbsolute(path) && resolve(path) === path)
    || new Set(paths).size !== paths.length || !Object.hasOwn(TARGETS, target)
    || !Object.hasOwn(PREDECESSORS, previousVersion)) {
    reject('distinct absolute release, state, output, and work paths are required');
  }
  if (cosign !== 'cosign' && (!isAbsolute(cosign) || resolve(cosign) !== cosign)) {
    reject('cosign must be the command name or an absolute normalized path');
  }
  const expectedPrevious = PREDECESSORS[previousVersion];
  requireCleanHostImpl(target);
  const previous = await loadRelease({ cosign, directory: previousDirectory, expected: expectedPrevious,
    statePath: previousStatePath, target, verifyArchiveImpl, verifySignedReleaseImpl });
  const current = await loadRelease({ cosign, directory: currentDirectory, expected: CURRENT,
    statePath: currentStatePath, target, verifyArchiveImpl, verifySignedReleaseImpl });
  const transition = admitReleaseTransition(previous.envelope.version, current.envelope.version);
  const root = mkdtempSync(join(workRoot, 'zryna-published-upgrade-'));
  try {
    const projectParent = join(root, 'projects');
    mkdirSync(projectParent, { mode: 0o700 });
    const previousRoot = extractVerifiedProductionFiles(join(root, 'previous'), previous.files);
    const previousCli = join(previousRoot, ...TARGETS[target].cli.split('/'));
    const previousEnvironment = cleanInstalledEnvironment(previousCli, root);
    command(spawn, previousCli, ['--version'], projectParent, previousEnvironment,
      `zryna ${expectedPrevious.version}`);
    command(spawn, previousCli, ['new', 'hello'], projectParent, previousEnvironment);
    runPortableProject(spawn, previousCli, projectParent, previousEnvironment, 'before');
    const projectRoot = join(projectParent, 'hello');
    const retained = retainedProject(projectRoot);
    const projectNote = join(projectParent, 'user-notes.txt');
    writeFileSync(projectNote, 'retain project note\n', { flag: 'wx' });
    writeFileSync(join(previousRoot, 'unrelated.txt'), 'retain old-root file\n', { flag: 'wx' });
    const previousVersionBytes = readFileSync(join(previousRoot, 'VERSION'));
    let downgradeRejected = false;
    try {
      admitReleaseTransition(CURRENT.version, expectedPrevious.version);
    } catch {
      downgradeRejected = true;
    }
    if (!downgradeRejected || !readFileSync(join(previousRoot, 'VERSION')).equals(previousVersionBytes)) {
      reject('downgrade did not fail before filesystem mutation');
    }

    const currentRoot = extractVerifiedProductionFiles(join(root, 'current'), current.files);
    const currentCli = join(currentRoot, ...TARGETS[target].cli.split('/'));
    const currentEnvironment = cleanInstalledEnvironment(currentCli, root);
    command(spawn, currentCli, ['--version'], projectParent, currentEnvironment,
      `zryna ${CURRENT.version}`);
    runPortableProject(spawn, currentCli, projectParent, currentEnvironment, 'after');
    assertRetained(projectRoot, retained);
    writeFileSync(join(currentRoot, 'unrelated.txt'), 'retain new-root file\n', { flag: 'wx' });
    removeVerifiedProductionFiles(previousRoot, previous.files);
    if (existsSync(previousCli) || !readFileSync(join(previousRoot, 'unrelated.txt'), 'utf8')
      .includes('retain old-root') || !readFileSync(projectNote, 'utf8')
      .includes('retain project note')) reject('old-version removal changed foreign files');
    assertRetained(projectRoot, retained);
    removeVerifiedProductionFiles(currentRoot, current.files);
    if (existsSync(currentCli) || !lstatSync(join(currentRoot, 'unrelated.txt')).isFile()) {
      reject('new-version uninstall changed foreign files');
    }
    assertRetained(projectRoot, retained);

    const receipt = Object.freeze({
      format: 'zryna.published-upgrade-acceptance.v1', transition,
      previous: Object.freeze({ tag: expectedPrevious.tag, immutable: true,
        sourceCommit: previous.envelope.source.commit, archiveSha256: sha256(previous.archive) }),
      current: Object.freeze({ tag: CURRENT.tag, immutable: true,
        sourceCommit: current.envelope.source.commit, archiveSha256: sha256(current.archive) }),
      target, checks: Object.freeze({ signedReleases: 2, exactAssets: true, unprivileged: true,
        projectReused: true, javascript: true, webassembly: true, downgradeRejectedBeforeMutation: true,
        oldOwnedRemoval: true, currentOwnedRemoval: true, projectFilesRetained: true,
        unrelatedFilesRetained: true }),
    });
    createReleaseFile(outputRoot, 'published-upgrade-acceptance.json',
      Buffer.from(`${canonicalBounded(receipt)}\n`));
    return receipt;
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
}

function argumentsFrom(argv) {
  const names = ['--cosign', '--current', '--current-state', '--output', '--previous',
    '--previous-state', '--previous-version', '--target', '--work'];
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!names.includes(argv[index]) || argv[index + 1] === undefined || values.has(argv[index])) {
      reject('invalid arguments');
    }
    values.set(argv[index], ['--target', '--previous-version'].includes(argv[index])
      ? argv[index + 1] : resolve(argv[index + 1]));
  }
  if (values.size !== names.length) reject('every upgrade argument is required once');
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    console.log(canonicalBounded(await acceptPublishedUpgrade({
      cosign: values.get('--cosign'), currentDirectory: values.get('--current'),
      currentStatePath: values.get('--current-state'), outputRoot: values.get('--output'),
      previousDirectory: values.get('--previous'), previousStatePath: values.get('--previous-state'),
      previousVersion: values.get('--previous-version'), target: values.get('--target'),
      workRoot: values.get('--work'),
    })));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
