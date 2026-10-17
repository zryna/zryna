import { closeSync, constants, fstatSync, openSync, readSync, readdirSync } from 'node:fs';
import { isAbsolute, resolve } from 'node:path';
import { requireValue } from '../../distribution/canonical.mjs';

const KEYS = ['dev', 'ino', 'mode', 'nlink', 'uid', 'gid', 'size', 'mtimeNs', 'ctimeNs'];
const stable = (before, after) => KEYS.every(key => before[key] === after[key]);

export function captureCompilerCacheTree(root) {
  requireValue(typeof root === 'string' && isAbsolute(root) && resolve(root) === root,
    'compiler cache root');
  const descriptors = [];
  const directories = [];
  const retainedFiles = [];
  const files = new Map();
  let total = 0;
  let primary;
  try {
    let parent = openSync('/', constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
    descriptors.push(parent);
    for (const part of root.slice(1).split('/')) {
      parent = openSync(`/proc/self/fd/${parent}/${part}`,
        constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
      descriptors.push(parent);
    }
    function walk(directory, prefix, depth) {
      requireValue(depth <= 16, 'compiler cache path depth');
      const before = fstatSync(directory, { bigint: true });
      directories.push({ descriptor: directory, before, path: prefix });
      requireValue(directories.length <= 4096, 'compiler cache directory count');
      const names = readdirSync(`/proc/self/fd/${directory}`).sort();
      requireValue(names.length <= 4096, 'compiler cache directory budget');
      for (const name of names) {
        const path = prefix ? `${prefix}/${name}` : name;
        requireValue(path.length <= 256 && /^[A-Za-z0-9@+._-]+$/.test(name)
          && !/[. ]$/.test(name), 'compiler cache path');
        const descriptor = openSync(`/proc/self/fd/${directory}/${name}`,
          constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
        descriptors.push(descriptor);
        const first = fstatSync(descriptor, { bigint: true });
        if (first.isDirectory()) walk(descriptor, path, depth + 1);
        else {
          requireValue(first.isFile() && first.nlink === 1n && first.size >= 0n && first.size <= 67108864n
            && !(first.mode & 0o6000n) && files.size < 4098, 'compiler cache ordinary source');
          total += Number(first.size);
          requireValue(total <= 66 * 1024 * 1024, 'compiler cache byte budget');
          const data = Buffer.alloc(Number(first.size) + 1);
          let offset = 0;
          while (offset < data.length) {
            const count = readSync(descriptor, data, offset, Math.min(65536, data.length - offset), offset);
            if (!count) break;
            offset += count;
          }
          requireValue(offset === Number(first.size)
            && stable(first, fstatSync(descriptor, { bigint: true })), 'compiler cache stable source');
          retainedFiles.push({ descriptor, before: first });
          files.set(path, { path, mode: Number(first.mode & 0o777n), data: data.subarray(0, offset) });
        }
      }
      requireValue(stable(before, fstatSync(directory, { bigint: true })), 'compiler cache stable directory');
    }
    walk(parent, '', 0);
    for (const captured of [...directories, ...retainedFiles]) {
      requireValue(stable(captured.before, fstatSync(captured.descriptor, { bigint: true })),
        'compiler cache retained source tree');
    }
  } catch (error) { primary = error; }
  const errors = primary ? [primary] : [];
  for (const descriptor of descriptors.reverse()) {
    try { closeSync(descriptor); } catch (error) { errors.push(error); }
  }
  if (errors.length === 1) throw errors[0];
  if (errors.length) throw new AggregateError(errors, 'compiler material cache cleanup');
  return { files, directories: directories.map(value => value.path).filter(Boolean).sort() };
}
