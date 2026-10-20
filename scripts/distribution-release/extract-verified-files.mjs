// Shared verified-byte extraction only; this module grants no archive/release admission.
import { closeSync, lstatSync, mkdirSync, openSync, writeFileSync } from 'node:fs';
import { dirname, isAbsolute, join, resolve } from 'node:path';

const DEFAULT_SYSTEM = Object.freeze({
  close: closeSync, lstat: lstatSync, make: mkdirSync, open: openSync, write: writeFileSync,
});
function reject(message) {
  throw new Error(`R406-INSTALLED-ACCEPTANCE: ${message}`);
}

export function safePath(path) {
  if (typeof path !== 'string' || path.length < 1 || path.length > 240
    || path.startsWith('qualification/') || path.includes('\\')) return false;
  const parts = path.split('/');
  return /^[A-Za-z0-9][A-Za-z0-9._@+-]*$/.test(parts[0])
    && parts.slice(1).every((part) => /^[A-Za-z0-9@][A-Za-z0-9._@+-]*$/.test(part)
      && part !== '.' && part !== '..');
}

function requireDirectory(system, path) {
  const entry = system.lstat(path, { throwIfNoEntry: false });
  if (!entry?.isDirectory() || entry.isSymbolicLink()) reject('installation ancestor is unsafe');
}

function createAncestors(system, root, path) {
  let current = root;
  for (const part of dirname(path).split('/')) {
    if (part === '.') continue;
    current = join(current, part);
    const entry = system.lstat(current, { throwIfNoEntry: false });
    if (entry === undefined) system.make(current, { recursive: false, mode: 0o700 });
    requireDirectory(system, current);
  }
}

export function extractVerifiedProductionFiles(root, files, system = DEFAULT_SYSTEM) {
  if (!isAbsolute(root) || resolve(root) !== root || !Array.isArray(files) || files.length < 1) {
    reject('an absolute isolated root and verified files are required');
  }
  system.make(root, { recursive: false, mode: 0o700 });
  requireDirectory(system, root);
  const identities = new Set();
  for (const file of files) {
    if (!safePath(file?.path) || !Buffer.isBuffer(file?.data)
      || ![0o644, 0o755].includes(file?.mode)) reject('verified file tuple is unsafe');
    const identity = file.path.toLowerCase();
    if (identities.has(identity)) reject('verified file paths collide');
    identities.add(identity);
    createAncestors(system, root, file.path);
    const destination = join(root, ...file.path.split('/'));
    if (!resolve(destination).startsWith(`${root}${process.platform === 'win32' ? '\\' : '/'}`)) {
      reject('verified file escaped the installation root');
    }
    let descriptor;
    try {
      descriptor = system.open(destination, 'wx', file.mode);
      system.write(descriptor, file.data);
    } finally {
      if (descriptor !== undefined) system.close(descriptor);
    }
    const installed = system.lstat(destination);
    if (!installed.isFile() || installed.isSymbolicLink() || installed.size !== file.data.length) {
      reject('installed entry is not the verified ordinary file');
    }
  }
  return root;
}

