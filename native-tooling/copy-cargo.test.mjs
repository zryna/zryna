import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, mkdir, writeFile, link, symlink, readFile, stat, rm } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { performance } from 'node:perf_hooks';
import { createHash } from 'node:crypto';
import { copyCargoExecutable, prepareCargoExecutable } from './binding-producer/copy-cargo.mjs';
import { descriptor } from './owner/files.mjs';

test('Cargo output copies preserve content while original single-link guards remain strict', async () => {
  const root = await mkdtemp(join(tmpdir(), 'm6410-cargo-copy-'));
  try {
    const debug = join(root, 'debug'), out = join(root, 'captured');
    await mkdir(debug); await mkdir(out); await mkdir(join(debug, 'deps'));
    const source = join(debug, 'tool'), alias = join(debug, 'deps', 'tool-hash'), data = Buffer.from('actual bounded executable fixture\n');
    await writeFile(source, data); await link(source, alias);
    await assert.rejects(descriptor(source), /Single-link/);
    const destination = join(out, 'tool');
    const observed = await copyCargoExecutable(source, destination, debug, performance.now() + 10000);
    assert.equal(observed.source.identity.links, '2');
    const accepted = await descriptor(destination);
    assert.equal(accepted.identity.links, '1'); assert.equal(accepted.bytes, data.length);
    assert.equal(accepted.sha256, createHash('sha256').update(data).digest('hex'));
    assert.deepEqual(await readFile(source), data); assert.equal((await stat(source)).nlink, 2);
    await assert.rejects(copyCargoExecutable(source, destination, debug, performance.now() + 10000), /EEXIST/);
    await symlink(source, join(debug, 'linked'));
    await assert.rejects(copyCargoExecutable(join(debug, 'linked'), join(out, 'symlink'), debug, performance.now() + 10000));
    await assert.rejects(copyCargoExecutable(source, join(out, 'oversize'), debug, performance.now() + 10000, data.length - 1), /size bound/);
    await assert.rejects(copyCargoExecutable(source, join(out, 'expired'), debug, performance.now() - 1), /deadline/);
    await assert.rejects(copyCargoExecutable(source, join(out, 'escaped'), out, performance.now() + 10000), /escaped/);
    const originals = join(debug, 'retained-cargo-originals'); await mkdir(originals);
    const oldIdentity = await stat(source, {bigint:true});
    const prepared = await prepareCargoExecutable(source, debug, originals, 0, performance.now() + 10000);
    assert.equal(prepared.path, source);
    const strict = await descriptor(source);
    assert.equal(strict.sha256, accepted.sha256); assert.equal(strict.identity.links, '1');
    assert.equal((await stat(prepared.source.path, {bigint:true})).ino, oldIdentity.ino);
    assert.equal((await stat(alias, {bigint:true})).ino, oldIdentity.ino);
    assert.deepEqual(await readFile(prepared.source.path), data);
    const unchanged = await prepareCargoExecutable(source, debug, originals, 1, performance.now() + 10000);
    assert.equal(unchanged.unchangedSingleLink, true);
  } finally { await rm(root, {recursive:true}); }
});
