import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { chmodSync, lstatSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { pathToFileURL } from 'node:url';
import test from 'node:test';
import { captureFile } from './launcher-capture.mjs';
import { snapshot } from './launcher-snapshot.mjs';
const pin = path => ({ path, bytes: statSync(path).size, ownerUid: statSync(path).uid,
  sha256: createHash('sha256').update(readFileSync(path)).digest('hex') });
function release(path) {
  // Explicit test-owned recovery only; production cleanup must preserve foreign entries.
  const visit = item => {
    if (!lstatSync(item).isDirectory()) return;
    chmodSync(item, 0o700);
    for (const name of readdirSync(item)) visit(join(item, name));
  };
  visit(path); rmSync(path, { recursive: true });
}

test('actual retained no-follow source capture rejects replacements and independently wrong pins', () => {
  const root = mkdtempSync(join(tmpdir(), 'playground-source-capture-'));
  try {
    const path = join(root, 'source.mjs'); writeFileSync(path, 'export const value=1;', { mode: 0o600 });
    const expected = pin(path);
    const captured = captureFile(expected, 128, { mode: 0o600 });
    assert.equal(captured.bytes.toString(), 'export const value=1;');
    assert.throws(() => captureFile({ ...expected, sha256: '0'.repeat(64) }, 128), /SOURCE-MATERIAL/);
    const alias = join(root, 'alias.mjs'); symlinkSync(path, alias);
    assert.throws(() => captureFile({ ...expected, path: alias }, 128));
    writeFileSync(path, 'export const value=2;');
    assert.throws(() => captured.revalidate(), /SOURCE-MATERIAL/);
    captured.close();
  } finally { rmSync(root, { recursive: true }); }
});

test('private source copy executes exact captured bytes and explicit cleanup removes its owned inventory', async () => {
  const original = Buffer.from('export const captured=42;');
  const staged = snapshot(new Map([['examples/playground/restricted/daemon/launcher-core.mjs', original]]));
  original.fill(0);
  assert.equal((await import(pathToFileURL(staged.modulePath()).href)).captured, 42);
  assert.equal(statSync(staged.path).mode & 0o777, 0o500);
  staged.verify(); staged.cleanup();
  assert.throws(() => statSync(staged.path), /ENOENT/);
});

test('unknown snapshot entries are retained, and unsafe paths reject before private materialization', () => {
  for (const path of ['../escape', '/absolute', 'a/../../escape', 'a//b']) {
    assert.throws(() => snapshot(new Map([[path, Buffer.from('x')]])), /SOURCE-POLICY/);
  }
  const staged = snapshot(new Map([['main.mjs', Buffer.from('export const captured=1;')]]));
  chmodSync(staged.path, 0o700);
  const foreign = join(staged.path, 'foreign'); writeFileSync(foreign, 'retain');
  assert.throws(() => staged.verify(), /SOURCE-CLEANUP/);
  assert.throws(() => staged.cleanup(), /SOURCE-CLEANUP/);
  assert.equal(readFileSync(foreign).toString(), 'retain');
  release(staged.path);
});

test('separately bounded compiler bytes stage as a private executable without mutable metadata aliases', () => {
  const binary = new Uint8Array(readFileSync('/usr/bin/false'));
  const expected = createHash('sha256').update(binary).digest('hex');
  const staged = snapshot(new Map([['main.mjs', Buffer.from('export const captured=1;')]]), binary);
  try {
    binary.fill(0);
    const compiler = staged.compiler();
    assert.equal(compiler.sha256, expected);
    assert.equal(statSync(compiler.path).mode & 0o777, 0o500);
    assert.equal(createHash('sha256').update(readFileSync(compiler.path)).digest('hex'), expected);
    staged.verify();
  } finally { staged.cleanup(); }
});
