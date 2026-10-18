// Cargo may hardlink its completed executable to a deps entry. Only this fixed
// build output admits links; published files and all other materials stay ordinary().
import { constants, closeSync, fstatSync, openSync, readSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { PRODUCT, TARGET, requireValue } from './common.mjs';

const MAXIMUM = 128 * 1024 * 1024;
const state = value => ['dev','ino','size','mode','nlink','uid','gid','mtimeNs','ctimeNs']
  .map(key => value[key]);
const same = (left,right) => left.every((value,index) => value === right[index]);
const directoryFlags = constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW;

export function captureBuiltCompiler(targetRoot) {
  requireValue(process.platform === 'linux' && process.arch === 'x64' &&
    typeof targetRoot === 'string' && targetRoot === resolve(targetRoot), 'compiler build output root');
  const path = join(targetRoot,TARGET,'release',PRODUCT), handles = [], ancestors = [];
  let data, failure;
  try {
    let parent = openSync('/',directoryFlags); handles.push(parent);
    for (const name of dirname(path).slice(1).split('/')) {
      const child = openSync(`/proc/self/fd/${parent}/${name}`,directoryFlags);
      handles.push(child); ancestors.push({parent,name,child}); parent = child;
    }
    const named = `/proc/self/fd/${parent}/${PRODUCT}`;
    const fd = openSync(named,constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    handles.push(fd);
    const before = fstatSync(fd,{bigint:true});
    requireValue(before.isFile() && before.nlink > 0n && before.ino !== 0n &&
      before.size > 0n && before.size <= BigInt(MAXIMUM), 'compiler completed build output');
    data = Buffer.allocUnsafe(Number(before.size));
    let offset = 0;
    while (offset < data.length) {
      const count = readSync(fd,data,offset,Math.min(65536,data.length-offset),offset);
      requireValue(count > 0,'compiler complete build output'); offset += count;
    }
    requireValue(readSync(fd,Buffer.alloc(1),0,1,offset) === 0,'compiler build output first extra byte');
    const reopened = openSync(named,constants.O_RDONLY | constants.O_NOFOLLOW | constants.O_NONBLOCK);
    handles.push(reopened);
    requireValue(same(state(before),state(fstatSync(fd,{bigint:true}))) &&
      same(state(before),state(fstatSync(reopened,{bigint:true}))), 'compiler stable build output identity');
    for (const {parent,name,child} of ancestors) {
      const current = openSync(`/proc/self/fd/${parent}/${name}`,directoryFlags); handles.push(current);
      const held = fstatSync(child,{bigint:true}), observed = fstatSync(current,{bigint:true});
      requireValue(held.dev === observed.dev && held.ino === observed.ino, 'compiler build output ancestor');
    }
  } catch (error) { failure = error; }
  const errors = failure ? [failure] : [];
  for (const fd of handles.reverse()) {
    try { closeSync(fd); } catch (error) { errors.push(error); }
  }
  if (errors.length === 1) throw errors[0];
  if (errors.length) throw new AggregateError(errors,'compiler build output cleanup');
  return data;
}
