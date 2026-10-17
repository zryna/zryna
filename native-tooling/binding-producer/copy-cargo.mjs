// Trusted no-run build output acquisition only. Original material admission stays unchanged.
import assert from 'node:assert/strict';
import { constants } from 'node:fs';
import { lstat, open, realpath, rename } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';

const identity = s => ({dev: String(s.dev), ino: String(s.ino), bytes: String(s.size),
  links: String(s.nlink), mode: String(s.mode), modifiedNs: String(s.mtimeNs), changedNs: String(s.ctimeNs)});
const regular = s => assert(s.isFile() && !s.isSymbolicLink() && s.nlink > 0n && s.ino !== 0n,
  'Ordinary Cargo build output required.');
const remaining = d => assert(Number.isFinite(d) && performance.now() < d, 'Cargo copy deadline.');
async function ordinary(path) {
  assert.equal(path, resolve(path));
  for (let current = path;; current = dirname(current)) {
    assert((await lstat(current)).isDirectory());
    assert.equal(await realpath(current), current);
    if (dirname(current) === current) break;
  }
}
export async function copyCargoExecutable(source, destination, debugRoot, deadline, maxBytes = 512 * 1024 * 1024) {
  remaining(deadline);
  assert(Number.isSafeInteger(maxBytes) && maxBytes > 0 && maxBytes <= 512 * 1024 * 1024);
  assert.equal(debugRoot, resolve(debugRoot)); assert.equal(source, resolve(source));
  assert([debugRoot, join(debugRoot, 'deps'), join(debugRoot, 'retained-cargo-originals')].includes(dirname(source)), 'Cargo output escaped fixed build root.');
  await ordinary(dirname(source)); await ordinary(dirname(destination));
  assert.equal(await realpath(source), source);
  const before = await lstat(source, {bigint: true}); regular(before);
  assert(before.size > 0n && before.size <= BigInt(maxBytes), 'Cargo output size bound.');
  const input = await open(source, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0)
    | (process.platform === 'win32' ? 0 : constants.O_NONBLOCK));
  let output;
  try {
    const held = await input.stat({bigint: true}); regular(held);
    assert.deepEqual(identity(held), identity(before));
    output = await open(destination, constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL
      | (constants.O_NOFOLLOW ?? 0), 0o600);
    const hash = createHash('sha256'), buffer = Buffer.alloc(65536); let total = 0;
    for (;;) {
      remaining(deadline);
      const {bytesRead} = await input.read(buffer, 0, Math.min(buffer.length, maxBytes - total + 1), total);
      remaining(deadline);
      if (!bytesRead) break;
      total += bytesRead; assert(total <= maxBytes, 'Cargo output first-extra byte.');
      const chunk = buffer.subarray(0, bytesRead); hash.update(chunk);
      for (let written = 0; written < chunk.length;) {
        remaining(deadline);
        const result = await output.write(chunk, written, chunk.length - written);
        assert(result.bytesWritten > 0); written += result.bytesWritten;
      }
    }
    assert.equal(BigInt(total), before.size);
    for (const after of [await input.stat({bigint: true}), await lstat(source, {bigint: true})]) {
      regular(after); assert.deepEqual(identity(after), identity(before));
    }
    await output.sync(); await output.chmod(0o500);
    const captured = await output.stat({bigint: true});
    assert(captured.isFile() && captured.nlink === 1n && captured.size === before.size);
    assert.notDeepEqual([captured.dev, captured.ino], [before.dev, before.ino]);
    remaining(deadline);
    return {source: {path: source, bytes: total, sha256: hash.digest('hex'), identity: identity(before)},
      destination, policy: 'fixed-Cargo-selected-output-copy-v1',
      scope: 'Build output acquisition; original single-link executable admission still required.'};
  } finally { if (output) await output.close(); await input.close(); }
}

export async function prepareCargoExecutable(source, debugRoot, originalRoot, index, deadline) {
  remaining(deadline);
  assert(Number.isSafeInteger(index) && index >= 0 && index < 16);
  assert.equal(originalRoot, join(debugRoot, 'retained-cargo-originals'));
  assert([debugRoot, join(debugRoot, 'deps')].includes(dirname(source)));
  await ordinary(dirname(source)); assert.equal(await realpath(source), source);
  const before = await lstat(source, {bigint:true}); regular(before);
  if (before.nlink === 1n) return {path: source, unchangedSingleLink: true};
  // Only temporary output of the completed owned no-run Cargo build is moved.
  // Preserve its original inode and every byte; create a new ordinary file at Cargo's exact path.
  await ordinary(originalRoot);
  const retained = join(originalRoot, `original-${index}${process.platform === 'win32' ? '.exe' : ''}`);
  try { await lstat(retained); assert.fail('Retained original already exists.'); }
  catch (error) { if (error.code !== 'ENOENT') throw error; }
  assert.deepEqual(identity(await lstat(source, {bigint:true})), identity(before));
  await rename(source, retained);
  const moved = await lstat(retained, {bigint:true}); regular(moved);
  assert.deepEqual([moved.dev,moved.ino,moved.size,moved.nlink,moved.mode,moved.mtimeNs],
    [before.dev,before.ino,before.size,before.nlink,before.mode,before.mtimeNs]);
  const copied = await copyCargoExecutable(retained, source, debugRoot, deadline);
  return {...copied, path:source, originalCargoPath:source, originalIdentityBeforeMove:identity(before),
    scope:'Owned temporary Cargo output preparation; original inode retained, exact Cargo path restored with equal bytes. No tests executed.'};
}
