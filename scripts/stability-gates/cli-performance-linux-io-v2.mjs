// Linux trusted-command adapter, not a source/host/threshold authority or arbitrary-code sandbox.
import assert from 'node:assert/strict';
import { closeSync, fsyncSync, lstatSync, mkdirSync, openSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { canonical, MAX_DOCUMENT, MAX_LOG, portable, readSafe, sha256 } from './input.mjs';
import { checkSelectedBytes, matrixFor } from './cli-performance-methodology-v4.mjs';
import { execute } from './process.mjs';
import { checkKnownGraphs } from './cli-performance-methodology-v4.mjs';
import { openLinuxTimeProvider } from './cli-performance-linux-provider.mjs';
import { collectLinuxRound, validateLinuxCapture } from './cli-performance-linux-round-v2.mjs';
function sameFile(root, pin, limit) {
  portable(pin.path);
  assert(Number.isSafeInteger(pin.bytes) && pin.bytes > 0 && pin.bytes <= limit);
  assert.match(pin.sha256, /^[a-f0-9]{64}$/u);
  const bytes = readSafe(root, pin.path, limit);
  assert.equal(bytes.length, pin.bytes, 'exact observed file size');
  assert.equal(sha256(bytes), pin.sha256, 'exact observed file hash');
}
function inventory(root, directory) {
  const result = [];
  let total = 0;
  let visited = 0;
  const pending = [directory];
  while (pending.length) {
    const current = pending.pop();
    assert(++visited <= 128 && current.split('/').length <= 16, 'bounded artifact walk');
    const stat = lstatSync(resolve(root, current));
    assert(!stat.isSymbolicLink(), 'no owned artifact links');
    if (stat.isDirectory()) {
      const names = readdirSync(resolve(root, current));
      assert(names.length <= 64 && result.length + pending.length + names.length <= 128);
      for (const name of names) { portable(name); pending.push(`${current}/${name}`); }
    } else {
      assert(stat.isFile(), 'regular owned artifacts');
      total += stat.size;
      assert(total <= 64 * 1024 * 1024, 'bounded fixture artifact bytes');
      result.push(current);
    }
  }
  return result.sort();
}
function retention(output) {
  assert(isAbsolute(output));
  const parent = lstatSync(dirname(output));
  assert(parent.isDirectory() && !parent.isSymbolicLink(), 'trusted private output ancestor');
  mkdirSync(output, { mode: 0o700 }); // exclusive: never replace a previous collection
  return (name, bytes) => {
    portable(name);
    assert(!name.includes('/') && Buffer.isBuffer(bytes));
    assert(bytes.length <= MAX_LOG, 'retained file budget');
    const fd = openSync(join(output, name), 'wx', 0o600);
    try { writeFileSync(fd, bytes); fsyncSync(fd); } finally { closeSync(fd); }
  };
}
function validateOracles(document, capture, oracles) {
  assert.equal(sha256(canonical(oracles)), capture.artifactOracleSha256);
  const rows = matrixFor(document, 'linux-x86_64');
  assert.deepEqual(oracles.map(row => row.selector), rows.map(row => row.id));
  for (const [index, row] of oracles.entries()) {
    assert.equal(row.manifest, rows[index].expectedResponse.manifest);
    assert(row.files.length > 0 && row.files.length <= 64);
    const prefix = `${dirname(row.manifest)}/`;
    const paths = row.files.map(pin => { portable(pin.path); assert(pin.path.startsWith(prefix)); return pin.path; });
    assert.deepEqual(paths, [...new Set(paths)].sort(), 'exact sorted artifact inventory');
    assert(paths.includes(row.manifest));
    assert(typeof row.stdout === 'string' && Buffer.byteLength(row.stdout) <= MAX_DOCUMENT);
    assert.deepEqual(JSON.parse(row.stdout), rows[index].expectedResponse);
  }
}
function stageOriginalM3(root, document) {
  const family = document.families[2];
  const reader = (path, bytes) => readSafe(root, path, bytes);
  checkKnownGraphs(document, reader);
  // Same ignored-cache convention as the existing public M3 harness. --root remains the compiler checkout.
  for (const path of ['.zryna', '.zryna/cache']) {
    const full = resolve(root, path);
    try { mkdirSync(full, { mode: 0o700 }); } catch (error) { if (error.code !== 'EEXIST') throw error; }
    const stat = lstatSync(full); assert(stat.isDirectory() && !stat.isSymbolicLink());
  }
  mkdirSync(resolve(root, family.staging.directory), { mode: 0o700 }); // owned create-only leaf
  for (const copy of family.staging.copies) {
    const pin = family.sources.find(row => row.path === copy.from);
    const bytes = reader(pin.path, pin.bytes);
    writeFileSync(resolve(root, copy.to), bytes, { flag: 'wx', mode: 0o600 });
  }
  return () => {
    const expected = family.staging.copies.map(row => row.to).sort();
    assert.deepEqual(inventory(root, family.staging.directory), expected);
    for (const copy of family.staging.copies) {
      const pin = family.sources.find(row => row.path === copy.from);
      sameFile(root, { ...pin, path: copy.to }, MAX_DOCUMENT);
    }
    rmSync(resolve(root, family.staging.directory), { recursive: true, force: false });
  };
}
export async function captureLinuxDraft({ document, capture, oracles, root, output }) {
  assert.equal(process.platform, 'linux');
  assert.equal(process.arch, 'x64');
  assert(isAbsolute(root) && root === resolve(root) && isAbsolute(output));
  assert(!relative(root, output).startsWith('.zryna/out'), 'logs outside generated bundle');
  validateLinuxCapture(document, capture);
  validateOracles(document, capture, oracles);
  const provider = openLinuxTimeProvider({ providerRoot: capture.providerRoot });
  assert.equal(capture.providerReceiptSha256, provider.binding.providerReceiptSha256, 'fresh runtime receipt binding');
  assert.deepEqual(capture.tools.time, provider.binding.executable);
  const bySelector = new Map(oracles.map(row => [row.selector, row]));
  const retain = retention(output);
  retain('artifact-oracles.json', Buffer.from(canonical(oracles)));
  retain('provider-admission.json', provider.admissionBytes);
  retain('provider-runtime-receipt.json', provider.receiptBytes);
  const environment = capture.environment;
  canonical(environment);
  const allowed = ['PATH', 'HOME', 'TMPDIR', 'CARGO_HOME', 'RUSTUP_HOME', 'RUSTUP_TOOLCHAIN', 'RUSTC'];
  for (const [key, value] of Object.entries(environment)) {
    assert(allowed.includes(key) && typeof value === 'string' && value.length < 8192,
      'explicit bounded environment, never inherit credentials');
  }
  const clearStage = stageOriginalM3(root, document);
  const checkMaterials = () => {
    provider.check(); // exact recipe, fresh receipt, package, runtime providers and executable before/after each child
    checkSelectedBytes(document, (path, bytes) => readSafe(root, path, bytes));
    checkKnownGraphs(document, (path, bytes) => readSafe(root, path, bytes));
    for (const family of document.families) if (family.staging) {
      for (const copy of family.staging.copies) {
        const pin = family.sources.find(row => row.path === copy.from);
        sameFile(root, { ...pin, path: copy.to }, MAX_DOCUMENT);
      }
    }
    for (const tool of Object.values(capture.tools)) {
      assert(isAbsolute(tool.path) && tool.path === resolve(tool.path));
      sameFile('/', { ...tool, path: tool.path.slice(1) }, 256 * 1024 * 1024);
      assert(lstatSync(tool.path).mode & 0o111, 'executable observed tool');
    }
  };
  const bundle = row => dirname(row.expectedResponse.manifest);
  const checkArtifactsAndStdout = (row, stdout) => {
    const oracle = bySelector.get(row.id);
    assert(stdout.equals(Buffer.from(oracle.stdout)), 'independently frozen exact public JSON bytes');
    assert.deepEqual(inventory(root, bundle(row)), oracle.files.map(pin => pin.path));
    for (const pin of oracle.files) sameFile(root, pin, 64 * 1024 * 1024);
  };
  return collectLinuxRound(document, capture, {
    root, retain, checkMaterials, finishRound: clearStage, nowNs: () => process.hrtime.bigint(),
    execute: (file, args, timeoutMs) => execute(file, args, { root, timeoutMs, env: environment }),
    beforeAttempt: row => {
      try { lstatSync(resolve(root, bundle(row))); }
      catch (error) { if (error.code === 'ENOENT') return; throw error; }
      assert.fail('owned output already exists; never overwrite or delete it');
    },
    checkArtifactsAndStdout,
    afterAttempt: row => { checkArtifactsAndStdout(row, Buffer.from(bySelector.get(row.id).stdout));
      rmSync(resolve(root, bundle(row)), { recursive: true, force: false }); },
  });
}
