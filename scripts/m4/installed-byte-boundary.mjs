import assert from 'node:assert/strict';
import path from 'node:path';
import { digest, readStableFile } from './registry-files.mjs';
import { exact } from './evidence-input.mjs';

export const installedByteCases = Object.freeze({
  'component-bytes-exact': Object.freeze({ file: 'exact.zry', bytes: 87161,
    sha256: 'e42343006d7e7dea34eb5a27f9034dbecce8f6d7ba10267443f050021e7d495d' }),
  'component-bytes-first-extra': Object.freeze({ file: 'extra.zry', bytes: 87161,
    sha256: 'd15526b21a384033b77467c959c8fae1769b2a000cf444643ce29271c8b6e8b7' }),
});
export function installedByteSource(name, bytes, sourceRoot) {
  const expected = installedByteCases[name];
  assert(expected, 'fixed installed component byte case');
  const fixture = readStableFile(path.join(sourceRoot, 'tests/installed-command-qualification/fixtures/byte-boundary', expected.file), 256 * 1024);
  assert.equal(fixture.length, expected.bytes, 'frozen authored source byte count');
  assert.equal(digest(fixture), expected.sha256, 'frozen authored source identity');
  assert(bytes.equals(fixture), 'actual installed authored source equals frozen fixture');
  return { path: expected.file, bytes: bytes.length, sha256: digest(bytes) };
}
export function validateInstalledByteSource(item, bytes, sourceRoot) {
  exact(item.source, ['path', 'bytes', 'sha256'], 'actual installed byte source');
  assert.deepEqual(item.source, installedByteSource(item.name, bytes, sourceRoot), 'actual source observation binding');
}
export function validateInstalledByteDenial(item, response) {
  assert.equal(item.name, 'component-bytes-first-extra');
  assert.equal(item.exitCode, 3, 'first extra is actual component factory rejection');
  assert.equal(item.bundleExists, false, 'first extra publishes no bundle');
  exact(response, ['version', 'command', 'ok', 'manifest', 'results', 'diagnostics'], 'actual component byte denial');
  assert.equal(response.version, 1);
  assert.equal(response.command, 'run');
  assert.equal(response.ok, false);
  assert.equal(response.manifest, null, 'factory rejection has no manifest');
  assert.deepEqual(response.results, [], 'factory rejection has no executed results');
  assert.deepEqual(response.diagnostics, [{ code: 'ZRYNA-W4102', severity: 'error', primary: { kind: 'global' },
    message: 'command component exceeds 1048576 bytes', guidance: 'reduce the verified command within its component budget' }]);
}
