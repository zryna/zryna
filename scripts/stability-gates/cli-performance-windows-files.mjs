import assert from 'node:assert/strict';
import { closeSync, fsyncSync, lstatSync, mkdirSync, openSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, posix, resolve } from 'node:path';
import { canonical, MAX_DOCUMENT, MAX_LOG, portable, readSafe, sha256 } from './input.mjs';
import { checkKnownGraphs, matrixFor } from './cli-performance-methodology-v4.mjs';
export function sameFile(root, pin, maximum) {
  portable(pin.path); assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= maximum);
  assert.match(pin.sha256, /^[a-f0-9]{64}$/u);
  const bytes = readSafe(root, pin.path, maximum);
  assert.equal(bytes.length, pin.bytes); assert.equal(sha256(bytes), pin.sha256);
}
export function inventory(root, directory) {
  const files = [], pending = [directory]; let total = 0, visited = 0;
  while (pending.length) {
    const current = pending.pop(); portable(current);
    assert(++visited <= 128 && current.split('/').length <= 16, 'bounded owned inventory');
    const stat = lstatSync(resolve(root, current)); assert(!stat.isSymbolicLink(), 'no owned links');
    if (stat.isDirectory()) {
      const names = readdirSync(resolve(root, current));
      assert(names.length <= 64 && files.length + pending.length + names.length <= 128);
      for (const name of names) { portable(name); pending.push(`${current}/${name}`); }
    } else {
      assert(stat.isFile()); total += stat.size; assert(total <= 64 * 1024 * 1024);
      files.push(current);
    }
  }
  return files.sort();
}
export function retention(output) {
  assert(isAbsolute(output)); const parent = lstatSync(dirname(output));
  assert(parent.isDirectory() && !parent.isSymbolicLink(), 'trusted private output ancestor');
  mkdirSync(output, { mode: 0o700 });
  return (name, bytes) => {
    portable(name); assert(!name.includes('/') && Buffer.isBuffer(bytes) && bytes.length <= MAX_LOG);
    const fd = openSync(join(output, name), 'wx', 0o600);
    try { writeFileSync(fd, bytes); fsyncSync(fd); } finally { closeSync(fd); }
  };
}
export function validateWindowsOracles(document, capture, oracles) {
  assert.equal(sha256(canonical(oracles)), capture.artifactOracleSha256, 'externally frozen complete oracles');
  const rows = matrixFor(document, 'windows-x86_64');
  assert.deepEqual(oracles.map(row => row.selector), rows.map(row => row.id));
  for (const [index, oracle] of oracles.entries()) {
    assert.equal(oracle.manifest, rows[index].expectedResponse.manifest);
    assert(oracle.files.length > 0 && oracle.files.length <= 64);
    const prefix = `${posix.dirname(oracle.manifest)}/`;
    const paths = oracle.files.map(pin => {
      portable(pin.path); assert(pin.path.startsWith(prefix));
      assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= 64 * 1024 * 1024);
      assert.match(pin.sha256, /^[a-f0-9]{64}$/u); return pin.path;
    });
    assert.deepEqual(paths, [...new Set(paths)].sort(), 'complete sorted unique artifact inventory');
    assert(paths.includes(oracle.manifest));
    assert(typeof oracle.stdout === 'string' && Buffer.byteLength(oracle.stdout) <= MAX_DOCUMENT);
    assert.deepEqual(JSON.parse(oracle.stdout), rows[index].expectedResponse);
    assert(typeof oracle.stderr === 'string' && Buffer.byteLength(oracle.stderr) <= MAX_DOCUMENT);
  }
}
export function stageOriginalWindowsM3(root, document) {
  checkKnownGraphs(document, (path, maximum) => readSafe(root, path, maximum));
  const family = document.families[2];
  for (const path of ['.zryna', '.zryna/cache']) {
    try { mkdirSync(resolve(root, path), { mode: 0o700 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
    const stat = lstatSync(resolve(root, path)); assert(stat.isDirectory() && !stat.isSymbolicLink());
  }
  mkdirSync(resolve(root, family.staging.directory), { mode: 0o700 });
  for (const copy of family.staging.copies) {
    const pin = family.sources.find(row => row.path === copy.from);
    writeFileSync(resolve(root, copy.to), readSafe(root, pin.path, pin.bytes), { flag: 'wx', mode: 0o600 });
  }
  return () => {
    assert.deepEqual(inventory(root, family.staging.directory), family.staging.copies.map(row => row.to).sort());
    for (const copy of family.staging.copies) sameFile(root,
      { ...family.sources.find(row => row.path === copy.from), path: copy.to }, MAX_DOCUMENT);
    rmSync(resolve(root, family.staging.directory), { recursive: true, force: false });
  };
}
