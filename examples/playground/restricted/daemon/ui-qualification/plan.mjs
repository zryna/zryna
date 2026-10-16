// Image-pinned first probe; this plan never declares corpus or final accessibility acceptance.
import { closeSync, constants, fstatSync, openSync, readSync } from 'node:fs';
import { decode, fail, sha256 } from './wire.mjs';

export const initialPlan = Object.freeze({ format: 'zryna.playground-ui-qualification-plan.v1', version: 1,
  phase: 'initial-abort-static-ax', transport: 'operator-stdio-browser-loopback-http-v1',
  listener: 'tcp4:127.0.0.1:same-origin-port',
  abortProbe: 'requestfailed-and-response-close-before-dispatch', lifetimeMs: 300000, cleanupMs: 2000 });
export const planBytes = () => Buffer.from(JSON.stringify(initialPlan));
export function readImagePlan() {
  let directory; let file;
  try {
    directory = openSync('/resources', constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
    const parent = fstatSync(directory, { bigint: true });
    if (parent.uid !== 0n || parent.gid !== 0n || (parent.mode & 0o7777n) !== 0o555n) fail();
    file = openSync(`/proc/self/fd/${directory}/ui-plan.json`, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const before = fstatSync(file, { bigint: true });
    if (!before.isFile() || before.uid !== 0n || before.gid !== 0n || before.nlink !== 1n ||
        (before.mode & 0o7777n) !== 0o444n || before.size < 1n || before.size > 4096n) fail();
    const buffer = Buffer.alloc(4097);
    const count = readSync(file, buffer, 0, buffer.length, 0);
    const after = fstatSync(file, { bigint: true });
    if (BigInt(count) !== before.size || ['dev', 'ino', 'mode', 'uid', 'gid', 'nlink', 'size', 'mtimeNs', 'ctimeNs']
      .some(name => before[name] !== after[name]) ||
      JSON.stringify(decode(buffer.subarray(0, count))) !== JSON.stringify(initialPlan)) fail();
    return sha256(buffer.subarray(0, count));
  } finally {
    if (file !== undefined) closeSync(file);
    if (directory !== undefined) closeSync(directory);
  }
}
