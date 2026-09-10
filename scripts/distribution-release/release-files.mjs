import { createHash } from 'node:crypto';
import {
  closeSync, constants, fstatSync, lstatSync, openSync, readSync, readdirSync, writeSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';

export const MAX_RELEASE_FILE = 2 * 1024 * 1024 * 1024;
export const MAX_RELEASE_DOCUMENT = 16 * 1024 * 1024;
const CHUNK = 1024 * 1024;
const SAFE_NAME = /^[A-Za-z0-9][A-Za-z0-9._-]{0,159}$/;

function reject(message) {
  throw new Error(`R406-RELEASE-FILE: ${message}`);
}

export function releaseName(name) {
  if (typeof name !== 'string' || !SAFE_NAME.test(name) || basename(name) !== name) {
    reject('release file name is not one portable path segment');
  }
  return name;
}

function sameIdentity(left, right) {
  return left.dev === right.dev && left.ino === right.ino && left.size === right.size
    && left.mtimeMs === right.mtimeMs && left.mode === right.mode;
}

export function openReleaseFile(root, name, maximum = MAX_RELEASE_FILE) {
  if (!isAbsolute(root) || resolve(root) !== root) reject('release root must be absolute and normalized');
  releaseName(name);
  if (!Number.isSafeInteger(maximum) || maximum < 1 || maximum > MAX_RELEASE_FILE) {
    reject('release file bound is invalid');
  }
  const path = join(root, name);
  const before = lstatSync(path);
  if (!before.isFile() || before.isSymbolicLink() || before.size < 1 || before.size > maximum) {
    reject(`${name} must be one direct regular file within its byte bound`);
  }
  const noFollow = constants.O_NOFOLLOW ?? 0;
  const descriptor = openSync(path, constants.O_RDONLY | noFollow);
  try {
    const opened = fstatSync(descriptor);
    if (!opened.isFile() || !sameIdentity(before, opened)) {
      reject(`${name} identity changed while opening`);
    }
    return { descriptor, name, path, metadata: opened };
  } catch (error) {
    closeSync(descriptor);
    throw error;
  }
}

export function closeReleaseFile(file) {
  closeSync(file.descriptor);
}

export function revalidateReleaseFile(file) {
  const after = fstatSync(file.descriptor);
  if (!sameIdentity(file.metadata, after)) reject(`${file.name} changed while reading`);
}

export function hashReleaseFile(file) {
  const hash = createHash('sha256');
  const buffer = Buffer.allocUnsafe(CHUNK);
  let offset = 0;
  while (offset < file.metadata.size) {
    const count = readSync(
      file.descriptor, buffer, 0, Math.min(buffer.length, file.metadata.size - offset), offset,
    );
    if (count === 0) reject(`${file.name} ended before its retained size`);
    hash.update(buffer.subarray(0, count));
    offset += count;
  }
  revalidateReleaseFile(file);
  return { path: file.name, size: file.metadata.size, sha256: hash.digest('hex') };
}

export function readReleaseFile(root, name, maximum = MAX_RELEASE_DOCUMENT) {
  const file = openReleaseFile(root, name, maximum);
  try {
    const bytes = Buffer.allocUnsafe(file.metadata.size);
    let offset = 0;
    while (offset < bytes.length) {
      const count = readSync(file.descriptor, bytes, offset, bytes.length - offset, offset);
      if (count === 0) reject(`${name} ended before its retained size`);
      offset += count;
    }
    revalidateReleaseFile(file);
    return bytes;
  } finally {
    closeReleaseFile(file);
  }
}

export function compareReleaseFiles(leftRoot, rightRoot, name, maximum = MAX_RELEASE_FILE) {
  const left = openReleaseFile(leftRoot, name, maximum);
  let right;
  try {
    right = openReleaseFile(rightRoot, name, maximum);
    if (left.metadata.size !== right.metadata.size) reject(`${name} replica sizes differ`);
    const leftBuffer = Buffer.allocUnsafe(CHUNK);
    const rightBuffer = Buffer.allocUnsafe(CHUNK);
    const hash = createHash('sha256');
    let offset = 0;
    while (offset < left.metadata.size) {
      const length = Math.min(CHUNK, left.metadata.size - offset);
      const leftCount = readSync(left.descriptor, leftBuffer, 0, length, offset);
      const rightCount = readSync(right.descriptor, rightBuffer, 0, length, offset);
      if (leftCount !== length || rightCount !== length
        || !leftBuffer.subarray(0, length).equals(rightBuffer.subarray(0, length))) {
        reject(`${name} replica bytes differ at or before offset ${offset}`);
      }
      hash.update(leftBuffer.subarray(0, length));
      offset += length;
    }
    revalidateReleaseFile(left);
    revalidateReleaseFile(right);
    return { path: name, size: left.metadata.size, sha256: hash.digest('hex') };
  } finally {
    closeReleaseFile(left);
    if (right) closeReleaseFile(right);
  }
}

export function exactReleaseNames(root, expected) {
  if (!isAbsolute(root) || resolve(root) !== root) reject('release root must be absolute and normalized');
  const metadata = lstatSync(root);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) reject('release root must be a direct directory');
  const actual = readdirSync(root, { withFileTypes: true }).map((entry) => {
    if (!entry.isFile() || entry.isSymbolicLink()) reject('release root contains a non-file entry');
    return releaseName(entry.name);
  }).sort();
  const wanted = [...expected].map(releaseName).sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) reject('release file inventory differs');
}

