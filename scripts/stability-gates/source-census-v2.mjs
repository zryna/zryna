import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { portable } from './input.mjs';
import { digest, pinFile, requireValue } from './release-build-input.mjs';

export const CENSUS_LIMITS = Object.freeze({ files: 50_000, fileBytes: 2 * 1024 * 1024,
  privateRepositoryBytes: 64 * 1024 * 1024, gitOutputBytes: 16 * 1024 * 1024, shardBytes: 128 * 1024 });
function git(root, executable, env, args) {
  const result = spawnSync(executable, ['--no-optional-locks', ...args], { cwd: root, env, shell: false, encoding: 'utf8',
    timeout: 30_000, maxBuffer: CENSUS_LIMITS.gitOutputBytes, windowsHide: true });
  requireValue(!result.error && result.status === 0 && !result.signal, 'read-only Git census');
  return result.stdout;
}
export function parseIndex(text) {
  const entries = text.split('\0');
  requireValue(entries.pop() === '' && entries.length > 0 && entries.length <= CENSUS_LIMITS.files,
    'complete bounded index');
  const seen = new Set();
  return entries.map(entry => {
    const match = /^(100644|100755) ([a-f0-9]{40}) 0\t(.+)$/.exec(entry);
    requireValue(match, 'tracked regular stage-zero file');
    const [, mode, blob, path] = match;
    portable(path);
    requireValue(!seen.has(path.toLowerCase()), 'case collision'); seen.add(path.toLowerCase());
    return { mode, blob, path };
  });
}
export function indexMatchesHead(entries, treeText) {
  const expected = treeText.split('\0');
  requireValue(expected.pop() === '', 'complete tree output');
  const actual = entries.map(({ mode, blob, path }) => `${mode} blob ${blob}\t${path}`);
  requireValue(actual.length === expected.length && actual.every((row, n) => row === expected[n]),
    'index equals HEAD tree');
}
export function* shardRecords(records) {
  let current = ''; let bytes = 0;
  for (const record of records) {
    const size = Buffer.byteLength(record);
    requireValue(size <= CENSUS_LIMITS.shardBytes, 'bounded inventory record');
    if (bytes + size > CENSUS_LIMITS.shardBytes) { yield current; current = ''; bytes = 0; }
    current += record; bytes += size;
  }
  if (current) yield current;
}
export function captureSourceCensus({ root, directory, executable, env }) {
  mkdirSync(directory, { mode: 0o700 });
  const inspect = args => git(root, executable, env, args);
  const commit = inspect(['rev-parse', 'HEAD']).trim();
  const tree = inspect(['rev-parse', 'HEAD^{tree}']).trim();
  requireValue(/^[a-f0-9]{40}$/.test(commit) && /^[a-f0-9]{40}$/.test(tree), 'Git SHA1 source identity');
  requireValue(inspect(['status', '--porcelain=v1', '--untracked-files=all']) === '', 'clean source');
  const indexText = inspect(['ls-files', '--stage', '-z']);
  const entries = parseIndex(indexText);
  indexMatchesHead(entries, inspect(['ls-tree', '-r', '-z', '--full-tree', 'HEAD']));
  const inventory = createHash('sha256'); const shards = []; let total = 0;
  function* records() {
    for (const { mode, blob, path } of entries) {
      const file = pinFile(join(root, path), CENSUS_LIMITS.fileBytes, true);
      total += file.bytes;
      requireValue(total <= CENSUS_LIMITS.privateRepositoryBytes, 'v2 private repository census byte bound');
      requireValue(file.gitBlob === blob, `working file differs from Git blob: ${path}`);
      if (process.platform !== 'win32') requireValue(Boolean(parseInt(file.mode, 8) & 0o111) ===
        (mode === '100755'), 'working executable mode differs from index');
      const row = `${mode} ${blob} ${file.sha256} ${path}\n`;
      inventory.update(row); yield row;
    }
  }
  for (const text of shardRecords(records())) {
    const path = `inventory-${String(shards.length).padStart(4, '0')}.txt`;
    writeFileSync(join(directory, path), text, { flag: 'wx', mode: 0o600 });
    shards.push({ path, bytes: Buffer.byteLength(text), sha256: digest(text), records: text.split('\n').length - 1 });
  }
  requireValue(inspect(['ls-files', '--stage', '-z']) === indexText &&
    inspect(['rev-parse', 'HEAD']).trim() === commit && inspect(['rev-parse', 'HEAD^{tree}']).trim() === tree &&
    inspect(['status', '--porcelain=v1', '--untracked-files=all']) === '', 'stable source and index');
  const manifest = { protocol: 's418-private-source-census-v2', repository: 'https://github.com/zryna/zryna',
    commit, tree, files: entries.length, bytes: total, inventorySha256: inventory.digest('hex'),
    limits: CENSUS_LIMITS, shards };
  writeFileSync(join(directory, 'manifest.json'), `${JSON.stringify(manifest)}\n`, { flag: 'wx', mode: 0o600 });
  return { ...manifest, manifestSha256: digest(`${JSON.stringify(manifest)}\n`) };
}
export function sourceUnchanged(before, after) {
  requireValue(JSON.stringify(before) === JSON.stringify(after), 'source census changed during build');
}
