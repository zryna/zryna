import {
  lstatSync, readFileSync, rmdirSync, unlinkSync,
} from 'node:fs';
import { delimiter, dirname, isAbsolute, join, resolve } from 'node:path';

function reject(message) {
  throw new Error(`R423-INSTALLED-LIFECYCLE: ${message}`);
}

const DEFAULT_SYSTEM = Object.freeze({
  lstat: lstatSync, read: readFileSync, removeDirectory: rmdirSync, unlink: unlinkSync,
});

function safePath(path) {
  if (typeof path !== 'string' || path.length < 1 || path.length > 240
    || path.startsWith('qualification/') || path.includes('\\')) return false;
  const parts = path.split('/');
  return /^[A-Za-z0-9][A-Za-z0-9._@+-]*$/.test(parts[0])
    && parts.slice(1).every((part) => /^[A-Za-z0-9@][A-Za-z0-9._@+-]*$/.test(part)
      && part !== '.' && part !== '..');
}

export function cleanInstalledEnvironment(executable, workRoot, locale = 'C') {
  const bin = dirname(executable);
  const systemPath = process.platform === 'win32'
    ? [join(process.env.SystemRoot ?? 'C:\\Windows', 'System32'), process.env.SystemRoot ?? 'C:\\Windows']
    : ['/usr/bin', '/bin'];
  const env = {
    HOME: workRoot, LANG: locale, LC_ALL: locale,
    PATH: [bin, ...systemPath].join(delimiter), TMPDIR: workRoot,
  };
  if (process.platform === 'win32') Object.assign(env, {
    COMSPEC: process.env.ComSpec ?? 'C:\\Windows\\System32\\cmd.exe',
    PATHEXT: process.env.PATHEXT ?? '.COM;.EXE;.BAT;.CMD',
    SYSTEMROOT: process.env.SystemRoot ?? 'C:\\Windows', TEMP: workRoot, TMP: workRoot,
    USERPROFILE: workRoot, WINDIR: process.env.WINDIR ?? 'C:\\Windows',
  });
  return env;
}

export function exactInstalledOutput(result, expected) {
  return [Buffer.from(`${expected}\n`), Buffer.from(`${expected}\r\n`)]
    .some((value) => result.stdout.equals(value)) && result.stderr.length === 0;
}

function removableDirectories(files) {
  const paths = new Set();
  for (const { path } of files) {
    let current = dirname(path);
    while (current !== '.') {
      paths.add(current);
      current = dirname(current);
    }
  }
  return [...paths].sort((left, right) => right.split('/').length - left.split('/').length
    || right.localeCompare(left));
}

export function removeVerifiedProductionFiles(root, files, system = DEFAULT_SYSTEM) {
  if (!isAbsolute(root) || resolve(root) !== root || !Array.isArray(files) || files.length < 1) {
    reject('an absolute installed root and verified files are required for removal');
  }
  const rootEntry = system.lstat(root, { throwIfNoEntry: false });
  if (!rootEntry?.isDirectory() || rootEntry.isSymbolicLink()) reject('installed root is unsafe');
  const identities = new Set();
  for (const file of files) {
    if (!safePath(file?.path) || !Buffer.isBuffer(file?.data)) reject('removal tuple is unsafe');
    const identity = file.path.toLowerCase();
    if (identities.has(identity)) reject('removal paths collide');
    identities.add(identity);
    const installedPath = join(root, ...file.path.split('/'));
    const entry = system.lstat(installedPath, { throwIfNoEntry: false });
    if (!entry?.isFile() || entry.isSymbolicLink() || !system.read(installedPath).equals(file.data)) {
      reject('owned installation file changed before removal');
    }
  }
  for (const { path } of [...files].sort((left, right) => right.path.localeCompare(left.path))) {
    system.unlink(join(root, ...path.split('/')));
  }
  for (const path of removableDirectories(files)) {
    try {
      system.removeDirectory(join(root, ...path.split('/')));
    } catch (error) {
      if (!['ENOTEMPTY', 'EEXIST', 'ENOENT'].includes(error?.code)) throw error;
    }
  }
  try {
    system.removeDirectory(root);
  } catch (error) {
    if (!['ENOTEMPTY', 'EEXIST'].includes(error?.code)) throw error;
  }
}

export function admitReleaseTransition(current, candidate) {
  const parse = (version) => {
    const match = /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$/.exec(version);
    if (!match) reject('release transition version is invalid');
    return match.slice(1).map(Number);
  };
  const left = parse(current);
  const right = parse(candidate);
  const comparison = right.findIndex((value, index) => value !== left[index]);
  if (comparison < 0) reject('same-version replacement is not an upgrade');
  if (right[comparison] < left[comparison]) reject('release downgrade is forbidden');
  return Object.freeze({ current, candidate, operation: 'upgrade' });
}
