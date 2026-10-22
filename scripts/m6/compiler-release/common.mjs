import { spawnSync } from 'node:child_process';
import { constants, closeSync, fstatSync, openSync, readFileSync, writeFileSync } from 'node:fs';
import { bytes, exactKeys, orderedPaths, parseCanonical, requireValue, sha256 }
  from '../../distribution/canonical.mjs';
export { bytes, exactKeys, orderedPaths, parseCanonical, requireValue, sha256 };
export const REPO = 'zryna/zryna', TAG = 'v0.1.0-playground-compiler.3';
export const WORKFLOW = '.github/workflows/playground-compiler-release.yml';
export const TARGET = 'x86_64-unknown-linux-gnu', PRODUCT = 'zryna-playground-compiler';
export const REF = `refs/tags/${TAG}`;
export const SHA = value => typeof value === 'string' && /^[a-f0-9]{64}$/.test(value);
export const OID = value => typeof value === 'string' && /^[a-f0-9]{40}$/.test(value);
export function id(value) {
  requireValue(Number.isSafeInteger(Number(value)) && /^[1-9][0-9]{0,15}$/.test(String(value)), 'compiler release ID');
  return Number(value);
}
export function command(executable, args, { cwd, env = process.env, timeout = 120000,
  maximum = 2 * 1024 * 1024, spawn = spawnSync } = {}) {
  const result = spawn(executable, args, { cwd, env, timeout, shell: false,
    maxBuffer: maximum + 1, killSignal: 'SIGKILL', encoding: null });
  const stdout = Buffer.from(result.stdout ?? []), stderr = Buffer.from(result.stderr ?? []);
  const record = { executable, args, status: result.status, signal: result.signal,
    errorCode: result.error?.code ?? null, stdoutBytes: stdout.length, stderrBytes: stderr.length,
    stdoutSha256: sha256(stdout), stderrSha256: sha256(stderr) };
  if (result.error || result.status !== 0 || result.signal || stdout.length + stderr.length > maximum) {
    const error = new Error(`M6-COMPILER-RELEASE-COMMAND: ${executable} ${args[0]}`);
    error.proof = { record, stdout, stderr }; throw error;
  }
  return { record, stdout, stderr };
}
export function ordinary(path, maximum = 2 * 1024 * 1024) {
  const descriptor = openSync(path, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
  try {
    const before = fstatSync(descriptor, { bigint: true });
    requireValue(before.isFile() && before.nlink === 1n && before.size >= 0n &&
      before.size <= BigInt(maximum), 'compiler release ordinary file');
    const data = readFileSync(descriptor), after = fstatSync(descriptor, { bigint: true });
    requireValue(['dev','ino','mode','nlink','size','uid','gid','mtimeNs','ctimeNs']
      .every(key => before[key] === after[key]) && BigInt(data.length) === before.size,
    'compiler release stable file');
    return data;
  } finally { closeSync(descriptor); }
}
export const describe = (path, data) => ({ path, bytes: data.length, sha256: sha256(data) });
export function writeNew(path, data) {
  writeFileSync(path, data, { flag: 'wx', mode: 0o600 });
}
export function receiptSource(source) {
  return { repository: source.repository, commit: source.commit, tree: source.tree };
}
