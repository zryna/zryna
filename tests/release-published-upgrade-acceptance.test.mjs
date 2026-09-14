import assert from 'node:assert/strict';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { sha256 } from '../scripts/distribution-release/canonical.mjs';
import { acceptPublishedUpgrade } from '../scripts/distribution-release/run-published-upgrade-acceptance.mjs';

const TARGET = 'x86_64-pc-windows-msvc';

function release(root, version, commit, channel, name) {
  const directory = join(root, `release-${version}`);
  mkdirSync(directory);
  const archiveName = `zryna-${version}-${TARGET}.zip`;
  const archive = Buffer.from(`archive ${version}`);
  writeFileSync(join(directory, archiveName), archive);
  const envelope = {
    version, tag: `v${version}`, channel, assetAllowlist: [archiveName],
    source: { repository: 'https://github.com/zryna/zryna', ref: `refs/tags/v${version}`,
      commit, tree: version === '0.2.1' ? 'b'.repeat(40) : 'd'.repeat(40),
      sourceDateEpoch: 1_789_081_200 },
    recipe: { format: 'zryna.distribution-recipe.v1', sha256: 'e'.repeat(64) },
    subjects: [{ target: TARGET, archive: { path: archiveName, size: archive.length,
      sha256: sha256(archive) }, platformBaseline: { os: 'windows', product: 'windows-server',
      version: '2022', architecture: 'x86_64', runtime: 'operating-system-ucrt' } }],
  };
  const statePath = join(root, `state-${version}.json`);
  const state = {
    immutable: true, tag_name: `v${version}`, name, draft: false, prerelease: true,
    html_url: `https://github.com/zryna/zryna/releases/tag/v${version}`,
    published_at: '2026-09-14T00:00:00Z',
    assets: [{ name: archiveName, state: 'uploaded', size: archive.length,
      browser_download_url: `https://github.com/zryna/zryna/releases/download/v${version}/${archiveName}` }],
  };
  writeFileSync(statePath, JSON.stringify(state));
  const files = [
    { path: 'VERSION', mode: 0o644, data: Buffer.from(`${version}\n`) },
    { path: 'bin/zryna.exe', mode: 0o755, data: Buffer.from(`cli ${version}`) },
    { path: 'runtime/node/node.exe', mode: 0o755, data: Buffer.from(`runtime ${version}`) },
    { path: 'lib/zryna/bootstrap/worker.mjs', mode: 0o644,
      data: Buffer.from(`provider ${version}`) },
    { path: 'metadata/distribution.json', mode: 0o644, data: Buffer.from('{}\n') },
  ];
  return { archive, directory, envelope, files, statePath };
}

function fixture(t) {
  const root = resolve(mkdtempSync(join(tmpdir(), 'zryna-published-upgrade-test-')));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const outputRoot = join(root, 'output');
  const workRoot = join(root, 'work');
  mkdirSync(outputRoot);
  mkdirSync(workRoot);
  const previous = release(root, '0.2.1',
    '841c8aee901782c9f7bf442bfe8eb2fe6b7f6446', 'beta', 'Zryna 0.2.1 beta');
  const current = release(root, '0.2.2', 'c'.repeat(40), 'developer-preview',
    'Zryna 0.2.2 Developer Preview');
  return { current, outputRoot, previous, workRoot };
}

function mockSpawn(executable, args, options) {
  const operation = args[0];
  const version = readFileSync(join(executable, '..', '..', 'VERSION'), 'utf8').trim();
  const project = join(options.cwd, 'hello');
  if (operation === 'new') {
    mkdirSync(join(project, 'src'), { recursive: true });
    writeFileSync(join(project, 'zryna.package.json'), '{}\n');
    writeFileSync(join(project, 'zryna.lock.json'), '{}\n');
    writeFileSync(join(project, 'src', 'main.zry'),
      'export function main(): i32 { return 42; }\n');
  } else if (operation === 'build') {
    const name = args[args.indexOf('--name') + 1];
    const bundle = join(project, '.zryna', 'out', `${name}.build`);
    mkdirSync(bundle, { recursive: true });
    writeFileSync(join(bundle, 'zryna-manifest-v1.json'), '{}\n');
  }
  const target = args.includes('--target') ? args[args.indexOf('--target') + 1] : undefined;
  const stdout = operation === '--version' ? `zryna ${version}\n`
    : operation === 'run' ? `${target}: i32 42\n` : '';
  return { status: 0, signal: null, stdout: Buffer.from(stdout), stderr: Buffer.alloc(0) };
}

test('proves an exact immutable v0.2.1 to v0.2.2 upgrade and owned removal', async (t) => {
  const f = fixture(t);
  const envelopes = new Map([
    [f.previous.directory, f.previous.envelope], [f.current.directory, f.current.envelope],
  ]);
  const files = new Map([['0.2.1', f.previous.files], ['0.2.2', f.current.files]]);
  const receipt = await acceptPublishedUpgrade({
    currentDirectory: f.current.directory, currentStatePath: f.current.statePath,
    outputRoot: f.outputRoot, previousDirectory: f.previous.directory,
    previousStatePath: f.previous.statePath, workRoot: f.workRoot, target: TARGET,
    requireCleanHostImpl: (target) => assert.equal(target, TARGET), spawn: mockSpawn,
    verifySignedReleaseImpl: async ({ directory }) => envelopes.get(directory),
    verifyArchiveImpl: async (archive, descriptor) => ({
      archiveSha256: sha256(archive), files: files.get(descriptor.version),
    }),
  });
  assert.deepEqual(receipt.transition,
    { current: '0.2.1', candidate: '0.2.2', operation: 'upgrade' });
  assert.equal(receipt.previous.sourceCommit,
    '841c8aee901782c9f7bf442bfe8eb2fe6b7f6446');
  assert.equal(receipt.current.sourceCommit, 'c'.repeat(40));
  assert.equal(receipt.checks.downgradeRejectedBeforeMutation, true);
  assert.equal(receipt.checks.projectFilesRetained, true);
  assert.equal(receipt.checks.unrelatedFilesRetained, true);
  assert.deepEqual(JSON.parse(readFileSync(
    join(f.outputRoot, 'published-upgrade-acceptance.json'), 'utf8')), receipt);
});

test('rejects a relabeled previous release before archive execution', async (t) => {
  const f = fixture(t);
  f.previous.envelope.channel = 'developer-preview';
  let reachedArchive = false;
  await assert.rejects(() => acceptPublishedUpgrade({
    currentDirectory: f.current.directory, currentStatePath: f.current.statePath,
    outputRoot: f.outputRoot, previousDirectory: f.previous.directory,
    previousStatePath: f.previous.statePath, workRoot: f.workRoot, target: TARGET,
    requireCleanHostImpl: () => {}, verifySignedReleaseImpl: async ({ directory }) =>
      directory === f.previous.directory ? f.previous.envelope : f.current.envelope,
    verifyArchiveImpl: async () => { reachedArchive = true; },
  }), /v0\.2\.1 signed identity differs/);
  assert.equal(reachedArchive, false);
});
