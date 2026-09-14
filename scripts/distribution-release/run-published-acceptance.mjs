import { readFileSync } from 'node:fs';
import { arch, platform, release } from 'node:os';
import { isAbsolute, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { canonicalBounded, sha256 } from './canonical.mjs';
import { createReleaseFile, MAX_RELEASE_FILE, readReleaseFile } from './release-files.mjs';
import { runInstalledAcceptance } from './run-installed-acceptance.mjs';
import { verifySignedRelease } from './verify-signed-release.mjs';
import { verifyArchive } from '../distribution/verify.mjs';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const TAG = 'v0.2.1';
const REPOSITORY = 'zryna/zryna';
const TARGETS = Object.freeze({
  linux: 'x86_64-unknown-linux-gnu',
  win32: 'x86_64-pc-windows-msvc',
});

function reject(message) {
  throw new Error(`R423-PUBLISHED-ACCEPTANCE: ${message}`);
}

export function requireCleanHost(target) {
  if (arch() !== 'x64' || TARGETS[platform()] !== target) reject('release target differs from host');
  if (platform() === 'linux') {
    const fields = Object.fromEntries(readFileSync('/etc/os-release', 'utf8').split('\n')
      .filter((line) => line.includes('=')).map((line) => {
        const index = line.indexOf('=');
        return [line.slice(0, index), line.slice(index + 1).replace(/^"|"$/g, '')];
      }));
    if (fields.ID !== 'ubuntu' || fields.VERSION_ID !== '24.04' || process.getuid?.() === 0) {
      reject('Linux host is not unprivileged Ubuntu 24.04 x86-64');
    }
  } else {
    const groups = spawnSync('whoami', ['/groups', '/fo', 'csv', '/nh'], {
      encoding: 'utf8', shell: false, windowsHide: true,
    });
    if (release() !== '10.0.20348' || groups.status !== 0
      || groups.stdout.includes('S-1-5-32-544')) {
      reject('Windows host is not an unprivileged Windows Server 2022 x64 user');
    }
  }
}

function validatePublishedState(state, envelope, directory) {
  if (state?.immutable !== true || state.tag_name !== TAG || state.draft !== false
    || state.prerelease !== true || state.html_url !== `https://github.com/${REPOSITORY}/releases/tag/${TAG}`
    || typeof state.published_at !== 'string' || !Array.isArray(state.assets)) {
    reject('GitHub release state is not the expected immutable prerelease');
  }
  const assets = [...state.assets].sort((left, right) => left.name.localeCompare(right.name));
  if (canonicalBounded(assets.map(({ name }) => name))
    !== canonicalBounded(envelope.assetAllowlist)) reject('published asset allowlist differs');
  for (const asset of assets) {
    const bytes = readReleaseFile(directory, asset.name, MAX_RELEASE_FILE);
    if (asset.state !== 'uploaded' || asset.size !== bytes.length
      || asset.browser_download_url !== `https://github.com/${REPOSITORY}/releases/download/${TAG}/${asset.name}`) {
      reject('published asset identity differs');
    }
  }
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

async function rejects(operation, detail) {
  try {
    await operation();
  } catch {
    return true;
  }
  reject(`${detail} did not reject before installation`);
}

export async function acceptPublishedRelease({
  cosign = 'cosign', directory, outputRoot, statePath, workRoot, target,
  requireCleanHostImpl = requireCleanHost, runInstalledAcceptanceImpl = runInstalledAcceptance,
  verifyArchiveImpl = verifyArchive, verifySignedReleaseImpl = verifySignedRelease,
}) {
  if (![directory, outputRoot, statePath, workRoot].every((path) => isAbsolute(path)
    && resolve(path) === path) || !Object.values(TARGETS).includes(target)) {
    reject('absolute release, state, output, and work paths plus a supported target are required');
  }
  requireCleanHostImpl(target);
  if (cosign !== 'cosign' && (!isAbsolute(cosign) || resolve(cosign) !== cosign)) {
    reject('cosign must be the command name or an absolute normalized path');
  }
  const envelope = await verifySignedReleaseImpl({ directory, cosign });
  if (envelope.version !== '0.2.1' || envelope.source.commit
    !== '841c8aee901782c9f7bf442bfe8eb2fe6b7f6446') reject('signed release identity differs');
  const state = JSON.parse(readFileSync(statePath, 'utf8'));
  validatePublishedState(state, envelope, directory);
  const subject = envelope.subjects.find(({ target: value }) => value === target);
  if (!subject) reject('signed target subject is missing');
  const expected = descriptor(envelope, subject);
  const archive = readReleaseFile(directory, subject.archive.path, MAX_RELEASE_FILE);
  const changed = Buffer.from(archive);
  changed[Math.floor(changed.length / 2)] ^= 1;
  await rejects(() => verifyArchiveImpl(changed, expected), 'tampered archive');
  await rejects(() => verifyArchiveImpl(archive.subarray(0, archive.length - 1), expected),
    'incomplete archive');
  await rejects(() => verifyArchiveImpl(archive, { ...expected, version: '0.2.0' }),
    'wrong-version archive');
  await rejects(async () => requireCleanHostImpl(
    Object.values(TARGETS).find((value) => value !== target)),
    'wrong-platform archive');
  const installed = await runInstalledAcceptanceImpl({
    archive, descriptor: expected, workRoot, verifyArchiveImpl,
  });
  const receipt = Object.freeze({
    format: 'zryna.published-clean-host-acceptance.v1',
    release: Object.freeze({ tag: TAG, immutable: true, publishedAt: state.published_at,
      sourceCommit: envelope.source.commit }),
    target, archiveSha256: sha256(archive),
    checks: Object.freeze({ signedRelease: true, exactAssets: true, unprivileged: true,
      cleanEnvironment: true, tamperedArchiveRejected: true, incompleteArchiveRejected: true,
      wrongVersionRejected: true, wrongPlatformRejected: true, ...installed.checks }),
  });
  createReleaseFile(outputRoot, 'published-clean-host-acceptance.json',
    Buffer.from(`${canonicalBounded(receipt)}\n`));
  return receipt;
}

function argumentsFrom(argv) {
  const values = new Map();
  for (let index = 0; index < argv.length; index += 2) {
    if (!['--cosign', '--directory', '--output', '--state', '--target', '--work'].includes(argv[index])
      || argv[index + 1] === undefined || values.has(argv[index])) reject('invalid arguments');
    values.set(argv[index], argv[index] === '--target' ? argv[index + 1] : resolve(argv[index + 1]));
  }
  if (values.size !== 6) reject('expected cosign, directory, output, state, target, and work once');
  return values;
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    const values = argumentsFrom(process.argv.slice(2));
    console.log(canonicalBounded(await acceptPublishedRelease({
      cosign: values.get('--cosign'), directory: values.get('--directory'), outputRoot: values.get('--output'),
      statePath: values.get('--state'), target: values.get('--target'), workRoot: values.get('--work'),
    })));
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
