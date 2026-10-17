import assert from 'node:assert/strict';
import { constants } from 'node:fs';
import { lstat, open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { basename, dirname, isAbsolute, join, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { remaining } from './process.mjs';

const fingerprint = stat => ({ dev: String(stat.dev), ino: String(stat.ino),
  bytes: String(stat.size), links: String(stat.nlink), mode: String(stat.mode),
  modifiedNs: String(stat.mtimeNs), changedNs: String(stat.ctimeNs) });
function regular(stat) {
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.nlink === 1n && stat.ino !== 0n,
    'Single-link ordinary file identity required.');
}
export async function ordinaryRoot(root) {
  assert(isAbsolute(root) && root === resolve(root));
  let current = root;
  for (;;) {
    const stat = await lstat(current);
    assert(stat.isDirectory() && !stat.isSymbolicLink());
    assert.equal(await realpath(current), current);
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
}
async function retainedRead(path, { deadline, maxBytes, expected, admit = regular }, consume) {
  remaining(deadline);
  await ordinaryRoot(dirname(path));
  const before = await lstat(path, { bigint: true });
  admit(before);
  assert(before.size <= BigInt(maxBytes));
  // POSIX rejects final links in open itself. Windows additionally binds regular path and
  // retained-handle identities before and after every complete operation; no link fallback.
  const fd = await open(path, constants.O_RDONLY | (constants.O_NOFOLLOW ?? 0)
    | (process.platform === 'win32' ? 0 : constants.O_NONBLOCK));
  try {
    const held = await fd.stat({ bigint: true });
    admit(held);
    assert.deepEqual(fingerprint(held), fingerprint(before));
    const hash = createHash('sha256');
    const buffer = Buffer.alloc(65536);
    let total = 0;
    for (;;) {
      remaining(deadline);
      const { bytesRead } = await fd.read(buffer, 0, Math.min(buffer.length, maxBytes - total + 1), total);
      remaining(deadline);
      if (!bytesRead) break;
      total += bytesRead;
      assert(total <= maxBytes, 'Material first-extra byte rejected.');
      const bytes = buffer.subarray(0, bytesRead);
      hash.update(bytes);
      await consume(bytes);
    }
    const after = await fd.stat({ bigint: true });
    const pathAfter = await lstat(path, { bigint: true });
    admit(after); admit(pathAfter);
    assert.deepEqual(fingerprint(after), fingerprint(before));
    assert.deepEqual(fingerprint(pathAfter), fingerprint(before));
    assert.equal(BigInt(total), held.size);
    const result = { path, bytes: total, sha256: hash.digest('hex'), identity: fingerprint(held) };
    if (expected) {
      assert.equal(result.bytes, expected.bytes);
      assert.equal(result.sha256, expected.sha256);
    }
    remaining(deadline);
    return result;
  } finally { await fd.close(); }
}
export async function descriptor(path, deadline = performance.now() + 30000, maxBytes = 512 * 1024 * 1024) {
  return retainedRead(path, { deadline, maxBytes }, async () => {});
}
export async function capture(path, destination, expected, deadline) {
  assert(Number.isSafeInteger(expected.bytes) && expected.bytes >= 0);
  assert(/^[a-f0-9]{64}$/.test(expected.sha256));
  await ordinaryRoot(dirname(destination));
  const fd = await open(destination,
    constants.O_WRONLY | constants.O_CREAT | constants.O_EXCL | (constants.O_NOFOLLOW ?? 0), 0o600);
  try {
    const source = await retainedRead(path, { deadline, maxBytes: expected.bytes, expected }, async bytes => {
      let written = 0;
      while (written < bytes.length) {
        remaining(deadline);
        const result = await fd.write(bytes, written, bytes.length - written);
        assert(result.bytesWritten > 0);
        written += result.bytesWritten;
      }
    });
    await fd.sync();
    await fd.chmod(0o400);
    remaining(deadline);
    return { source, captured: { path: destination, bytes: source.bytes, sha256: source.sha256 } };
  } finally { await fd.close(); }
}

// Runtime-module observation only; supplied archives/materials keep regular() unchanged.
function runtimeModuleRegular(stat) {
  assert(stat.isFile() && !stat.isSymbolicLink() && stat.nlink > 0n && stat.ino !== 0n,
    'Ordinary trusted platform runtime module identity required.');
}
export async function createRuntimeDependencyDescriptor(deadline) {
  assert(Number.isFinite(deadline));
  remaining(deadline);
  let system32 = null;
  if (process.platform === 'win32') {
    // Bind only the trusted Node bootstrap environment, never options.env or a material path.
    const root = process.env.SystemRoot;
    assert(typeof root === 'string' && root.length <= 32768 && isAbsolute(root));
    const canonicalRoot = await realpath(root);
    await ordinaryRoot(canonicalRoot);
    system32 = join(canonicalRoot, 'System32');
    await ordinaryRoot(system32);
  }
  remaining(deadline);
  return async path => {
    remaining(deadline);
    const directDll = system32 !== null && typeof path === 'string' && isAbsolute(path)
      && path === resolve(path) && dirname(path) === system32
      && /^[A-Za-z0-9_-][A-Za-z0-9._-]*\.dll$/i.test(basename(path));
    const namedDll = system32 !== null && [
      ['WindowsPowerShell', 'v1.0', 'pwrshsip.dll'],
      ['wbem', 'fastprox.dll'], ['wbem', 'wbemprox.dll'], ['wbem', 'wbemsvc.dll'],
    ].some(parts => path === join(system32, ...parts));
    const platformModule = (directDll || namedDll) && await realpath(path) === path;
    remaining(deadline);
    const result = await retainedRead(path, { deadline, maxBytes: 512 * 1024 * 1024,
      admit: platformModule ? runtimeModuleRegular : regular }, async () => {});
    return { ...result, admission: {
      policy: !platformModule ? 'single-link-ordinary-v1'
        : namedDll ? 'trusted-windows-system32-named-runtime-module-v1'
          : 'trusted-windows-system32-runtime-module-v1',
      links: result.identity.links, publisherAuthenticated: false,
    } };
  };
}

export async function readBytes(path, deadline, maxBytes) {
  const chunks = [];
  await retainedRead(path, { deadline, maxBytes }, async bytes => chunks.push(Buffer.from(bytes)));
  return Buffer.concat(chunks);
}
