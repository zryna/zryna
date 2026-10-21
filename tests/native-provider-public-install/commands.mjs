// Evidence for actual, bounded source-producer and installed CLI processes.
import { spawnSync } from 'node:child_process';
import {
  closeSync, constants, fstatSync, lstatSync, openSync, readFileSync, realpathSync, writeFileSync,
} from 'node:fs';
import { isAbsolute, join } from 'node:path';
import { bytes, requireValue, sha256 } from '../../scripts/distribution/canonical.mjs';

export function ordinaryBytes(path, maximum = 256 * 1024 * 1024) {
  requireValue(isAbsolute(path) && realpathSync.native(path) === path, 'direct regular input path');
  const before = lstatSync(path);
  requireValue(before.isFile() && !before.isSymbolicLink() && before.nlink === 1
    && before.size >= 1 && before.size <= maximum, 'bounded ordinary input');
  const fd = openSync(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const opened = fstatSync(fd), data = readFileSync(fd), after = fstatSync(fd), named = lstatSync(path);
    for (const key of ['dev', 'ino', 'mode', 'nlink', 'size', 'mtimeMs', 'ctimeMs']) {
      requireValue([opened, after, named].every(value => value[key] === before[key]),
        'input identity changed while reading');
    }
    requireValue(data.length === before.size, 'whole input length');
    return data;
  } finally { closeSync(fd); }
}

export const pinBytes = (path, data) => ({ path, bytes: data.length, sha256: sha256(data) });
export function saveEvidence(root, name, value) {
  const path = join(root, name), data = Buffer.isBuffer(value) ? value : bytes(value);
  writeFileSync(path, data, { flag: 'wx', mode: 0o600 });
  return pinBytes(path, data);
}

export function executeRecorded(root, name, executable, args, cwd, env, {
  timeoutMs = 90_000, maximum = 16 * 1024 * 1024,
} = {}) {
  const startedAt = new Date().toISOString();
  saveEvidence(root, name + '.command.json', { executable, args, cwd, startedAt, timeoutMs,
    environmentNames: Object.keys(env).sort(), shell: false });
  const result = spawnSync(executable, args, { cwd, env, shell: false, windowsHide: true,
    encoding: null, timeout: timeoutMs, maxBuffer: maximum, killSignal: 'SIGTERM' });
  const stdout = saveEvidence(root, name + '.stdout', result.stdout ?? Buffer.alloc(0));
  const stderr = saveEvidence(root, name + '.stderr', result.stderr ?? Buffer.alloc(0));
  const observation = { executable, args, cwd, startedAt, finishedAt: new Date().toISOString(),
    exitCode: result.status, signal: result.signal, errorCode: result.error?.code ?? null,
    stdout, stderr, ownerExitObserved: result.status !== null,
    descendantCleanupClaimed: false, timeoutMeansCleanupUnconfirmed: true };
  saveEvidence(root, name + '.result.json', observation);
  requireValue(!result.error && result.signal === null
    && stdout.bytes + stderr.bytes <= maximum, 'command transport/timeout; preserve partial evidence');
  return { ...observation, stdoutBytes: result.stdout, stderrBytes: result.stderr };
}

export function successful(observation) {
  requireValue(observation.exitCode === 0, 'actual command failed; inspect retained whole output');
  return observation.stdoutBytes;
}
