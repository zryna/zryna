// Known-inventory private copies; cleanup never removes an uncertain or foreign object.
import { chmodSync, closeSync, constants, fstatSync, fsyncSync, lstatSync, mkdirSync,
  mkdtempSync, openSync, readSync, readdirSync, rmdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { createHash } from 'node:crypto';
import { fail, sha256 } from './launcher-policy.mjs';
import { sameState } from './launcher-capture.mjs';

const state = path => lstatSync(path, { bigint: true });
const sameObject = (left, right) => left.dev === right.dev && left.ino === right.ino;
export function snapshot(selectedFiles, compilerBytes = null) {
  if (!(selectedFiles instanceof Map) || selectedFiles.size < 1 || selectedFiles.size > 64 ||
      selectedFiles.has('resources/compiler')) fail('SOURCE-POLICY');
  const files = new Map(selectedFiles);
  let executablePin = null;
  if (compilerBytes !== null) {
    if (!(compilerBytes instanceof Uint8Array) || compilerBytes.length < 1 || compilerBytes.length > 268435456) fail('SOURCE-POLICY');
    executablePin = Object.freeze({ bytes: compilerBytes.length, sha256: sha256(compilerBytes) });
    files.set('resources/compiler', compilerBytes);
  }
  if (!(files instanceof Map) || files.size < 1 || files.size > 65) fail('SOURCE-POLICY');
  let total = 0;
  for (const [relative, bytes] of files) {
    if (typeof relative !== 'string' || relative.length > 240 || relative.split('/').length > 16 ||
        !/^[A-Za-z0-9._/-]+$/.test(relative) || relative.split('/').some(part => !part || part === '.' || part === '..') ||
        !(bytes instanceof Uint8Array) || bytes.length < 1 || bytes.length > (relative === 'resources/compiler' ? 268435456 : 131072) ||
        (total += relative === 'resources/compiler' ? 0 : bytes.length) > 1114112) fail('SOURCE-POLICY');
  }
  const path = mkdtempSync('/tmp/zryna-playground-source-v2-');
  const root = openSync(path, constants.O_RDONLY | constants.O_DIRECTORY | constants.O_NOFOLLOW);
  const rootIdentity = fstatSync(root, { bigint: true });
  const directories = new Map([['', rootIdentity]]);
  const retained = new Map();
  let sealed = false;
  let removed = false;
  const location = relative => relative ? `/proc/self/fd/${root}/${relative}` : `/proc/self/fd/${root}`;
  const directoryState = relative => relative ? state(location(relative)) : fstatSync(root, { bigint: true });
  function verify() {
    const namedRoot = state(path);
    if (removed || !namedRoot.isDirectory() || !sameObject(namedRoot, rootIdentity)) fail('SOURCE-CLEANUP');
    for (const [relative, expected] of directories) {
      const current = directoryState(relative);
      if (!sameObject(current, expected) || !current.isDirectory() || sealed && !sameState(current, expected)) fail('SOURCE-CLEANUP');
      const actual = readdirSync(location(relative)).sort();
      const wanted = [...directories.keys(), ...retained.keys()].filter(item => item &&
        (dirname(item) === '.' ? '' : dirname(item)) === relative).map(item => item.split('/').at(-1)).sort();
      if (JSON.stringify(actual) !== JSON.stringify(wanted)) fail('SOURCE-CLEANUP');
    }
    for (const [relative, original] of retained) {
      const current = state(location(relative));
      if (!current.isFile() || !sameObject(current, fstatSync(original.fd, { bigint: true })) ||
          sealed && !sameState(original.state, current)) fail('SOURCE-CLEANUP');
    }
  }
  function cleanup() {
    if (removed) return;
    verify();
    for (const directory of directories.keys()) chmodSync(location(directory), 0o700);
    for (const [relative, original] of [...retained].reverse()) {
      unlinkSync(location(relative)); closeSync(original.fd); retained.delete(relative);
    }
    for (const relative of [...directories.keys()].filter(Boolean).sort((a, b) => b.length - a.length)) rmdirSync(location(relative));
    if (!sameObject(state(path), rootIdentity)) fail('SOURCE-CLEANUP');
    rmdirSync(path); closeSync(root); removed = true;
  }
  try {
    for (const [relative, bytes] of files) {
      const parts = relative.split('/');
      for (let index = 1; index < parts.length; index++) {
        const directory = parts.slice(0, index).join('/');
        if (!directories.has(directory)) {
          mkdirSync(location(directory), { mode: 0o700 });
          directories.set(directory, directoryState(directory));
        }
      }
      const fd = openSync(location(relative), constants.O_RDWR | constants.O_CREAT | constants.O_EXCL | constants.O_NOFOLLOW, relative === 'resources/compiler' ? 0o500 : 0o400);
      const original = { fd, state: null };
      retained.set(relative, original);
      original.state = fstatSync(fd, { bigint: true }); // Register original authority before fallible writes/readback.
      writeFileSync(fd, bytes); fsyncSync(fd);
      const readback = Buffer.alloc(Math.min(bytes.length, 65536));
      const digest = createHash('sha256');
      let offset = 0;
      while (offset < bytes.length) {
        const count = readSync(fd, readback, 0, Math.min(readback.length, bytes.length - offset), offset);
        if (!count) fail('SOURCE-MATERIAL');
        digest.update(readback.subarray(0, count)); offset += count;
      }
      if (digest.digest('hex') !== sha256(bytes)) fail('SOURCE-MATERIAL');
      const readonly = openSync(location(relative), constants.O_RDONLY | constants.O_NOFOLLOW);
      if (!sameObject(fstatSync(fd, { bigint: true }), fstatSync(readonly, { bigint: true }))) {
        closeSync(readonly); fail('SOURCE-MATERIAL');
      }
      closeSync(fd); original.fd = readonly; original.state = fstatSync(readonly, { bigint: true });
    }
    for (const directory of directories.keys()) chmodSync(location(directory), 0o500);
    for (const relative of directories.keys()) directories.set(relative, directoryState(relative));
    sealed = true; verify();
  } catch (primary) {
    try { cleanup(); } catch (error) { throw new AggregateError([primary, error], 'PLAYGROUND-SOURCE-CLEANUP'); }
    throw primary;
  }
  return Object.freeze({ path, verify, cleanup,
    compiler() {
      const original = retained.get('resources/compiler');
      if (!sealed || removed || !original || executablePin === null) fail('SOURCE-MATERIAL');
      return Object.freeze({ path: join(path, 'resources/compiler'), bytes: executablePin.bytes,
        sha256: executablePin.sha256, ownerUid: Number(original.state.uid) });
    },
    modulePath() { if (!sealed || removed) fail('SOURCE-MATERIAL'); return join(path, 'examples/playground/restricted/daemon/launcher-core.mjs'); } });
}
