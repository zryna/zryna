import assert from 'node:assert/strict';
import { constants } from 'node:fs';
import { lstat, open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { dirname, isAbsolute, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { remaining } from './process.mjs';

const fingerprint = stat => ({ dev: String(stat.dev), ino: String(stat.ino),
  bytes: String(stat.size), links: String(stat.nlink), mode: String(stat.mode),
  modifiedNs: String(stat.mtimeNs), changedNs: String(stat.ctimeNs) });
function regular(stat, path, site) {
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.nlink === 1n && stat.ino !== 0n,
    'Single-link ordinary file identity required. ' + JSON.stringify({ path, site,
      isFile: stat.isFile(), isSymbolicLink: stat.isSymbolicLink(), links: String(stat.nlink),
      inode: String(stat.ino), device: String(stat.dev), mode: String(stat.mode) }));
}
export async function ordinaryRoot(root) {
  assert(isAbsolute(root) && root === resolve(root));
  let current = root;
  for (;;) {
    const stat = await lstat(current);
    assert(stat.isDirectory() && !stat.isSymbolicLink());
    assert.equal(await realpath(current), current);
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
}
async function retainedRead(path, { deadline, maxBytes, expected }, consume) {
  remaining(deadline);
  await ordinaryRoot(dirname(path));
  const before = await lstat(path, { bigint: true });
  regular(before, path, 'BEFORE');
  assert(before.size <= BigInt(maxBytes));
  // POSIX rejects final links in open itself. Windows additionally binds regular path and
  // retained-handle identities before and after every complete operation; no link fallback.
  const fd = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0)
    | (process.platform === 'win32' ? 0 : constants.O_NONBLOCK));
  try {
    const held = await fd.stat({ bigint: true });
    regular(held, path, 'HELD');
    assert.deepEqual(fingerprint(held), fingerprint(before));
    const hash = createHash('sha256');
    const buffer = Buffer.alloc(65536);
    let total = 0;
    for (;;) {
      remaining(deadline);
      const { bytesRead } = await fd.read(buffer, 0, Math.min(buffer.length, maxBytes - total + 1), total);
      remaining(deadline);
      if (!bytesRead) break;
      total += bytesRead;
      assert(total <= maxBytes, 'Material first-extra byte rejected.');
      const bytes = buffer.subarray(0, bytesRead);
      hash.update(bytes);
      await consume(bytes);
    }
    const after = await fd.stat({ bigint: true });
    const pathAfter = await lstat(path, { bigint: true });
    regular(after, path, 'AFTER'); regular(pathAfter, path, 'NAMED_AFTER');
    assert.deepEqual(fingerprint(after), fingerprint(before));
    assert.deepEqual(fingerprint(pathAfter), fingerprint(before));
    assert.equal(BigInt(total), held.size);
    const result = { path, bytes: total, sha256: hash.digest('hex'), identity: fingerprint(held) };
    if (expected) {
      assert.equal(result.bytes, expected.bytes);
      assert.equal(result.sha256, expected.sha256);
    }
    remaining(deadline);
    return result;
  } finally { await fd.close(); }
}
export async function descriptor(path, deadline = performance.now() + 30000, maxBytes = 512 * 1024 * 1024) {
  return retainedRead(path, { deadline, maxBytes }, async () => {});
}
export async function capture(path, destination, expected, deadline) {
  assert(Number.isSafeInteger(expected.bytes) && expected.bytes >= 0);
  assert(/^[a-f0-9]{64}$/.test(expected.sha256));
  await ordinaryRoot(dirname(destination));
  const fd = await open(destination,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o600);
  try {
    const source = await retainedRead(path, { deadline, maxBytes: expected.bytes, expected }, async bytes => {
      let written = 0;
      while (written < bytes.length) {
        remaining(deadline);
        const result = await fd.write(bytes, written, bytes.length - written);
        assert(result.bytesWritten > 0);
        written += result.bytesWritten;
      }
    });
    await fd.sync();
    await fd.chmod(0o400);
    remaining(deadline);
    return { source, captured: { path: destination, bytes: source.bytes, sha256: source.sha256 } };
  } finally { await fd.close(); }
}

export async function readBytes(path, deadline, maxBytes) {
  const chunks = [];
  await retainedRead(path, { deadline, maxBytes }, async bytes => chunks.push(Buffer.from(bytes)));
  return Buffer.concat(chunks);
}
