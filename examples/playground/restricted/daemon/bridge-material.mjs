// Retained no-follow host material capture; independent expected hashes are operator inputs.
import { closeSync, constants, fstatSync, openSync, readSync } from 'node:fs';
import { isAbsolute, posix } from 'node:path';
import { createHash } from 'node:crypto';
import { exact, fail, isHash } from '../limits.mjs';

const fields = ['dev', 'ino', 'size', 'mode', 'uid', 'gid', 'mtimeNs', 'ctimeNs'];
const same = (left, right) => fields.every(key => left[key] === right[key]);

export function capture(policy, executable) {
  exact(policy, ['path', 'bytes', 'sha256', 'ownerUid']);
  const maximum = executable ? 268435456 : 262144;
  if (process.platform !== 'linux' || process.arch !== 'x64' || !isAbsolute(policy.path) ||
      posix.normalize(policy.path) !== policy.path || !/^\/[A-Za-z0-9._/-]+$/.test(policy.path) ||
      !Number.isSafeInteger(policy.bytes) || policy.bytes < 1 || policy.bytes > maximum ||
      !isHash(policy.sha256) || !Number.isInteger(policy.ownerUid) ||
      policy.ownerUid < 0 || policy.ownerUid > 4294967295) fail('HOST-APPROVAL');
  const ancestors = [];
  let fd;
  try {
    let parent = openSync('/', constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
    ancestors.push(parent);
    const parts = policy.path.slice(1).split('/');
    for (const part of parts.slice(0, -1)) {
      parent = openSync(`/proc/self/fd/${parent}/${part}`,
        constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
      ancestors.push(parent);
    }
    fd = openSync(`/proc/self/fd/${parent}/${parts.at(-1)}`,
      constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const state = fstatSync(fd, { bigint: true });
    if (!state.isFile() || state.uid !== BigInt(policy.ownerUid) || state.size !== BigInt(policy.bytes) ||
        state.mode & 0o22n || executable && !(state.mode & 0o111n)) fail('HOST-MATERIAL');
    const hash = createHash('sha256');
    const bytes = Buffer.alloc(Math.min(policy.bytes, 65536));
    let offset = 0;
    while (offset < policy.bytes) {
      const count = readSync(fd, bytes, 0, Math.min(bytes.length, policy.bytes - offset), offset);
      if (!count) fail('HOST-MATERIAL');
      hash.update(bytes.subarray(0, count)); offset += count;
    }
    if (!same(state, fstatSync(fd, { bigint: true })) || hash.digest('hex') !== policy.sha256) fail('HOST-MATERIAL');
    const retained = fd; fd = undefined;
    return { fd: retained, path: policy.path, sha256: policy.sha256, state };
  } finally {
    if (fd !== undefined) closeSync(fd);
    for (const descriptor of ancestors.reverse()) closeSync(descriptor);
  }
}

export function stable(material) {
  if (!same(material.state, fstatSync(material.fd, { bigint: true }))) fail('HOST-MATERIAL');
}
