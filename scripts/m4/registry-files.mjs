import { createHash } from 'node:crypto';
import {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync, opendirSync,
} from 'node:fs';
import path from 'node:path';

export const digest = bytes => createHash('sha256').update(bytes).digest('hex');
export function fail(message) { throw new Error(`M4 registry: ${message}`); }
export function portablePath(relative) {
  if (typeof relative !== 'string' || relative.length > 240 ||
      !/^[A-Za-z0-9_.-][A-Za-z0-9_./{}-]*$/.test(relative) ||
      relative.split('/').some(part => !part || part === '.' || part === '..')) {
    fail('unsafe portable path');
  }
  return relative;
}
export function operationalSourcePath(relative) {
  portablePath(relative);
  const parts = relative.split('/');
  const files = ['Cargo.lock', 'Cargo.toml', 'CONTRIBUTING.md', 'LICENSE', 'NOTICE', 'package.json',
    'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'rust-toolchain.toml', 'zryna.workspace.json'];
  const directories = ['adapters', 'apps', 'crates', 'examples', 'schemas', 'scripts', 'spec', 'tests'];
  const workflow = parts[0] === '.github' && parts[1] === 'workflows' && parts.length > 2;
  const policyDocument = ['docs/DOCUMENTATION_BUNDLES.md', 'docs/CODE_NAVIGATION.md'].includes(relative);
  const namespace = parts.length === 1 ? files.includes(relative) : directories.includes(parts[0]) || workflow || policyDocument;
  const derived = ['tests/m4-conformance-v1.json', 'scripts/m4/registry-authority.mjs',
    'scripts/m4/limit-contract-authority.mjs', 'apps/zryna/src/m4_support.json'];
  if (!namespace || derived.includes(relative.toLowerCase()) ||
      parts.some(part => ['.zryna', '.git', 'node_modules', 'target'].includes(part.toLowerCase()))) {
    fail('generated or undeclared operational source path');
  }
  return relative;
}
function identity(a, b) {
  return a.dev === b.dev && a.ino === b.ino && a.mode === b.mode;
}
function state(a, b) {
  return identity(a, b) && a.size === b.size &&
    a.mtimeNs === b.mtimeNs && a.ctimeNs === b.ctimeNs;
}
function ancestors(file) {
  const parents = [];
  let parent = path.dirname(path.resolve(file));
  for (;;) {
    const before = lstatSync(parent, { bigint: true });
    if (!before.isDirectory() || before.isSymbolicLink()) fail('linked or non-directory ancestor');
    parents.push([parent, before]);
    const next = path.dirname(parent);
    if (next === parent) break;
    parent = next;
  }
  return parents;
}
export function readerOpenFlags(platform = process.platform, values = constants) {
  if (platform !== 'win32' && (!values.O_NOFOLLOW || !values.O_NONBLOCK)) {
    fail('Unix bounded reads require O_NOFOLLOW and O_NONBLOCK');
  }
  // Windows Node lacks these POSIX flags; retained identity/reparse checks remain mandatory.
  return values.O_RDONLY | (values.O_NOFOLLOW ?? 0) | (values.O_NONBLOCK ?? 0);
}
export function readStableFile(file, maximum, label = file) {
  const parents = ancestors(file);
  const before = lstatSync(file, { bigint: true });
  if (!before.isFile() || before.isSymbolicLink()) fail(`${label} is not a regular no-follow file`);
  if (before.size > BigInt(maximum)) fail(`${label} exceeds byte limit`);
  let fd;
  try {
    fd = openSync(file, readerOpenFlags());
    const opened = fstatSync(fd, { bigint: true });
    if (!opened.isFile() || !identity(before, opened) || opened.size > BigInt(maximum)) {
      fail(`${label} opened identity or size changed`);
    }
    const bytes = Buffer.alloc(Number(opened.size) + 1);
    let length = 0;
    while (length < bytes.length) {
      const count = readSync(fd, bytes, length, bytes.length - length, null);
      if (!count) break;
      length += count;
    }
    if (length !== Number(opened.size) || length > maximum) fail(`${label} length changed`);
    const after = fstatSync(fd, { bigint: true });
    if (!state(opened, after) || !state(after, lstatSync(file, { bigint: true }))) {
      fail(`${label} changed during same-handle read`);
    }
    for (const [parent, original] of parents) {
      if (!identity(original, lstatSync(parent, { bigint: true }))) fail('ancestor identity changed');
    }
    return bytes.subarray(0, length);
  } finally {
    if (fd !== undefined) closeSync(fd);
  }
}
export function parseCanonical(bytes, label = 'registry') {
  let document;
  try { document = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes)); }
  catch { fail(`${label} is not strict UTF-8 JSON`); }
  if (!bytes.equals(Buffer.from(`${JSON.stringify(document, null, 2)}\n`))) {
    fail(`${label} is not canonical JSON (duplicates, alternate trivia and records reject)`);
  }
  return document;
}
export function inventory(root, relative) {
  const budget = { entries: 0 };
  return walk(root, relative, 0, budget).sort();
}
function walk(root, relative, depth, budget) {
  portablePath(relative);
  if (depth > 32) fail('fixture depth exceeds bound');
  const directory = path.join(root, relative);
  const parents = ancestors(path.join(directory, 'entry'));
  const before = lstatSync(directory, { bigint: true });
  if (!before.isDirectory() || before.isSymbolicLink()) fail('fixture directory is linked or invalid');
  const folded = new Set();
  const files = [];
  let handle;
  try {
    // One buffered entry bounds allocation before the explicit global entry count.
    handle = opendirSync(directory, { bufferSize: 1 });
    for (;;) {
      const entry = handle.readSync();
      if (entry === null) break;
      if (++budget.entries > 512) fail('fixture inventory entries exceed bound');
      const key = entry.name.toLowerCase();
      if (folded.has(key)) fail('fixture case collision');
      folded.add(key);
      const child = `${relative}/${entry.name}`;
      portablePath(child);
      if (entry.isSymbolicLink()) fail('linked fixture entry');
      if (entry.isDirectory()) files.push(...walk(root, child, depth + 1, budget));
      else if (entry.isFile()) files.push(child);
      else fail('nonregular fixture entry');
    }
  } finally {
    if (handle !== undefined) handle.closeSync();
  }
  if (!state(before, lstatSync(directory, { bigint: true }))) fail('fixture directory changed');
  for (const [parent, original] of parents) {
    if (!identity(original, lstatSync(parent, { bigint: true }))) fail('fixture ancestor identity changed');
  }
  return files;
}
