import assert from 'node:assert/strict';
import {
  mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { join, resolve } from 'node:path';
import test from 'node:test';
import { sha256 } from '../scripts/distribution-release/canonical.mjs';
import { acceptPublishedRelease } from '../scripts/distribution-release/run-published-acceptance.mjs';

function fixture(t) {
  const root = resolve(mkdtempSync(join(tmpdir(), 'zryna-published-acceptance-test-')));
  t.after(() => rmSync(root, { recursive: true, force: true }));
  const directory = join(root, 'release');
  const workRoot = join(root, 'work');
  const outputRoot = join(root, 'output');
  mkdirSync(directory);
  mkdirSync(workRoot);
  mkdirSync(outputRoot);
  const name = 'zryna-0.2.1-x86_64-pc-windows-msvc.zip';
  const archive = Buffer.from('immutable published archive');
  writeFileSync(join(directory, name), archive);
  const envelope = {
    version: '0.2.1', assetAllowlist: [name], recipe: { format: 'recipe', sha256: 'c'.repeat(64) },
    source: { repository: 'https://github.com/zryna/zryna', ref: 'refs/tags/v0.2.1',
      commit: '841c8aee901782c9f7bf442bfe8eb2fe6b7f6446', tree: 'b'.repeat(40),
      sourceDateEpoch: 1_789_324_916 },
    subjects: [{ target: 'x86_64-pc-windows-msvc',
      archive: { path: name, size: archive.length, sha256: sha256(archive) },
      platformBaseline: { os: 'windows', product: 'windows-server', version: '2022' } }],
  };
  const statePath = join(root, 'state.json');
  const state = {
    immutable: true, tag_name: 'v0.2.1', draft: false, prerelease: true,
    html_url: 'https://github.com/zryna/zryna/releases/tag/v0.2.1',
    published_at: '2026-09-13T20:20:19Z',
    assets: [{ name, size: archive.length, state: 'uploaded',
      browser_download_url: `https://github.com/zryna/zryna/releases/download/v0.2.1/${name}` }],
  };
  writeFileSync(statePath, JSON.stringify(state));
  return { archive, directory, envelope, outputRoot, root, state, statePath, workRoot };
}

test('binds immutable state and actual negative archive cases before installed acceptance', async (t) => {
  const f = fixture(t);
  const hosts = [];
  const archiveCases = [];
  let installed;
  const receipt = await acceptPublishedRelease({
    directory: f.directory, outputRoot: f.outputRoot, statePath: f.statePath,
    workRoot: f.workRoot, target: 'x86_64-pc-windows-msvc',
    requireCleanHostImpl: (target) => {
      hosts.push(target);
      if (target !== 'x86_64-pc-windows-msvc') throw new Error('wrong host');
    },
    verifySignedReleaseImpl: async () => f.envelope,
    verifyArchiveImpl: async (archive, descriptor) => {
      archiveCases.push({ length: archive.length, version: descriptor.version,
        digest: sha256(archive) });
      throw new Error('rejected before extraction');
    },
    runInstalledAcceptanceImpl: async (input) => {
      installed = input;
      return { checks: { uninstall: true, userFilesRetained: true } };
    },
  });
  assert.deepEqual(hosts,
    ['x86_64-pc-windows-msvc', 'x86_64-unknown-linux-gnu']);
  assert.equal(archiveCases.length, 3);
  assert.equal(archiveCases[0].length, f.archive.length);
  assert.notEqual(archiveCases[0].digest, sha256(f.archive));
  assert.equal(archiveCases[1].length, f.archive.length - 1);
  assert.equal(archiveCases[2].version, '0.2.0');
  assert(installed.archive.equals(f.archive));
  assert.equal(installed.acceptedVersion, '0.2.1');
  assert.equal(installed.descriptor.source.commit, f.envelope.source.commit);
  assert.equal(receipt.release.immutable, true);
  assert.equal(receipt.checks.wrongPlatformRejected, true);
  assert.equal(receipt.checks.userFilesRetained, true);
  assert.deepEqual(JSON.parse(readFileSync(
    join(f.outputRoot, 'published-clean-host-acceptance.json'), 'utf8')),
  receipt);
});

test('rejects mutable or extra published assets before archive cases or installation', async (t) => {
  const f = fixture(t);
  for (const change of [
    (state) => { state.immutable = false; },
    (state) => { state.assets.push({ name: 'extra', size: 1, state: 'uploaded',
      browser_download_url: 'https://github.com/zryna/zryna/releases/download/v0.2.1/extra' }); },
  ]) {
    const state = structuredClone(f.state);
    change(state);
    writeFileSync(f.statePath, JSON.stringify(state));
    let reached = false;
    await assert.rejects(() => acceptPublishedRelease({
      directory: f.directory, outputRoot: f.outputRoot, statePath: f.statePath,
      workRoot: f.workRoot, target: 'x86_64-pc-windows-msvc',
      requireCleanHostImpl: () => {}, verifySignedReleaseImpl: async () => f.envelope,
      verifyArchiveImpl: async () => { reached = true; },
      runInstalledAcceptanceImpl: async () => { reached = true; },
    }), /GitHub release state|asset allowlist/);
    assert.equal(reached, false);
  }
});
