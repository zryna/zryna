import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readFileSync, readdirSync, rmSync, symlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { createM4GateTemporaryEnvironment } from '../scripts/run-m4-conformance.mjs';

function fixture(run) {
  const parent = mkdtempSync(path.join(tmpdir(), 'zryna-m4-gate-temp-'));
  try {
    const root = path.join(parent, 'evidence');
    mkdirSync(root, { mode: 0o700 });
    run(root, parent);
  } finally { rmSync(parent, { recursive: true, force: true }); }
}
const keys = ['TMPDIR', 'TEMP', 'TMP'];
const parentEnvironment = () => Object.fromEntries(keys.map(key => [key, process.env[key]]));
const child = String.raw`
const assert = require('node:assert/strict'), fs = require('node:fs'), path = require('node:path');
const [marker, selected] = process.argv.slice(1);
for (const name of ['TMPDIR', 'TEMP', 'TMP']) assert.equal(process.env[name], selected);
assert.equal(require('node:os').tmpdir(), selected);
const directory = path.join(selected, 'fixed-pid-4242-evidence');
fs.mkdirSync(directory);
fs.writeFileSync(path.join(directory, 'original.txt'), marker, { flag: 'wx' });
`;

test('the shared-temp control reproduces the fixed-token collision without replacing prior evidence', () => fixture((root, parent) => {
  const shared = path.join(parent, 'shared-temp');
  mkdirSync(shared);
  const environment = { ...process.env, ...Object.fromEntries(keys.map(key => [key, shared])) };
  const invoke = marker => spawnSync(process.execPath, ['-e', child, marker, shared], {
    env: environment, shell: false, windowsHide: true, timeout: 5000, maxBuffer: 4096, encoding: 'utf8',
  });
  assert.equal(invoke('original').status, 0);
  const repeated = invoke('replacement');
  assert.equal(repeated.error, undefined);
  assert.equal(repeated.signal, null);
  assert.notEqual(repeated.status, 0);
  assert.match(repeated.stderr, /EEXIST/);
  assert.equal(readFileSync(path.join(shared, 'fixed-pid-4242-evidence', 'original.txt'), 'utf8'), 'original');
}));

test('separate gates replay the same fixed evidence token in real children without overwriting either result', () => fixture(root => {
  const originalEnvironment = parentEnvironment(), results = [];
  for (const [gate, marker] of [['m0-full', 'first invocation'], ['m3-full', 'second invocation']]) {
    const environment = createM4GateTemporaryEnvironment(root, gate, 'core');
    assert.deepEqual(Object.keys(environment).sort(), [...keys].sort());
    assert.equal(path.dirname(environment.TMP), path.dirname(root), 'scratch is an evidence sibling');
    const result = spawnSync(process.execPath, ['-e', child, marker, environment.TMP], {
      env: { ...process.env, ...environment }, shell: false, windowsHide: true,
      timeout: 5000, maxBuffer: 4096, encoding: 'utf8',
    });
    assert.equal(result.error, undefined);
    assert.equal(result.signal, null);
    assert.equal(result.status, 0, result.stderr);
    results.push({ file: path.join(environment.TMP, 'fixed-pid-4242-evidence', 'original.txt'), marker });
  }
  assert.notEqual(results[0].file, results[1].file);
  for (const { file, marker } of results) assert.equal(readFileSync(file, 'utf8'), marker);
  assert.deepEqual(readdirSync(root), [], 'scratch is excluded from raw and retained evidence artifacts');
  assert.deepEqual(parentEnvironment(), originalEnvironment, 'the parent environment remains unchanged');
}));

test('reusing a gate namespace fails create-only and preserves prior bytes', () => fixture(root => {
  const environment = createM4GateTemporaryEnvironment(root, 'm0-full', 'core');
  const file = path.join(environment.TMP, 'original.txt');
  writeFileSync(file, 'preserved', { flag: 'wx' });
  assert.throws(() => createM4GateTemporaryEnvironment(root, 'm0-full', 'core'), { code: 'EEXIST' });
  assert.equal(readFileSync(file, 'utf8'), 'preserved');
  assert.deepEqual(readdirSync(environment.TMP), ['original.txt']);
}));

test('invalid selectors and non-directory or linked evidence roots are refused before scratch creation', () => fixture((root, parent) => {
  for (const id of ['', '.', '..', '../escape', '/absolute', 'C:drive', 'back\\slash', 'Upper', 'a'.repeat(65)]) {
    assert.throws(() => createM4GateTemporaryEnvironment(root, id, 'core'));
  }
  assert.throws(() => createM4GateTemporaryEnvironment('relative', 'm0-full', 'core'));
  const file = path.join(parent, 'file');
  writeFileSync(file, 'unchanged', { flag: 'wx' });
  assert.throws(() => createM4GateTemporaryEnvironment(file, 'm0-full', 'core'));
  const alias = path.join(parent, 'alias');
  symlinkSync(root, alias, process.platform === 'win32' ? 'junction' : 'dir');
  assert.throws(() => createM4GateTemporaryEnvironment(alias, 'm0-full', 'core'));
  assert.equal(readFileSync(file, 'utf8'), 'unchanged');
  assert.deepEqual(readdirSync(root), []);
  assert.deepEqual(readdirSync(parent).sort(), ['alias', 'evidence', 'file']);
}));

test('browser and installed lanes retain their original temp environment and create no scratch paths', () => fixture((root, parent) => {
  const before = parentEnvironment();
  for (const [lane, gate] of [['browser', 'pinned-browser-three-proof'], ['installed', 'installed-qualification']]) {
    assert.deepEqual(createM4GateTemporaryEnvironment(root, gate, lane), {});
  }
  for (const lane of [undefined, 'other']) {
    assert.throws(() => createM4GateTemporaryEnvironment(root, 'm0-full', lane));
  }
  assert.deepEqual(parentEnvironment(), before);
  assert.deepEqual(readdirSync(parent), ['evidence']);
}));
