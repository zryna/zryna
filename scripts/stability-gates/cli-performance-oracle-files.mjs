// Only verified owned bundles are removed. Both replay copies remain outside the checkout.
import assert from 'node:assert/strict';
import { closeSync, fsyncSync, lstatSync, mkdirSync, openSync, rmSync, writeFileSync } from 'node:fs';
import { dirname, join, posix, resolve } from 'node:path';
import { MAX_DOCUMENT, readSafe, sha256 } from './input.mjs';
import { checkKnownGraphs, checkSelectedBytes } from './cli-performance-methodology-v4.mjs';
import { inventory, sameFile, stageOriginalWindowsM3 } from './cli-performance-windows-files.mjs';
import { pinFile } from './release-build-input.mjs';
import { validateRetainedManifest } from './cli-performance-oracle-manifest.mjs';
export const MAX_ARTIFACTS = 64 * 1024 * 1024;
const identity = path => {
  const stat = lstatSync(path);
  return Object.fromEntries(['dev', 'ino', 'size', 'mode', 'nlink', 'mtimeMs', 'ctimeMs'].map(k => [k, stat[k]]));
};
export function writePrivate(path, bytes) {
  const fd = openSync(path, 'wx', 0o600);
  try { writeFileSync(fd, bytes); fsyncSync(fd); } finally { closeSync(fd); }
}
export function oracleFiles({ root, output, document, tools, toolPins }) {
  const toolIdentities = Object.fromEntries(Object.entries(tools).map(([key, path]) => [key, identity(path)]));
  let finish;
  const bundle = row => posix.dirname(row.expectedResponse.manifest);
  const copied = (id, path) => `artifacts/${id}/${path}`;
  const checkMaterials = staged => {
    for (const [key, path] of Object.entries(tools)) {
      assert.deepEqual(pinFile(path, 256 * 1024 * 1024), toolPins[key], 'stable oracle tool bytes/mode');
      assert.deepEqual(identity(path), toolIdentities[key], 'stable oracle tool identity');
    }
    const reader = (path, maximum) => readSafe(root, path, maximum);
    checkSelectedBytes(document, reader); checkKnownGraphs(document, reader);
    if (staged) for (const family of document.families) if (family.staging)
      for (const copy of family.staging.copies) sameFile(root,
        { ...family.sources.find(pin => pin.path === copy.from), path: copy.to }, MAX_DOCUMENT);
  };
  const checkArtifacts = (row, id, files) => {
    assert.deepEqual(inventory(root, bundle(row)), files.map(pin => pin.path), 'complete stable source artifact inventory');
    for (const pin of files) {
      sameFile(root, pin, MAX_ARTIFACTS);
      sameFile(output, { ...pin, path: copied(id, pin.path) }, MAX_ARTIFACTS);
    }
  };
  return { checkMaterials,
    stage: () => { finish = stageOriginalWindowsM3(root, document); },
    finishStage: () => { assert(finish); finish(); },
    beforeAttempt: row => assert(!lstatSync(resolve(root, bundle(row)), { throwIfNoEntry: false }),
      'owned output already exists; never overwrite foreign artifacts'),
    snapshotArtifacts: (row, id) => {
      const paths = inventory(root, bundle(row));
      assert(paths.length > 0 && paths.length <= 64, 'bounded complete artifact file set');
      let copiedBytes = 0;
      const files = paths.map(path => {
        const bytes = readSafe(root, path, MAX_ARTIFACTS);
        copiedBytes += bytes.length;
        assert(copiedBytes <= MAX_ARTIFACTS, 'bounded complete artifact copy bytes');
        const pin = { path, bytes: bytes.length, sha256: sha256(bytes) };
        const destination = join(output, copied(id, path));
        mkdirSync(dirname(destination), { recursive: true, mode: 0o700 });
        writePrivate(destination, bytes); sameFile(output, { ...pin, path: copied(id, path) }, MAX_ARTIFACTS);
        sameFile(root, pin, MAX_ARTIFACTS);
        return pin;
      });
      checkArtifacts(row, id, files);
      // Validate the retained copy only after every raw artifact is copied and hash-verified.
      validateRetainedManifest({ document, row, host: row.host, files,
        manifestBytes: readSafe(output, copied(id, row.expectedResponse.manifest), MAX_DOCUMENT),
        readOriginal: (path, maximum) => readSafe(root, path, maximum) });
      return files;
    },
    checkArtifacts,
    cleanup: (row, id, files) => {
      checkArtifacts(row, id, files);
      rmSync(resolve(root, bundle(row)), { recursive: true, force: false });
    },
  };
}
