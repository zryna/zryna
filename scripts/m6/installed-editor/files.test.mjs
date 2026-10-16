import assert from 'node:assert/strict';
import { mkdtemp, mkdir, readFile, realpath, writeFile, link, symlink, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import { createHash } from 'node:crypto';
import test from 'node:test';
import { capture, descriptor, ordinaryRoot, readBytes } from './files.mjs';

async function fixture(t) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'm6-retained-file-unit-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  return root;
}
const digest = bytes => createHash('sha256').update(bytes).digest('hex');

test('canonical temporary roots retain directory-alias refusal for supplied materials', async t => {
  const root = await fixture(t), directory = join(root, 'ordinary'), alias = join(root, 'alias');
  assert.equal(await realpath(root), root);
  await mkdir(directory);
  const source = join(directory, 'source');
  await writeFile(source, 'ordinary material');
  await ordinaryRoot(directory);
  await symlink(directory, alias, process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(ordinaryRoot(alias));
  await assert.rejects(descriptor(join(alias, 'source')));
  assert.equal((await descriptor(source)).sha256, digest(Buffer.from('ordinary material')));
});

test('retained-FD capture consumes exact synthetic bytes and preserves the protected copy', async t => {
  const root = await fixture(t), source = join(root, 'source');
  const privateRoot = join(root, 'private');
  await mkdir(privateRoot, { mode: 0o700 });
  const bytes = Buffer.alloc(65537, 42);
  await writeFile(source, bytes);
  const destination = join(privateRoot, 'captured');
  const record = await capture(source, destination,
    { bytes: bytes.length, sha256: digest(bytes) }, performance.now() + 10000);
  await writeFile(source, 'replaced external source after complete capture');
  assert.deepEqual(await readFile(destination), bytes);
  assert.equal(record.captured.sha256, digest(bytes));
  assert(record.source.identity.ino !== '0');
  await assert.rejects(capture(source, destination,
    { bytes: bytes.length, sha256: digest(bytes) }, performance.now() + 10000));
});

test('actual file bounds, wrong digests, multi-link identities and recovery fail closed', async t => {
  const root = await fixture(t), source = join(root, 'source');
  const bytes = Buffer.from('synthetic unit material, never an official archive');
  await writeFile(source, bytes);
  await assert.rejects(capture(source, join(root, 'short'),
    { bytes: bytes.length - 1, sha256: digest(bytes) }, performance.now() + 10000));
  await assert.rejects(capture(source, join(root, 'wrong'),
    { bytes: bytes.length, sha256: '0'.repeat(64) }, performance.now() + 10000));
  await link(source, join(root, 'second-name'));
  await assert.rejects(descriptor(source));
  await rm(join(root, 'second-name'));
  const good = await capture(source, join(root, 'good'),
    { bytes: bytes.length, sha256: digest(bytes) }, performance.now() + 10000);
  assert.equal(good.captured.bytes, bytes.length);
});

test('expired absolute FD-read deadline refuses before accepting or parsing bytes', async t => {
  const root = await fixture(t), source = join(root, 'source');
  await writeFile(source, '{}');
  await assert.rejects(readBytes(source, performance.now() - 1, 2));
  assert.deepEqual(await readBytes(source, performance.now() + 10000, 2), Buffer.from('{}'));
  await assert.rejects(readBytes(source, performance.now() + 10000, 1));
});
