import assert from 'node:assert/strict';
import test from 'node:test';
import { chmodSync, linkSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, symlinkSync,
  unlinkSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { sha256 } from '../../distribution/canonical.mjs';
import { validateCompilerCrateCache } from './material-cache.mjs';
import { syntheticArchive, writeSyntheticCache } from './material-fixtures.mjs';

function fixture(run) {
  const root = mkdtempSync(join(tmpdir(), 'compiler-material-cache-test-'));
  const cacheRoot = join(root, 'fixture-1.0.0');
  mkdirSync(cacheRoot);
  const files = writeSyntheticCache(cacheRoot);
  const archive = syntheticArchive('fixture-1.0.0', files);
  try { return run({ root, cacheRoot, files, archive, identity: 'fixture-1.0.0' }); }
  finally { rmSync(root, { recursive: true, force: true }); }
}

test('actual cache files match complete synthetic original crate bytes; no source is executed', () => fixture(input => {
  const proof = validateCompilerCrateCache(input, sha256(input.archive));
  assert.equal(proof.sourceFiles, 2);
  assert.equal(proof.checksumPresent, false);
  const checksums = Object.fromEntries(input.files.map(file => [file.path, sha256(file.data)]));
  writeFileSync(join(input.cacheRoot, '.cargo-checksum.json'),
    JSON.stringify({ files: checksums, package: sha256(input.archive) }));
  assert.equal(validateCompilerCrateCache(input, sha256(input.archive)).checksumPresent, true);
}));

for (const [name, mutate] of [
  ['changed source file', x => writeFileSync(join(x.cacheRoot, 'src/lib.rs'), 'changed')],
  ['missing source file', x => unlinkSync(join(x.cacheRoot, 'src/lib.rs'))],
  ['extra source file', x => writeFileSync(join(x.cacheRoot, 'extra.rs'), 'extra')],
  ['extra empty directory', x => mkdirSync(join(x.cacheRoot, 'unlisted-empty'))],
  ['external hardlink alias', x => linkSync(join(x.cacheRoot, 'src/lib.rs'), join(x.root, 'alias.rs'))],
  ['changed cache mode', x => chmodSync(join(x.cacheRoot, 'src/lib.rs'), 0o755)],
  ['source symlink', x => {
    unlinkSync(join(x.cacheRoot, 'src/lib.rs'));
    symlinkSync(join(x.cacheRoot, 'Cargo.toml'), join(x.cacheRoot, 'src/lib.rs'));
  }],
  ['directory symlink', x => {
    renameSync(join(x.cacheRoot, 'src'), join(x.root, 'moved'));
    symlinkSync(join(x.root, 'moved'), join(x.cacheRoot, 'src'));
  }],
  ['altered cargo cache marker', x => writeFileSync(join(x.cacheRoot, '.cargo-ok'), '{"v":2}')],
  ['altered checksum package', x => writeFileSync(join(x.cacheRoot, '.cargo-checksum.json'),
    JSON.stringify({ files: {}, package: 'f'.repeat(64) }))],
  ['altered checksum source map', x => writeFileSync(join(x.cacheRoot, '.cargo-checksum.json'),
    JSON.stringify({ files: { 'Cargo.toml': 'f'.repeat(64) }, package: sha256(x.archive) }))],
  ['extra checksum field', x => writeFileSync(join(x.cacheRoot, '.cargo-checksum.json'),
    JSON.stringify({ files: {}, package: sha256(x.archive), trusted: true }))],
]) {
  test(`original crate/cache proof rejects ${name}`, () => fixture(input => {
    mutate(input);
    assert.throws(() => validateCompilerCrateCache(input, sha256(input.archive)));
  }));
}

test('authenticated crate rejects truncated concatenated gzip special tar and wrong root', () => fixture(input => {
  for (const archive of [input.archive.subarray(0, -1), Buffer.concat([input.archive, input.archive]),
    syntheticArchive(input.identity, input.files, 50), syntheticArchive('other-1.0.0', input.files)]) {
    assert.throws(() => validateCompilerCrateCache({ ...input, archive }, sha256(archive)));
  }
  assert.throws(() => validateCompilerCrateCache(input, 'f'.repeat(64)));
}));

test('empty ordinary source files and missing optional generated marker remain exact ordinary members', () => fixture(input => {
  input.files.push({ path: 'src/empty.rs', data: Buffer.alloc(0), mode: 0o644 });
  writeFileSync(join(input.cacheRoot, 'src/empty.rs'), Buffer.alloc(0), { mode: 0o644 });
  unlinkSync(join(input.cacheRoot, '.cargo-ok'));
  input.archive = syntheticArchive(input.identity, input.files);
  assert.equal(validateCompilerCrateCache(input, sha256(input.archive)).sourceFiles, 3);
  assert.equal(readFileSync(join(input.cacheRoot, 'src/empty.rs')).length, 0);
}));