export function exactReleaseDirectories(root, expected) {
  if (!isAbsolute(root) || resolve(root) !== root) reject('release root must be absolute and normalized');
  const metadata = lstatSync(root);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) reject('release root must be a direct directory');
  const actual = readdirSync(root, { withFileTypes: true }).map((entry) => {
    if (!entry.isDirectory() || entry.isSymbolicLink()) reject('release root contains a non-directory entry');
    return releaseName(entry.name);
  }).sort();
  const wanted = [...expected].map(releaseName).sort();
  if (JSON.stringify(actual) !== JSON.stringify(wanted)) reject('release directory inventory differs');
}

function validateOutputRoot(root) {
  if (!isAbsolute(root) || resolve(root) !== root) {
    reject('release output root must be absolute and normalized');
  }
  const metadata = lstatSync(root);
  if (!metadata.isDirectory() || metadata.isSymbolicLink()) {
    reject('release output root must be a direct directory');
  }
}

export function createReleaseFile(root, name, bytes) {
  validateOutputRoot(root);
  if (!Buffer.isBuffer(bytes) || bytes.length < 1 || bytes.length > MAX_RELEASE_FILE) {
    reject('release output must be one non-empty bounded Buffer');
  }
  releaseName(name);
  const path = join(root, name);
  if (dirname(path) !== root) reject('release output escaped its root');
  const descriptor = openSync(path, constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY, 0o600);
  try {
    let offset = 0;
    while (offset < bytes.length) {
      const count = writeSync(descriptor, bytes, offset);
      if (count === 0) reject(`${name} could not be fully written`);
      offset += count;
    }
  } finally {
    closeSync(descriptor);
  }
  return path;
}

export function copyReleaseFile(sourceRoot, outputRoot, name, maximum = MAX_RELEASE_FILE) {
  validateOutputRoot(outputRoot);
  const source = openReleaseFile(sourceRoot, name, maximum);
  const targetPath = join(outputRoot, releaseName(name));
  if (dirname(targetPath) !== outputRoot) reject('release output escaped its root');
  let target;
  try {
    target = openSync(targetPath, constants.O_CREAT | constants.O_EXCL | constants.O_WRONLY, 0o600);
    const buffer = Buffer.allocUnsafe(CHUNK);
    let offset = 0;
    while (offset < source.metadata.size) {
      const count = readSync(
        source.descriptor, buffer, 0, Math.min(CHUNK, source.metadata.size - offset), offset,
      );
      if (count === 0) reject(`${name} ended before its retained size`);
      let written = 0;
      while (written < count) {
        const writeCount = writeSync(target, buffer, written, count - written);
        if (writeCount === 0) reject(`${name} could not be fully written`);
        written += writeCount;
      }
      offset += count;
    }
    revalidateReleaseFile(source);
  } finally {
    if (target !== undefined) closeSync(target);
    closeReleaseFile(source);
  }
}
