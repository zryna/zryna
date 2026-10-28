// Registered by files.test.mjs; no guest, PowerShell, editor or native process required.
import assert from 'node:assert/strict';
import { link, mkdir, mkdtemp, readFile, readdir, realpath, rm, symlink, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { performance } from 'node:perf_hooks';
import test from 'node:test';
import { retainUnitFailure } from './unit-failure-retention.mjs';
async function fixture(t) {
  const root = await mkdtemp(join(await realpath(tmpdir()), 'm6-unit-retention-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const output = join(root, 'records'); await mkdir(output);
  return { root, output, name: 'job-inventory', parent: root,
    deadline: performance.now() + 10000, error: new Error('synthetic ready failure; no host result') };
}
const manifest = async directory => JSON.parse(await readFile(join(directory, 'failure.json'), 'utf8'));
test('failed ready records survive original removal byte-for-byte without granting cleanup or copying extra files', async t => {
  const options = await fixture(t), material = join(options.output, 'job-inventory.owner-material');
  await mkdir(material);
  const expected = new Map([
    ['job-inventory.stdout', Buffer.from([0, 1, 2, 255])], ['job-inventory.stderr', Buffer.from('target-stderr')],
    ['job-inventory.owner.jsonl', Buffer.from('{"kind":"ready"}\n')],
    ['job-inventory.owner.stderr', Buffer.from('original native error')],
    ['job-inventory.json', Buffer.from('{"cleanupConfirmed":false,"diagnostics":{"phase":"waiting-ready"}}\n')],
    ['job-inventory.before-start.json', Buffer.from('{"retained":"original"}\n')],
    ['owner.ps1', Buffer.from('original pinned owner')], ['owner-windows-job.ps1', Buffer.from('original job dependency')],
  ]);
  for (const [name, bytes] of expected) await writeFile(join(name.startsWith('owner') ? material : options.output, name), bytes, { flag: 'wx' });
  await writeFile(join(options.output, 'unrelated-secret.txt'), 'must never be copied', { flag: 'wx' });
  const result = await retainUnitFailure(options); assert.equal(result.complete, true);
  await rm(options.output, { recursive: true, force: true });
  for (const [name, bytes] of expected) assert.deepEqual(await readFile(join(result.directory, name)), bytes);
  assert.deepEqual((await readdir(result.directory)).sort(), [...expected.keys(), 'failure.json'].sort());
  const record = await manifest(result.directory);
  assert.equal(record.entries.length, 8); assert(record.entries.every(row => row.state === 'captured'));
  assert.equal(record.cleanupConfirmed, false); assert.equal(record.proofQualified, false);
  assert.equal(record.sourceAuthenticated, false); assert.equal(record.thresholds, null);
  assert.match(record.primaryError, /synthetic ready failure/);
});
test('missing pre-ready records stay absent and repeated failure captures never overwrite prior observations', async t => {
  const options = await fixture(t);
  await writeFile(join(options.output, 'job-inventory.owner.stderr'), 'first native error', { flag: 'wx' });
  const first = await retainUnitFailure(options);
  await writeFile(join(options.output, 'job-inventory.owner.stderr'), 'second native error');
  const second = await retainUnitFailure(options); assert.notEqual(first.directory, second.directory);
  assert.equal(await readFile(join(first.directory, 'job-inventory.owner.stderr'), 'utf8'), 'first native error');
  assert.equal(await readFile(join(second.directory, 'job-inventory.owner.stderr'), 'utf8'), 'second native error');
  const record = await manifest(first.directory);
  assert.equal(record.entries.filter(row => row.state === 'absent').length, 7);
  assert.equal(record.entries.filter(row => row.state === 'captured').length, 1);
});
test('first-extra records refuse before capture while other bounded originals remain retained', async t => {
  const options = await fixture(t), material = join(options.output, 'job-inventory.owner-material');
  await mkdir(material); await writeFile(join(material, 'owner.ps1'), Buffer.alloc(65537));
  await writeFile(join(options.output, 'job-inventory.owner.stderr'), 'bounded independent failure');
  const result = await retainUnitFailure(options); assert.equal(result.complete, false);
  const record = await manifest(result.directory);
  assert.equal(record.entries.find(row => row.destination === 'owner.ps1').state, 'refused');
  assert(!(await readdir(result.directory)).includes('owner.ps1'));
  assert.equal(await readFile(join(result.directory, 'job-inventory.owner.stderr'), 'utf8'), 'bounded independent failure');
  assert.equal((await readFile(join(material, 'owner.ps1'))).length, 65537, 'refused original remains untouched');
});
test('hard-linked records and linked source directories fail closed without copying foreign bytes', async t => {
  const options = await fixture(t), foreign = join(options.root, 'foreign');
  await writeFile(foreign, 'foreign protected data');
  await link(foreign, join(options.output, 'job-inventory.owner.stderr'));
  const result = await retainUnitFailure(options); assert.equal(result.complete, false);
  assert(!(await readdir(result.directory)).includes('job-inventory.owner.stderr'));
  const foreignDirectory = join(options.root, 'foreign-directory'); await mkdir(foreignDirectory);
  await writeFile(join(foreignDirectory, 'marker'), 'protected');
  await symlink(foreignDirectory, join(options.output, 'job-inventory.owner-material'),
    process.platform === 'win32' ? 'junction' : 'dir');
  const nested = await retainUnitFailure(options);
  const nestedRecord = await manifest(nested.directory);
  assert(nestedRecord.entries.filter(row => row.destination.startsWith('owner')).every(row => row.state === 'refused'),
    'a missing leaf cannot conceal a linked material parent');
  assert.equal(await readFile(join(foreignDirectory, 'marker'), 'utf8'), 'protected');
  const alias = join(options.root, 'alias');
  await symlink(options.output, alias, process.platform === 'win32' ? 'junction' : 'dir');
  await assert.rejects(retainUnitFailure({ ...options, output: alias }));
  assert.equal(await readFile(foreign, 'utf8'), 'foreign protected data');
});
test('invalid record selectors and expired retention never renew a deadline or claim complete retention', async t => {
  const options = await fixture(t);
  for (const name of ['../escape', '', 'C:foreign', 'Upper', 'a'.repeat(65)]) {
    await assert.rejects(retainUnitFailure({ ...options, name }));
  }
  await writeFile(join(options.output, 'job-inventory.owner.stderr'), 'original');
  const result = await retainUnitFailure({ ...options, deadline: performance.now() - 1 });
  assert.equal(result.complete, false);
  const record = await manifest(result.directory);
  assert.equal(record.entries.find(row => row.destination === 'job-inventory.owner.stderr').state, 'refused');
  assert.equal(record.cleanupConfirmed, false); assert.equal(record.originalStagePreserved, true);
  assert.equal(await readFile(join(options.output, 'job-inventory.owner.stderr'), 'utf8'), 'original');
});
