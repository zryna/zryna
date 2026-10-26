import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { closeSync, constants, fstatSync, lstatSync, openSync, readSync, realpathSync } from 'node:fs';
import { dirname, isAbsolute, join, parse, relative, sep } from 'node:path';

export const requireValue = (value, message) => assert(value, `S418 release capture: ${message}`);
export const digest = bytes => createHash('sha256').update(bytes).digest('hex');
export const buildArguments = target => ['build', '--locked', '--offline', '--release', '--target', target,
  '-p', 'zryna', '--bin', 'zryna'];
export const TARGETS = Object.freeze({ 'x86_64-unknown-linux-gnu': ['linux', 'ubuntu-24.04'],
  'x86_64-pc-windows-msvc': ['win32', 'windows-2022'] });
export function outside(root, path) {
  const rel = relative(root, path);
  return rel !== '' && (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel));
}
export function cargoConfigurationPaths(root, cargoHome) {
  requireValue(isAbsolute(root) && isAbsolute(cargoHome), 'absolute Cargo configuration roots');
  const directories = [cargoHome];
  for (let ancestor = root; ; ancestor = dirname(ancestor)) {
    directories.push(join(ancestor, '.cargo'));
    if (ancestor === dirname(ancestor)) break;
  }
  return [...new Set(directories.flatMap(directory => ['config', 'config.toml'].map(name => join(directory, name))))];
}
export function inspectCargoConfiguration(root, cargoHome) {
  return cargoConfigurationPaths(root, cargoHome).map(path => ({ path,
    present: Boolean(lstatSync(path, { throwIfNoEntry: false })) }));
}
export function regularPath(path) {
  requireValue(isAbsolute(path), 'absolute path');
  let cursor = parse(path).root;
  for (const part of relative(cursor, path).split(sep).filter(Boolean)) {
    cursor = join(cursor, part);
    requireValue(!lstatSync(cursor).isSymbolicLink(), 'no symbolic path components');
  }
  requireValue(lstatSync(path).isFile(), 'regular file');
  return realpathSync(path);
}
function same(a, b) {
  return ['dev', 'ino', 'size', 'mtimeMs', 'ctimeMs', 'mode'].every(key => a[key] === b[key]);
}
// At most 64 KiB of file data is buffered. Tool/CLI caps are evidence limits only.
export function pinFile(path, limit = 512 * 1024 * 1024, gitBlob = false) {
  regularPath(path);
  const before = lstatSync(path);
  requireValue((gitBlob || before.size > 0) && before.size <= limit, 'file byte bound');
  const fd = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const opened = fstatSync(fd);
    requireValue(same(before, opened), 'file replaced before read');
    const sha = createHash('sha256');
    const blob = createHash('sha1').update(`blob ${before.size}\0`);
    const buffer = Buffer.alloc(64 * 1024);
    let total = 0;
    let prefix = Buffer.alloc(0);
    for (;;) {
      const n = readSync(fd, buffer, 0, buffer.length, null);
      if (!n) break;
      total += n;
      requireValue(total <= before.size, 'file grew');
      const bytes = buffer.subarray(0, n);
      sha.update(bytes); if (gitBlob) blob.update(bytes);
      if (!prefix.length) prefix = Buffer.from(bytes.subarray(0, 128));
    }
    requireValue(total === before.size && same(opened, fstatSync(fd)) && same(opened, lstatSync(path)),
      'file changed while reading');
    return { path, bytes: total, sha256: sha.digest('hex'), mode: (before.mode & 0o777).toString(8),
      prefixHex: prefix.toString('hex'), ...(gitBlob ? { gitBlob: blob.digest('hex') } : {}) };
  } finally { closeSync(fd); }
}
const common = ['PATH', 'HOME', 'CARGO_HOME', 'RUSTUP_HOME', 'LANG', 'LC_ALL'];
const linux = ['TMPDIR'];
const windows = ['SystemRoot', 'WINDIR', 'TEMP', 'TMP', 'USERPROFILE', 'INCLUDE', 'LIB', 'LIBPATH',
  'WindowsSdkDir', 'WindowsSDKVersion', 'UniversalCRTSdkDir', 'UCRTVersion', 'VCToolsInstallDir', 'VCToolsVersion'];
