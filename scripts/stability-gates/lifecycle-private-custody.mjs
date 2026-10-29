// Physical custody only; these checks never issue source/runtime approval.
import assert from 'node:assert/strict';
import { chmodSync, lstatSync, mkdirSync, mkdtempSync, readdirSync, renameSync } from 'node:fs';
import { join, posix } from 'node:path';
import { release } from '../../examples/playground/restricted/daemon/cli-parity/plan.mjs';
import { verifierPins } from '../../examples/playground/restricted/daemon/compiler-publication-constants.mjs';
import { exact, sha256 } from './input.mjs';
import { extractVerifiedProductionFiles } from '../distribution-release/extract-verified-files.mjs';

export function validateAuth(auth, base) {
  exact(auth, ['archive', 'envelope', 'bundle', 'trustedRoot', 'verifierPolicy'], 'original auth');
  const pins = { archive: release.archive, envelope: { bytes: 7253, sha256: release.envelopeSha256 },
    bundle: { bytes: 10990, sha256: release.bundleSha256 },
    trustedRoot: { bytes: verifierPins.trustedRootBytes, sha256: verifierPins.trustedRootSha256 } };
  for (const [key, pin] of Object.entries(pins)) {
    assert.deepEqual(auth[key], { sourcePath: posix.join(base, 'originals', key === 'trustedRoot' ? 'trusted-root' : key),
      bytes: pin.bytes, sha256: pin.sha256, ownerUid: 0, mode: 0o600 });
  }
  assert.deepEqual(auth.verifierPolicy, { ...verifierPins, path: posix.join(base, 'run/cosign') });
}

export function protectedWork() {
  assert.equal(process.getuid(), 0);
  for (const path of ['/', '/tmp']) {
    const value = lstatSync(path);
    assert(value.isDirectory() && value.uid === 0 && (!(value.mode & 0o022) || path === '/tmp' && value.mode & 0o1000));
  }
  const work = mkdtempSync('/tmp/zryna-lifecycle-');
  const value = lstatSync(work); assert.equal(value.uid, 0); assert.equal(value.mode & 0o777, 0o700);
  return work;
}

export function relocate(path, base, work) {
  assert.equal(posix.resolve(path), path);
  for (const prefix of ['originals', 'binaries', 'capture']) {
    if (path.startsWith(base + '/' + prefix + '/')) return work + '/input' + path.slice(base.length);
  }
  assert(path.startsWith(base + '/run/'), 'only explicit lifecycle physical source namespaces');
  return work + path.slice((base + '/run').length);
}

export function verifyOriginalBuffers(auth, files, base) {
  validateAuth(auth, base);
  const byPath = new Map(files.map(file => [file.path, file.data]));
  for (const [key, pin] of Object.entries(auth)) {
    const name = key === 'verifierPolicy' ? 'cosign' : key === 'trustedRoot' ? 'trusted-root' : key;
    const data = byPath.get('originals/' + name); assert(Buffer.isBuffer(data));
    assert.equal(data.length, pin.bytes); assert.equal(sha256(data), pin.sha256);
  }
}

export function restoreInputs(auth, files, base, work) {
  verifyOriginalBuffers(auth, files, base);
  extractVerifiedProductionFiles(join(work, 'input'), files);
}

export function exposeObservations(work) {
  // Upload only this ordinary-file directory, never recursive extracted/source trees.
  const output = join(work, 'observations'); mkdirSync(output, { mode: 0o700 });
  for (const entry of readdirSync(work, { withFileTypes: true })) {
    if (!entry.isFile()) continue;
    const target = join(output, entry.name); renameSync(join(work, entry.name), target);
    chmodSync(target, 0o644);
  }
  chmodSync(output, 0o755); chmodSync(work, 0o755);
}
