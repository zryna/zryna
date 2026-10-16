// Pretrusted bootstrap I/O: no-follow ancestor capabilities and independently pinned ordinary bytes.
import { closeSync, constants, fstatSync, openSync, readSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { descriptor, fail } from './launcher-policy.mjs';

const fields = ['dev', 'ino', 'size', 'mode', 'uid', 'gid', 'nlink', 'mtimeNs', 'ctimeNs'];
export const sameState = (left, right) => fields.every(key => left[key] === right[key]);
const sameIdentity = (left, right) => ['dev', 'ino', 'mode', 'uid', 'gid'].every(key => left[key] === right[key]);

export function captureFile(policy, maximum, { copy = true, executable = false, mode } = {}) {
  descriptor(policy, maximum);
  const ancestors = [];
  let fd;
  let primary;
  let result;
  try {
    let parent = openSync('/', constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
    ancestors.push({ fd: parent, state: fstatSync(parent, { bigint: true }), parent: null, name: null });
    const names = policy.path.slice(1).split('/');
    for (const name of names.slice(0, -1)) {
      const directory = openSync(`/proc/self/fd/${parent}/${name}`,
        constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
      ancestors.push({ fd: directory, state: fstatSync(directory, { bigint: true }), parent, name });
      parent = directory;
    }
    const name = names.at(-1);
    fd = openSync(`/proc/self/fd/${parent}/${name}`, constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    const state = fstatSync(fd, { bigint: true });
    if (!state.isFile() || state.uid !== BigInt(policy.ownerUid) || state.size !== BigInt(policy.bytes) ||
        state.nlink !== 1n || state.mode & 0o7022n || executable && !(state.mode & 0o111n) ||
        mode !== undefined && (state.mode & 0o7777n) !== BigInt(mode)) fail('SOURCE-MATERIAL');
    const hash = createHash('sha256');
    const buffer = Buffer.alloc(Math.min(65536, policy.bytes));
    const chunks = []; let used = 0;
    while (used < policy.bytes) {
      const size = readSync(fd, buffer, 0, Math.min(buffer.length, policy.bytes - used), used);
      if (!size) fail('SOURCE-MATERIAL');
      hash.update(buffer.subarray(0, size));
      if (copy) chunks.push(Buffer.from(buffer.subarray(0, size)));
      used += size;
    }
    const reopened = openSync(`/proc/self/fd/${parent}/${name}`,
      constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    try {
      if (!sameState(state, fstatSync(fd, { bigint: true })) || !sameState(state, fstatSync(reopened, { bigint: true })) ||
          hash.digest('hex') !== policy.sha256) fail('SOURCE-MATERIAL');
    } finally { closeSync(reopened); }
    for (const ancestor of ancestors) {
      if (!sameIdentity(ancestor.state, fstatSync(ancestor.fd, { bigint: true }))) fail('SOURCE-MATERIAL');
      if (ancestor.parent !== null) {
        const current = openSync(`/proc/self/fd/${ancestor.parent}/${ancestor.name}`,
          constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
        try { if (!sameIdentity(ancestor.state, fstatSync(current, { bigint: true }))) fail('SOURCE-MATERIAL'); }
        finally { closeSync(current); }
      }
    }
    const retained = fd; fd = undefined;
    let closed = false;
    result = Object.freeze({ bytes: copy ? Buffer.concat(chunks) : null,
      revalidate() { if (closed || !sameState(state, fstatSync(retained, { bigint: true }))) fail('SOURCE-MATERIAL'); },
      close() { if (!closed) { closed = true; closeSync(retained); } } });
  } catch (error) { primary = error; }
  const errors = primary ? [primary] : [];
  if (fd !== undefined) { try { closeSync(fd); } catch (error) { errors.push(error); } }
  for (const ancestor of ancestors.reverse()) { try { closeSync(ancestor.fd); } catch (error) { errors.push(error); } }
  if (errors.length) {
    try { result?.close(); } catch (error) { errors.push(error); }
    if (errors.length === 1) throw errors[0];
    throw new AggregateError(errors, 'PLAYGROUND-SOURCE-CLEANUP');
  }
  return result;
}
