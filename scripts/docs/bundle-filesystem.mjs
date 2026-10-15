import { constants } from 'node:fs';
import { lstat, mkdir, open, readdir } from 'node:fs/promises';
import path from 'node:path';

const NO_FOLLOW = constants.O_NOFOLLOW ?? 0;

export function fail(message) {
  throw new Error(`invalid documentation bundle: ${message}`);
}

export async function readBoundedRegular(filePath, maxBytes) {
  const metadata = await lstat(filePath);
  if (metadata.isSymbolicLink() || !metadata.isFile()) fail(`${filePath} is not a regular file`);
  if (metadata.size > maxBytes) fail(`${filePath} exceeds ${maxBytes} bytes`);
  let handle;
  try {
    handle = await open(filePath, constants.O_RDONLY | NO_FOLLOW);
    const before = await handle.stat();
    const bytes = await handle.readFile();
    const after = await handle.stat();
    if (
      before.dev !== metadata.dev ||
      before.ino !== metadata.ino ||
      before.size !== metadata.size ||
      before.dev !== after.dev ||
      before.ino !== after.ino ||
      before.size !== after.size ||
      before.mtimeMs !== after.mtimeMs ||
      before.ctimeMs !== after.ctimeMs
    ) {
      fail(`${filePath} changed while it was read`);
    }
    return bytes;
  } finally {
    await handle?.close();
  }
}

export async function ensureSafeOutputParent(workspaceRoot, outputPath) {
  const allowedRoot = path.resolve(workspaceRoot, '.zryna', 'out', 'docs');
  const resolvedOutput = path.resolve(outputPath);
  const relative = path.relative(allowedRoot, resolvedOutput);
  if (relative === '' || relative.startsWith('..') || path.isAbsolute(relative)) {
    fail('output must be a child of .zryna/out/docs');
  }
  let current = workspaceRoot;
  for (const segment of ['.zryna', 'out', 'docs', ...relative.split(path.sep).slice(0, -1)]) {
    current = path.join(current, segment);
    try {
      const metadata = await lstat(current);
      if (metadata.isSymbolicLink() || !metadata.isDirectory()) fail(`${current} is not a real directory`);
    } catch (error) {
      if (error.code !== 'ENOENT') throw error;
      await mkdir(current);
    }
  }
  return resolvedOutput;
}

export async function scanFiles(root, relative = '', files = []) {
  const entries = await readdir(path.join(root, relative), { withFileTypes: true });
  entries.sort((left, right) => left.name.localeCompare(right.name, 'en'));
  for (const entry of entries) {
    const child = relative ? path.posix.join(relative, entry.name) : entry.name;
    const metadata = await lstat(path.join(root, ...child.split('/')));
    if (metadata.isSymbolicLink() || (!metadata.isFile() && !metadata.isDirectory())) {
      fail(`bundle entry ${child} is not a regular file or directory`);
    }
    if (metadata.isDirectory()) await scanFiles(root, child, files);
    else files.push(child);
    if (files.length > 514) fail('bundle file budget exceeded');
  }
  return files;
}

