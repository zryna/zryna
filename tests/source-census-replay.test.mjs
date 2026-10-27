// Inert metadata/retained-byte fixtures, not authenticated platform or execution evidence.
import assert from 'node:assert/strict';
import test from 'node:test';
import { CENSUS_LIMITS } from '../scripts/stability-gates/source-census-v2.mjs';
import { sha256 } from '../scripts/stability-gates/input.mjs';
import { reconcileSourceCensuses } from '../scripts/stability-gates/source-census-replay.mjs';
const API = 'https://api.github.com/repos/zryna/zryna/git/';
const locations = ['release-build/source-before', 'release-build/source-after', 'source-after-round'];
const encode = value => Buffer.from(`${JSON.stringify(value)}\n`);
function fixture(count = 2) {
  const expected = { commit: 'a'.repeat(40), tree: 'b'.repeat(40) }, files = new Map();
  const rows = Array.from({ length: count }, (_, index) => ({ path: `f${String(index).padStart(5, '0')}.zry`,
    mode: index % 2 ? '100755' : '100644', type: 'blob', sha: 'c'.repeat(40), size: index % 3,
    url: `${API}blobs/${'c'.repeat(40)}` }));
  const gitTree = { sha: expected.tree, url: `${API}trees/${expected.tree}`, tree: rows, truncated: false };
  const lines = rows.map(row => `${row.mode} ${row.sha} ${'d'.repeat(64)} ${row.path}\n`);
  const chunks = []; let chunk = '';
  for (const line of lines) {
    if (Buffer.byteLength(chunk + line) > 131072) { chunks.push(Buffer.from(chunk)); chunk = ''; }
    chunk += line;
  }
  chunks.push(Buffer.from(chunk));
  const manifest = { protocol: 's418-private-source-census-v2', repository: 'https://github.com/zryna/zryna',
    ...expected, files: count, bytes: rows.reduce((sum, row) => sum + row.size, 0),
    inventorySha256: sha256(Buffer.concat(chunks)), limits: { ...CENSUS_LIMITS }, shards: [] };
  const captures = {};
  function retain(parts = chunks) {
    manifest.shards = parts.map((bytes, index) => ({ path: `inventory-${String(index).padStart(4, '0')}.txt`,
      bytes: bytes.length, sha256: sha256(bytes), records: bytes.toString().split('\n').length - 1 }));
    manifest.inventorySha256 = sha256(Buffer.concat(parts));
    for (const [index, directory] of locations.entries()) {
      const raw = encode(manifest);
      files.set(`${directory}/manifest.json`, raw);
      captures[['before', 'after', 'afterRound'][index]] = { ...structuredClone(manifest), manifestSha256: sha256(raw) };
      parts.forEach((bytes, n) => files.set(`${directory}/${manifest.shards[n].path}`, bytes));
    }
  }
  retain();
  return { expected, gitTree, captures, files, manifest, chunks, retain,
    read: (path, maximum) => { const bytes = files.get(path); assert(bytes && bytes.length <= maximum); return bytes; } };
}
function reject(change, count) { const f = fixture(count); change(f); assert.throws(() => reconcileSourceCensuses(f)); }
test('all three complete captures reconcile modes, empty files and repeated content under distinct paths', () => {
  const f = fixture(3), result = reconcileSourceCensuses(f);
  assert.equal(result.files, 3); assert.equal(result.bytes, 3); assert.equal(result.captures.length, 3);
  assert.equal(result.inventorySha256, f.manifest.inventorySha256);
  assert.match(result.scope, /not independently verified/u);
  assert(!Object.hasOwn(result, 'sourceAuthenticated') && !Object.hasOwn(result, 'performanceQualified'));
});
test('all selected identity, truncation, count and byte total bindings are required', () => {
  for (const change of [f => f.expected.commit = 'e'.repeat(40), f => f.expected.tree = 'e'.repeat(40),
    f => f.gitTree.truncated = true, f => f.gitTree.tree[0].size++,
    f => { f.manifest.files++; f.retain(); }, f => { f.manifest.bytes++; f.retain(); },
    f => { f.manifest.limits.shardBytes++; f.retain(); }]) reject(change);
});
test('platform entries reject unsupported modes/types, oversized files and malformed sizes', () => {
  for (const [field, values] of Object.entries({ mode: ['120000', '160000', '0100644'],
    type: ['commit', 'symlink'], size: [-1, '0', 0.5, 2097153], sha: ['z'.repeat(40)] }))
    for (const value of values) reject(f => f.gitTree.tree[0][field] = value);
});
test('missing, extra, duplicate, case colliding and reordered platform blobs reject', () => {
  for (const change of [f => f.gitTree.tree.pop(), f => f.gitTree.tree.push({ ...f.gitTree.tree[0] }),
    f => f.gitTree.tree.push({ ...f.gitTree.tree[0], path: 'f99999.zry' }),
    f => f.gitTree.tree.reverse(), f => f.gitTree.tree[1].path = f.gitTree.tree[0].path.toUpperCase(),
    f => f.gitTree.tree[0].path = '../escape', f => f.gitTree.tree[0].path = 'absent/child.zry']) reject(change);
});
test('platform parent trees reconcile without becoming tracked source files', () => {
  const f = fixture(), tree = { path: 'directory', mode: '040000', type: 'tree', sha: 'e'.repeat(40),
    url: `${API}trees/${'e'.repeat(40)}` };
  const first = { ...f.gitTree.tree[0], path: 'directory/child.zry' };
  f.gitTree.tree = [tree, first, f.gitTree.tree[1]];
  f.retain([Buffer.from(f.chunks[0].toString().replace('f00000.zry', first.path))]);
  assert.equal(reconcileSourceCensuses(f).files, 2);
  reject(g => { for (const row of g.gitTree.tree) row.size = 2097152; }, 33);
});
test('closed schemas and manifest pins reject altered records before replay', () => {
  for (const change of [f => delete f.captures.afterRound, f => f.captures.before.extra = 1,
    f => f.captures.before.manifestSha256 = '0'.repeat(64), f => f.captures.after.bytes++,
    f => f.gitTree.extra = 1, f => f.gitTree.tree[0].extra = 1]) reject(change);
});
test('raw shards require exact UTF8 spelling, pins, complete records and ordered inventory', () => {
  const path = `${locations[0]}/inventory-0000.txt`;
  for (const bytes of [Buffer.from([255]), Buffer.from('truncated'), Buffer.alloc(131073)])
    reject(f => f.files.set(path, bytes));
  for (const mutate of [text => text.replace('100644', '100755'), text => text.replace('c'.repeat(40), 'e'.repeat(40)),
    text => text.replace('d'.repeat(64), 'z'.repeat(64)), text => text.slice(0, -1),
    text => text.split('\n').slice(0, -1).reverse().join('\n') + '\n',
    text => text + text.split('\n')[0] + '\n']) reject(f => f.retain([Buffer.from(mutate(f.chunks[0].toString()))]));
});
test('per-file SHA256 values are retained consistency claims rather than blob-content proofs', () => {
  const f = fixture(); f.retain([Buffer.from(f.chunks[0].toString().replaceAll('d'.repeat(64), 'e'.repeat(64)))]);
  assert.match(reconcileSourceCensuses(f).scope, /not independently verified/u);
});
test('three matching manifests and shard contents are mandatory', () => {
  reject(f => f.files.delete(`${locations[2]}/inventory-0000.txt`));
  reject(f => {
    const value = { ...f.manifest, inventorySha256: '0'.repeat(64) }, raw = encode(value);
    f.files.set(`${locations[2]}/manifest.json`, raw);
    f.captures.afterRound = { ...value, manifestSha256: sha256(raw) };
  });
});
test('multiple shards require exact greedy 128KiB record boundaries even when all hashes are coherent', () => {
  const f = fixture(1200); assert.equal(f.chunks.length, 2); reconcileSourceCensuses(f);
  const lines = Buffer.concat(f.chunks).toString().split('\n'); lines.pop();
  f.retain([Buffer.from(lines.slice(0, 600).join('\n') + '\n'), Buffer.from(lines.slice(600).join('\n') + '\n')]);
  assert.throws(() => reconcileSourceCensuses(f), /greedy/u);
});
test('manifest JSON refuses duplicate fields, invalid UTF8, missing newline and caller buffer changes', () => {
  for (const mutate of [raw => Buffer.concat([Buffer.from([255]), raw]), raw => raw.subarray(0, -1),
    raw => Buffer.from(raw.toString().replace('{', '{"files":2,'))]) reject(f => {
    const path = `${locations[0]}/manifest.json`, raw = mutate(f.files.get(path)); f.files.set(path, raw);
    f.captures.before.manifestSha256 = sha256(raw);
  });
  const f = fixture(), original = f.read;
  f.read = (path, maximum) => { f.expected.commit = 'f'.repeat(40); f.captures.after.bytes++; return original(path, maximum); };
  assert.equal(reconcileSourceCensuses(f).commit, 'a'.repeat(40));
});
