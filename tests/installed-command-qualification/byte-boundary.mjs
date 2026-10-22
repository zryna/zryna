import assert from 'node:assert/strict';
import { writeFileSync } from 'node:fs';
import path from 'node:path';
import { readStableFile } from '../../scripts/m4/registry-files.mjs';
import { installedByteCases, installedByteSource, validateInstalledByteDenial } from '../../scripts/m4/installed-byte-boundary.mjs';

export function prepareByteSources(sourceRoot, project) {
  for (const [name, fixture] of Object.entries(installedByteCases)) {
    const bytes = readStableFile(path.join(sourceRoot, 'tests/installed-command-qualification/fixtures/byte-boundary', fixture.file), 256 * 1024);
    installedByteSource(name, bytes, sourceRoot);
    writeFileSync(path.join(project, fixture.file), bytes, { flag: 'wx' });
  }
}
export function observedByteSource(name, sourceRoot, project) {
  return installedByteSource(name, readStableFile(path.join(project, installedByteCases[name].file), 256 * 1024), sourceRoot);
}
export function verifyByteOutcome(row, sourceBefore, sourceAfter, componentBytes) {
  assert.deepEqual(sourceAfter, sourceBefore, 'actual invoked byte source remains unchanged');
  row.source = sourceAfter;
  if (row.name === 'component-bytes-exact') {
    assert.equal(componentBytes.length, 1048576, 'actual installed public factory exact component byte ceiling');
  } else validateInstalledByteDenial(row, JSON.parse(row.stdout));
}
