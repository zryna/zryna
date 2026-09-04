import assert from 'node:assert/strict';
import { mkdtemp, writeFile, mkdir, symlink, link, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { spawnSync } from 'node:child_process';
import { test } from 'node:test';
import { fileURLToPath } from 'node:url';
import { TIMING_DIRECTORY, TIMER_NAMES, numericTimers, prepareTimingDirectory, collectTiming } from '../scripts/lib/npm-timing.mjs';

const secret = 'https://user:credential@example.invalid/private\n::error::disclosure';
const raw = () => ({ metadata: { version: '10.9.4', command: ['ci', secret], logfiles: [secret] },
  timers: { npm: 900, 'command:ci': 800, reify: 700, 'reify:audit': 600, 'reify:unpack': 15, [secret]: 1 },
  unfinishedTimers: { [secret]: [1, 2] } });
const parse = value => numericTimers(Buffer.from(JSON.stringify(value)));
const failure = error => error.message === 'npm bootstrap timing unavailable';
async function temporary(context) {
  const root = await mkdtemp(path.join(tmpdir(), 'zryna-npm-timing-'));
  context.after(() => rm(root, { recursive: true, force: true }));
  return root;
}
async function record(root, name = 'fixture-timing.json', value = raw()) {
  await writeFile(path.join(root, TIMING_DIRECTORY, name), JSON.stringify(value));
}

test('timing parser emits only fixed numeric keys and never metadata or request details', () => {
  const output = parse(raw());
  assert.equal(output['reify:audit'], 600);
  assert.equal(output['reify:unpack'], 15);
  assert(!JSON.stringify(output).includes(secret));
  assert(Object.keys(output).every(key => TIMER_NAMES.includes(key)));
  assert(Object.values(output).every(Number.isSafeInteger));
  for (const value of [secret, {}, [], null, true, -1, 0.5, Number.MAX_SAFE_INTEGER + 1]) {
    const input = raw(); input.timers['reify:audit'] = value;
    assert.throws(() => parse(input), failure);
  }
});
test('timing parser rejects malformed or incomplete evidence without reflection', () => {
  for (const input of [Buffer.from(secret), Buffer.from([0xff]), Buffer.alloc(1024 * 1024 + 1)]) {
    assert.throws(() => numericTimers(input), failure);
  }
  for (const mutate of [
    input => { input.metadata.version = secret; },
    input => { input.metadata.command = [secret]; },
    input => { input.timers = []; },
    input => { input.unfinishedTimers = []; },
    input => { input.unfinishedTimers['reify:audit'] = [1, 2]; },
    input => { delete input.timers.reify; },
  ]) { const input = raw(); mutate(input); assert.throws(() => parse(input), failure); }
  const nonCi = raw(); delete nonCi.timers['command:ci']; assert.equal(parse(nonCi), null);
  const absent = raw(); delete absent.timers['reify:audit'];
  assert(!Object.hasOwn(parse(absent), 'reify:audit'));
});
test('fresh exclusive directory and exactly one completed ci record are required', async context => {
  const root = await temporary(context);
  await assert.rejects(collectTiming(root), failure);
  await prepareTimingDirectory(root);
  await assert.rejects(prepareTimingDirectory(root), failure);
  await assert.rejects(collectTiming(root), failure);
  await record(root);
  await writeFile(path.join(root, TIMING_DIRECTORY, 'secret-debug.log'), secret);
  assert.deepEqual(await collectTiming(root), parse(raw()));
  await record(root, 'second-timing.json');
  await assert.rejects(collectTiming(root), failure);
});
test('collector rejects malformed candidates and bounded input overruns', async context => {
  for (const mode of ['malformed', 'large', 'many', 'directory']) {
    const root = await temporary(context); await prepareTimingDirectory(root);
    const file = path.join(root, TIMING_DIRECTORY, 'bad-timing.json');
    if (mode === 'malformed') await writeFile(file, secret);
    if (mode === 'large') await writeFile(file, Buffer.alloc(1024 * 1024 + 1));
    if (mode === 'many') await Promise.all(Array.from({ length: 33 }, (_, i) => record(root, `${i}-timing.json`)));
    if (mode === 'directory') await mkdir(file);
    await assert.rejects(collectTiming(root), failure);
  }
});
test('existing child and symlink ancestors cannot supply stale timing', async context => {
  const root = await temporary(context);
  await mkdir(path.join(root, TIMING_DIRECTORY));
  await assert.rejects(prepareTimingDirectory(root), failure);
  const linked = await temporary(context);
  try { await symlink(root, path.join(linked, 'ancestor'), 'junction'); }
  catch (error) {
    if (['EPERM', 'EACCES'].includes(error.code)) { context.skip('host forbids test links'); return; }
    throw error;
  }
  await assert.rejects(prepareTimingDirectory(path.join(linked, 'ancestor')), failure);
});
test('collector rejects timing file links', async context => {
  const root = await temporary(context); await prepareTimingDirectory(root);
  const source = path.join(root, 'payload'); await writeFile(source, JSON.stringify(raw()));
  await link(source, path.join(root, TIMING_DIRECTORY, 'linked-timing.json'));
  await assert.rejects(collectTiming(root), failure);
});
test('CLI failure emits only the fixed unavailable message and preserves failure status', () => {
  const script = fileURLToPath(new URL('../scripts/collect-npm-timing.mjs', import.meta.url));
  for (const args of [['collect'], ['prepare'], ['collect', secret], [secret]]) {
    const result = spawnSync(process.execPath, [script, ...args], {
      env: { ...process.env, RUNNER_TEMP: 'relative-secret-path' }, encoding: 'utf8',
    });
    assert.equal(result.status, 1);
    assert.equal(result.stdout, '');
    assert.equal(result.stderr, 'npm bootstrap timing unavailable\n');
  }
});

test('CLI success emits exactly one sanitized numeric record', async context => {
  const root = await temporary(context);
  const script = fileURLToPath(new URL('../scripts/collect-npm-timing.mjs', import.meta.url));
  const options = { env: { ...process.env, RUNNER_TEMP: root }, encoding: 'utf8' };
  const prepared = spawnSync(process.execPath, [script, 'prepare'], options);
  assert.equal(prepared.status, 0);
  assert.equal(prepared.stdout + prepared.stderr, '');
  await record(root, 'credential-in-filename-timing.json');
  const collected = spawnSync(process.execPath, [script, 'collect'], options);
  assert.equal(collected.status, 0);
  assert.equal(collected.stderr, '');
  assert.equal(collected.stdout, `${JSON.stringify(parse(raw()))}\n`);
});
