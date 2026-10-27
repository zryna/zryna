import assert from 'node:assert/strict';
import { constants, openSync, closeSync, fstatSync, lstatSync, readSync, writeFileSync } from 'node:fs';
import { createHash } from 'node:crypto';
const same = (a, b) => ['dev', 'ino', 'size', 'mtimeMs', 'ctimeMs', 'mode'].every(key => a[key] === b[key]);
export function openBounded(path, maximum) {
  const pathStat = lstatSync(path);
  assert(pathStat.isFile() && !pathStat.isSymbolicLink(), 'regular owned file');
  const fd = openSync(path, constants.O_RDONLY);
  try {
    const stat = fstatSync(fd);
    assert(same(pathStat, stat) && stat.isFile() && stat.size <= maximum, 'bounded stable file before read');
    return { fd, stat };
  } catch (error) { closeSync(fd); throw error; }
}
export function readBounded(path, maximum) {
  const { fd, stat } = openBounded(path, maximum);
  try {
    const bytes = Buffer.alloc(stat.size); let at = 0;
    while (at < bytes.length) { const n = readSync(fd, bytes, at, bytes.length - at, null); assert(n > 0); at += n; }
    assert.equal(readSync(fd, Buffer.alloc(1), 0, 1, null), 0, 'first-extra file byte');
    assert(same(stat, fstatSync(fd)), 'file identity unchanged after read'); return bytes;
  } finally { closeSync(fd); }
}
export function fingerprint(path, maximum) {
  const { fd, stat } = openBounded(path, maximum), hash = createHash('sha256'), block = Buffer.alloc(65536);
  try {
    let total = 0;
    for (;;) { const n = readSync(fd, block, 0, block.length, null); if (!n) break;
      total += n; assert(total <= maximum && total <= stat.size, 'bounded full file hash'); hash.update(block.subarray(0, n)); }
    assert.equal(total, stat.size); assert(same(stat, fstatSync(fd)), 'stable file hash');
    return { bytes: total, sha256: hash.digest('hex'), dev: stat.dev, ino: stat.ino, mode: stat.mode, mtimeMs: stat.mtimeMs, ctimeMs: stat.ctimeMs };
  } finally { closeSync(fd); }
}
export function raw(path, count, oracle) {
  const { fd, stat } = openBounded(path, 16777216), hash = createHash('sha256'), block = Buffer.alloc(65536);
  try {
    assert.equal(stat.size, count, 'metadata/raw byte agreement');
    const text = oracle.value === undefined ? null : Buffer.from(oracle.value);
    if (oracle.exactBytes !== undefined) assert.equal(count, oracle.exactBytes);
    if (oracle.maximumBytes !== undefined) assert(count <= oracle.maximumBytes);
    if (text) assert(oracle.kind === 'prefix-text' ? count <= text.length : count === text.length);
    let total = 0;
    for (;;) { const n = readSync(fd, block, 0, block.length, null); if (!n) break;
      assert(total + n <= stat.size, 'raw first-extra');
      for (let i = 0; i < n; ++i) assert.equal(block[i], text ? text[total + i] : oracle.byte, 'fixed raw oracle');
      hash.update(block.subarray(0, n)); total += n; }
    assert.equal(total, stat.size); assert(same(stat, fstatSync(fd)), 'raw identity unchanged');
    return { bytes: total, sha256: hash.digest('hex') };
  } finally { closeSync(fd); }
}
export const createJson = (path, value) => writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`, { flag: 'wx', mode: 0o600 });
