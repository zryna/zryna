import assert from 'node:assert/strict';
import { mkdtemp, mkdir, realpath, writeFile, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { argumentsFor } from '../installed-editor.mjs';
import { canonicalJson, inventory, sha, verifyInstalled, verifyEditorArchive } from './materials.mjs';
import { debuggerEndpoint, waitJson } from './trust.mjs';
import { remaining } from './process.mjs';
import { EDITOR_ARCHIVES } from './pins.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'm6-installed-editor-unit-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}

test('closed installed-editor argv rejects missing, extra, relative and foreign source identities', () => {
  const root = process.platform === 'win32' ? 'C:\\m6' : '/m6';
  const args = ['--handoff', root, '--editor-archive', root, '--source-commit', 'a'.repeat(40),
    '--source-tree', 'b'.repeat(40), '--source-epoch', '1',
    '--setup-archive-sha256', 'c'.repeat(64), '--setup-manifest-sha256', 'd'.repeat(64),
    '--server-sha256', 'e'.repeat(64), '--vsix-sha256', 'f'.repeat(64), '--output', root];
  assert.equal(argumentsFor(args).prepareOnly, false);
  assert.equal(argumentsFor([...args, '--prepare-only']).prepareOnly, true);
  for (const changed of [args.slice(1), [...args, '--no-sandbox'], [...args, '--skip'],
    args.map((value, index) => index === 1 ? '../relative' : value),
    args.map((value, index) => index === 5 ? 'main' : value)]) {
    assert.throws(() => argumentsFor(changed));
  }
});

test('canonical selected descriptors reject duplicate keys, malformed UTF-8 and noncanonical text', async t => {
  const root = await fixture(t), file = join(root, 'selected.json');
  await writeFile(file, '{\n  "version": 1\n}\n');
  assert.deepEqual(await canonicalJson(file), { version: 1 });
  for (const value of ['{"a":1,"a":2}\n', '{}', '{"extra":0}\n', Buffer.from([0xff])]) {
    await writeFile(file, value);
    await assert.rejects(canonicalJson(file));
  }
  await writeFile(file, '{\n  "version": 1\n}\n');
  assert.deepEqual(await canonicalJson(file), { version: 1 });
});

test('actual installed file bodies, extra files and recovery are checked against independent payload', async t => {
  const root = await fixture(t), file = join(root, 'client.cjs');
  const bytes = Buffer.from('unit-only fixture; not an actual VSIX or editor receipt');
  const expected = { 'client.cjs': { bytes: bytes.length, sha256: sha(bytes) } };
  await writeFile(file, bytes);
  assert.equal((await verifyInstalled(root, expected)).entries.length, 1);
  await writeFile(file, Buffer.alloc(bytes.length, 0));
  await assert.rejects(verifyInstalled(root, expected));
  await writeFile(file, bytes);
  await writeFile(join(root, 'extra.cjs'), 'unselected');
  await assert.rejects(verifyInstalled(root, expected));
  await rm(join(root, 'extra.cjs'));
  await mkdir(join(root, 'unexpected-directory'));
  await assert.rejects(verifyInstalled(root, expected));
  await rm(join(root, 'unexpected-directory'), { recursive: true });
  await assert.rejects(verifyInstalled(root, { ...expected,
    'absent.cjs': { bytes: 1, sha256: sha(Buffer.from('x')) } }));
  assert.equal((await verifyInstalled(root, expected)).entries.length, 1);
});

test('editor inventory binds namespace and rejects a real escaping directory alias', async t => {
  const parent = await fixture(t), root = join(parent, 'root'), outside = join(parent, 'outside');
  await mkdir(root); await mkdir(outside);
  await writeFile(join(root, 'file'), 'original');
  await writeFile(join(outside, 'secret-canary'), 'not material');
  const before = await inventory(root);
  await symlink(outside, join(root, 'linked'), process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(inventory(root));
  await rm(join(root, 'linked'));
  assert.deepEqual(await inventory(root), before);
});

test('official Code archive identities cannot be supplied by a tiny adjacent self-hash', async t => {
  const root = await fixture(t), archive = join(root, 'editor.archive');
  await writeFile(archive, 'synthetic archive is never positive publisher evidence');
  await assert.rejects(verifyEditorArchive(archive));
  assert.equal(EDITOR_ARCHIVES.linux.bytes, 344482658);
  assert.equal(EDITOR_ARCHIVES.win32.bytes, 341288168);
  assert.notEqual(EDITOR_ARCHIVES.linux.sha256, EDITOR_ARCHIVES.win32.sha256);
});

test('debugger authority is restricted to a canonical finite loopback port and browser path', () => {
  assert.equal(debuggerEndpoint('1234\n/devtools/browser/a-b-c\n'),
    'ws://127.0.0.1:1234/devtools/browser/a-b-c');
  for (const text of ['0\n/devtools/browser/x', '65536\n/devtools/browser/x',
    '1234\nhttp://other/secret', '1234\n/devtools/browser/../../secret',
    '1234\n/devtools/browser/x\ntrailing', '0123\n/devtools/browser/x']) {
    assert.throws(() => debuggerEndpoint(text));
  }
});

test('completed receipt marker is required and malformed marked JSON fails', async t => {
  const root = await fixture(t), file = join(root, 'result.json');
  await writeFile(file, '{}');
  await assert.rejects(waitJson(file, performance.now() + 1000,
    () => new Error('owned editor ended before marker')));
  await writeFile(file + '.ready', '');
  assert.deepEqual(await waitJson(file, performance.now() + 1000,
    () => new Error('ended after publishing valid complete result')), {});
  await writeFile(file, '{broken');
  await assert.rejects(waitJson(file, performance.now() + 1000));
});

test('absolute local deadline is not renewed and an absent result cannot hang', async t => {
  const root = await fixture(t);
  assert.throws(() => remaining(performance.now() - 1));
  await assert.rejects(waitJson(join(root, 'absent'), performance.now() + 5));
  assert(remaining(performance.now() + 1000) > 0);
});
