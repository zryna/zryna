import { constants } from 'node:fs';
import { lstat, mkdir, open, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';

export const TIMING_DIRECTORY = 'zryna-npm-bootstrap-timing';
export const TIMER_NAMES = Object.freeze([
  'npm', 'command:ci', 'npm-ci:rm', 'idealTree', 'reify', 'reify:loadTrees',
  'reify:audit', 'auditReport:getReport', 'auditReport:init', 'reify:unpack', 'reify:build',
]);
const object = value => value !== null && typeof value === 'object' && !Array.isArray(value);
const unavailable = () => new Error('npm bootstrap timing unavailable');

export function numericTimers(bytes) {
  try {
    if (!Buffer.isBuffer(bytes) || bytes.length > 1024 * 1024) throw unavailable();
    const raw = JSON.parse(new TextDecoder('utf-8', { fatal: true }).decode(bytes));
    if (!object(raw) || !object(raw.timers) || !object(raw.unfinishedTimers)
      || !object(raw.metadata) || raw.metadata.version !== '10.9.4') throw unavailable();
    if (!Object.hasOwn(raw.timers, 'command:ci')) return null;
    if (!Array.isArray(raw.metadata.command) || raw.metadata.command[0] !== 'ci') throw unavailable();
    const result = {};
    for (const name of TIMER_NAMES) {
      if (Object.hasOwn(raw.unfinishedTimers, name)) throw unavailable();
      if (!Object.hasOwn(raw.timers, name)) continue;
      const duration = raw.timers[name];
      if (!Number.isSafeInteger(duration) || duration < 0) throw unavailable();
      result[name] = duration;
    }
    if (!Object.hasOwn(result, 'npm') || !Object.hasOwn(result, 'reify')) throw unavailable();
    return result;
  } catch { throw unavailable(); }
}

async function trustedDirectory(directory) {
  if (typeof directory !== 'string' || !path.isAbsolute(directory)) throw unavailable();
  const resolved = path.resolve(directory);
  let current = path.parse(resolved).root;
  for (const segment of resolved.slice(current.length).split(path.sep).filter(Boolean)) {
    current = path.join(current, segment);
    const info = await lstat(current);
    if (!info.isDirectory() || info.isSymbolicLink()) throw unavailable();
  }
  return resolved;
}

export async function prepareTimingDirectory(runnerTemp) {
  try {
    const parent = await trustedDirectory(runnerTemp);
    const directory = path.join(parent, TIMING_DIRECTORY);
    await mkdir(directory, { mode: 0o700 }); // Exclusive: existing paths must fail.
    await writeFile(path.join(directory, '.prepared'), 'npm-timing-v1\n', { flag: 'wx', mode: 0o600 });
  } catch { throw unavailable(); }
}

async function boundedFile(file, maxBytes) {
  const info = await lstat(file);
  if (!info.isFile() || info.isSymbolicLink() || info.nlink !== 1 || info.size > maxBytes) throw unavailable();
  const handle = await open(file, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0));
  try {
    const before = await handle.stat();
    if (!before.isFile() || before.dev !== info.dev || before.ino !== info.ino
      || before.size > maxBytes || before.nlink !== 1) throw unavailable();
    const buffer = Buffer.alloc(before.size + 1);
    let bytesRead = 0;
    while (bytesRead < buffer.length) {
      const chunk = await handle.read(buffer, bytesRead, buffer.length - bytesRead, bytesRead);
      if (chunk.bytesRead === 0) break;
      bytesRead += chunk.bytesRead;
    }
    const after = await handle.stat();
    if (bytesRead !== before.size || before.size !== after.size || before.mtimeMs !== after.mtimeMs) throw unavailable();
    return buffer.subarray(0, bytesRead);
  } finally { await handle.close(); }
}

export async function collectTiming(runnerTemp) {
  try {
    const parent = await trustedDirectory(runnerTemp);
    const directory = await trustedDirectory(path.join(parent, TIMING_DIRECTORY));
    const marker = await boundedFile(path.join(directory, '.prepared'), 32);
    if (marker.toString('utf8') !== 'npm-timing-v1\n') throw unavailable();
    const entries = await readdir(directory, { withFileTypes: true });
    if (entries.length > 256) throw unavailable();
    const candidates = entries.filter(entry => entry.name.endsWith('-timing.json'));
    if (candidates.length > 32) throw unavailable();
    const records = [];
    for (const entry of candidates) {
      if (!entry.isFile() || entry.isSymbolicLink()) throw unavailable();
      const record = numericTimers(await boundedFile(path.join(directory, entry.name), 1024 * 1024));
      if (record) records.push(record);
    }
    if (records.length !== 1) throw unavailable();
    return records[0];
  } catch { throw unavailable(); }
}
