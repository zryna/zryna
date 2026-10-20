// Narrow producer capture of Cargo's output alias and Rustup's multicall aliases.
// The resulting installation and ordinary input reader still require single-link files.
import {
  chmodSync, closeSync, constants, fstatSync, lstatSync, openSync, readdirSync,
  readSync, realpathSync, writeFileSync,
} from 'node:fs';
import { basename, dirname, isAbsolute, join } from 'node:path';
import { bytes, requireValue, sha256 } from './canonical.mjs';
import { ordinaryBytes, pinBytes } from '../../tests/native-provider-public-install/commands.mjs';

const MAX = 256 * 1024 * 1024;
const KEYS = ['dev', 'ino', 'mode', 'nlink', 'size', 'mtimeNs', 'ctimeNs'];
const EXACT = new Set(['dev', 'ino', 'mtimeNs', 'ctimeNs']);
const identity = stat => Object.fromEntries(KEYS.map(key =>
  [key, EXACT.has(key) ? String(stat[key]) : Number(stat[key])]));
const equal = (left, right) => JSON.stringify(identity(left)) === JSON.stringify(identity(right));
const statPath = path => lstatSync(path, { bigint: true });
const statFd = fd => fstatSync(fd, { bigint: true });
const SHIMS = new Set(['rustup', 'cargo', 'rustc', 'rustdoc', 'rustfmt', 'cargo-fmt',
  'cargo-clippy', 'clippy-driver', 'cargo-miri', 'rust-analyzer', 'rust-gdb', 'rust-gdbgui', 'rust-lldb', 'rls']);
function direct(path) {
  requireValue(isAbsolute(path) && realpathSync.native(path) === path, 'direct producer capture path');
}
function stableRead(path, expected) {
  const before = statPath(path);
  requireValue(before.isFile() && !before.isSymbolicLink() && before.size > 0 && before.size <= MAX
    && before.nlink >= 1 && before.nlink <= SHIMS.size, 'bounded producer executable source');
  if (expected) requireValue(equal(before, expected), 'producer source identity changed');
  const fd = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    requireValue(equal(before, statFd(fd)), 'producer source changed before read');
    const buffer = Buffer.alloc(Number(before.size) + 1);
    let count = 0;
    while (count < buffer.length) {
      const read = readSync(fd, buffer, count, buffer.length - count, null);
      if (!read) break;
      count += read;
    }
    const body = buffer.subarray(0, count);
    requireValue(body.length === Number(before.size) && equal(before, statFd(fd))
      && equal(before, statPath(path)), 'producer source changed during read');
    return { body, stat: identity(before), sha256: sha256(body) };
  } finally { closeSync(fd); }
}
function observeAliases(source, aliasDirectory, kind, sourceStat) {
  direct(source); direct(aliasDirectory);
  requireValue(['cargo-cli', 'rustup-shim'].includes(kind), 'producer capture kind');
  const directory = statPath(aliasDirectory);
  requireValue(directory.isDirectory() && !directory.isSymbolicLink(), 'producer alias directory');
  const names = readdirSync(aliasDirectory).sort();
  requireValue(names.length <= 8192, 'bounded producer alias directory');
  const executable = process.platform === 'win32' ? '.exe' : '';
  requireValue(kind === 'cargo-cli'
    ? basename(source) === 'zryna' + executable && aliasDirectory === join(dirname(source), 'deps')
    : basename(source) === 'rustup' + executable && aliasDirectory === dirname(source),
  'producer source/alias topology');
  const aliases = [{ path: source, ...stableRead(source, sourceStat) }];
  for (const name of names) {
    const path = join(aliasDirectory, name);
    if (path === source) continue;
    const st = statPath(path);
    if (String(st.dev) !== sourceStat.dev || String(st.ino) !== sourceStat.ino) continue;
    const approved = kind === 'cargo-cli' ? /^zryna-[a-f0-9]{16}(?:\.exe)?$/.test(name)
      : SHIMS.has(name.replace(/\.exe$/, ''));
    requireValue(approved && st.isFile() && !st.isSymbolicLink(), 'unapproved producer hardlink alias');
    aliases.push({ path, ...stableRead(path, sourceStat) });
  }
  requireValue(aliases.length === sourceStat.nlink && equal(directory, statPath(aliasDirectory)),
    'producer hardlinks are not fully explained by stable owned aliases');
  const digest = aliases[0].sha256;
  requireValue(aliases.every(item => item.sha256 === digest), 'producer alias bytes differ');
  return { directory: { path: aliasDirectory, ...identity(directory) }, aliases };
}

export function captureProducerExecutable({ source, destination, aliasDirectory, kind }) {
  direct(source); direct(aliasDirectory); direct(dirname(destination));
  requireValue(isAbsolute(destination), 'absolute create-only producer capture');
  const sourceStat = identity(statPath(source));
  const observed = observeAliases(source, aliasDirectory, kind, sourceStat);
  const body = observed.aliases[0].body;
  writeFileSync(destination, body, { flag: 'wx', mode: 0o700 }); chmodSync(destination, 0o755);
  const captured = ordinaryBytes(destination);
  requireValue(captured.equals(body), 'producer capture bytes differ');
  const proof = { kind, source: { path: source, ...sourceStat, sha256: sha256(body) },
    aliasDirectory: observed.directory,
    aliases: observed.aliases.map(({ path, stat, sha256: digest }) => ({ path, ...stat, sha256: digest })),
    capture: pinBytes(destination, captured), captureNlink: lstatSync(destination).nlink,
    copyCountedAsRebuild: false };
  verifyProducerExecutableCapture(proof);
  return proof;
}

export function verifyProducerExecutableCapture(proof) {
  const observed = observeAliases(proof.source.path, proof.aliasDirectory.path, proof.kind, proof.source);
  requireValue(equal(proof.aliasDirectory, observed.directory)
    && bytes(observed.aliases.map(({ path, stat, sha256: digest }) => ({ path, ...stat, sha256: digest })))
      .equals(bytes(proof.aliases)), 'producer source/alias proof changed');
  requireValue(sha256(ordinaryBytes(proof.capture.path)) === proof.source.sha256
    && proof.capture.sha256 === proof.source.sha256, 'producer capture changed');
}
