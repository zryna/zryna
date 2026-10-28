// Actual default filesystem IO only. No native process, Cargo or source authority.
import assert from 'node:assert/strict';
import test from 'node:test';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, realpathSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join, sep } from 'node:path';
import { MAX_DOCUMENT, MAX_LOG } from '../scripts/stability-gates/input.mjs';
import { captureWriter, prepareCapture } from '../scripts/stability-gates/lifecycle-owner-capture-io.mjs';

function temporary(body) {
  const base = realpathSync(mkdtempSync(join(tmpdir(), 'lifecycle-capture-io-')));
  try { return body(base); } finally { rmSync(base, { recursive: true, force: true }); }
}
test('native preparation rejects existing output without touching its sentinel', () => temporary(base => {
  const root = join(base, 'source'), output = join(base, 'output');
  mkdirSync(root); mkdirSync(output); writeFileSync(join(output, 'sentinel'), 'original');
  assert.throws(() => prepareCapture({ root, output }));
  assert.equal(readFileSync(join(output, 'sentinel'), 'utf8'), 'original');
  assert.equal(existsSync(join(output, 'target')), false);
}));
test('noncanonical parent traversal rejects before output creation on either host', () => temporary(base => {
  const root = join(base, 'source'), parent = join(base, 'parent'); mkdirSync(root); mkdirSync(parent);
  const output = parent + sep + '..' + sep + 'capture';
  assert.throws(() => prepareCapture({ root, output }), /canonical existing output parent/);
  assert.equal(existsSync(join(base, 'capture')), false);
}));
test('native capture writer preserves original bytes and rejects traversal and oversize', () => temporary(base => {
  const write = captureWriter(base), original = Buffer.from('original raw stream\n');
  write('commands/stdout.raw', original);
  assert.throws(() => write('commands/stdout.raw', Buffer.from('replacement')));
  assert(readFileSync(join(base, 'commands', 'stdout.raw')).equals(original));
  assert.throws(() => write('../escape.raw', Buffer.from('escaped')));
  assert.throws(() => write('oversized.json', { text: 'x'.repeat(MAX_DOCUMENT + 1) }), /bounded retained capture file/);
  assert.throws(() => write('oversized.raw', Buffer.alloc(MAX_LOG + 1)), /bounded retained capture file/);
  assert.equal(existsSync(join(base, 'oversized.json')), false); assert.equal(existsSync(join(base, 'oversized.raw')), false);
}));
test('native capture writer retains distribution license paths with SemVer build suffixes', () => temporary(base => {
  const path = 'licenses-before/licenses/rust/toml-0.9.12+spec-1.1.0/LICENSE-MIT';
  const data = Buffer.from('inert license fixture');
  captureWriter(base)(path, data);
  assert(readFileSync(join(base, path)).equals(data));
}));