const hosted = ['GITHUB_ACTIONS', 'GITHUB_REPOSITORY', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT', 'RUNNER_OS',
  'RUNNER_ARCH', 'ImageOS', 'ImageVersion'];
export function controlledEnvironment(environment, { target, targetDirectory, tools }) {
  requireValue(Object.hasOwn(TARGETS, target), 'supported target');
  const win = TARGETS[target][0] === 'win32';
  const required = [...common, ...(win ? windows : linux), ...hosted];
  requireValue(Object.keys(environment).length === required.length &&
    required.every(key => Object.hasOwn(environment, key)), 'exact explicit environment keys');
  for (const value of Object.values(environment)) requireValue(typeof value === 'string' &&
    value.length > 0 && value.length <= 32768 && !value.includes('\0'), 'bounded environment value');
  requireValue(isAbsolute(environment.CARGO_HOME) && isAbsolute(environment.RUSTUP_HOME) &&
    isAbsolute(environment.HOME), 'absolute explicit cache/home roots');
  requireValue(environment.GITHUB_ACTIONS === 'true' && environment.GITHUB_REPOSITORY === 'zryna/zryna'
    && environment.RUNNER_ARCH === 'X64' && environment.RUNNER_OS === (win ? 'Windows' : 'Linux'),
  'hosted environment projections (not authentication)');
  requireValue(environment.ImageOS === (win ? 'win22' : 'ubuntu24'), 'pinned runner image family');
  requireValue(/^[1-9][0-9]*$/.test(environment.GITHUB_RUN_ID) &&
    /^[1-9][0-9]*$/.test(environment.GITHUB_RUN_ATTEMPT), 'run identifiers');
  if (win) requireValue(/^10\.0\.[0-9]+\.0\\?$/.test(environment.WindowsSDKVersion) &&
    /^10\.0\.[0-9]+\.0$/.test(environment.UCRTVersion), 'SDK version projection');
  return { ...environment, CARGO_TARGET_DIR: targetDirectory, CARGO_BUILD_JOBS: '2', CARGO_INCREMENTAL: '0',
    CARGO_NET_OFFLINE: 'true', RUSTUP_TOOLCHAIN: '1.97.1', RUSTC: tools.rustc, CC: tools.cc,
    [target === 'x86_64-pc-windows-msvc' ? 'CARGO_TARGET_X86_64_PC_WINDOWS_MSVC_LINKER'
      : 'CARGO_TARGET_X86_64_UNKNOWN_LINUX_GNU_LINKER']: tools.linker };
}
export function validateVersions(observations, target) {
  const text = key => observations[key].stdout.trim();
  requireValue(text('cargo') === 'cargo 1.97.1 (c980f4866 2026-06-30)', 'actual exact Cargo 1.97.1');
  requireValue(text('rustc').split(/\r?\n/)[0] === 'rustc 1.97.1 (8bab26f4f 2026-07-14)' &&
    text('rustc').split(/\r?\n/).includes(`host: ${target}`), 'actual exact rustc and host');
  requireValue(text('node') === 'v22.22.1', 'actual Node 22.22.1');
  requireValue(process.version === 'v22.22.1', 'capture runtime Node 22.22.1');
}
export function validateBinary(pin, target) {
  const bytes = Buffer.from(pin.prefixHex, 'hex');
  if (target === 'x86_64-unknown-linux-gnu') {
    requireValue(bytes.length >= 20 && bytes.subarray(0, 6).equals(Buffer.from([127, 69, 76, 70, 2, 1])) &&
      bytes.readUInt16LE(18) === 62 && (parseInt(pin.mode, 8) & 0o111), 'Linux executable ELF x64');
  } else requireValue(bytes.length >= 64 && bytes.toString('ascii', 0, 2) === 'MZ', 'Windows executable MZ');
}
export function sdkFiles(environment) {
  const version = environment.WindowsSDKVersion.replace(/\\$/, '');
  return [join(environment.WindowsSdkDir, 'Include', version, 'shared', 'sdkddkver.h'),
    join(environment.UniversalCRTSdkDir, 'Include', environment.UCRTVersion, 'ucrt', 'corecrt.h')];
}
