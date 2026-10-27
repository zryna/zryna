// Pure retained-census/Git-metadata consistency. The caller owns platform/commit provenance.
// Inventory SHA-256 values are retained claims, not independently checked source-file hashes.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { TextDecoder } from 'node:util';
import { exact, MAX_DOCUMENT, portable, sha256 } from './input.mjs';
import { CENSUS_LIMITS } from './source-census-v2.mjs';

const API = 'https://api.github.com/repos/zryna/zryna/git/';
const FIELDS = ['protocol', 'repository', 'commit', 'tree', 'files', 'bytes', 'inventorySha256', 'limits', 'shards'];
const LOCATIONS = { before: 'release-build/source-before', after: 'release-build/source-after',
  afterRound: 'source-after-round' };
const MAX_SHARDS = Math.ceil(CENSUS_LIMITS.files * 354 / CENSUS_LIMITS.shardBytes);
const hex = (value, width) => assert(typeof value === 'string' && new RegExp(`^[a-f0-9]{${width}}$`, 'u').test(value));
const integer = (value, maximum) => assert(Number.isSafeInteger(value) && value >= 0 && value <= maximum);
const ordered = (left, right) => Buffer.compare(Buffer.from(left), Buffer.from(right)) < 0;
function stable(value) {
  if (Array.isArray(value)) return value.map(stable);
  if (value && typeof value === 'object')
    return Object.fromEntries(Object.keys(value).sort().map(key => [key, stable(value[key])]));
  return value;
}
function manifest(bytes) {
  const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes), value = JSON.parse(text);
  // The producer writes compact JSON plus newline; this also rejects duplicate keys.
  assert.equal(`${JSON.stringify(value)}\n`, text, 'complete producer JSON spelling');
  exact(value, FIELDS, 'source census manifest');
  return value;
}
function platformFiles(expected, response) {
  exact(response, ['sha', 'url', 'tree', 'truncated'], 'recursive Git tree response');
  assert.equal(response.sha, expected.tree); assert.equal(response.url, `${API}trees/${expected.tree}`);
  assert.equal(response.truncated, false, 'complete recursive Git tree required');
  assert(Array.isArray(response.tree) && response.tree.length > 0 && response.tree.length <= CENSUS_LIMITS.gitOutputBytes / 100);
  assert(Buffer.byteLength(JSON.stringify(response)) <= CENSUS_LIMITS.gitOutputBytes, 'bounded tree metadata');
  const paths = new Map(), seen = new Set(), files = []; let previous, total = 0;
  for (const row of response.tree) {
    assert(row && ['tree', 'blob'].includes(row.type), 'no submodule or other Git entry');
    exact(row, row.type === 'blob' ? ['path', 'mode', 'type', 'sha', 'size', 'url']
      : ['path', 'mode', 'type', 'sha', 'url'], 'recursive Git entry');
    portable(row.path); hex(row.sha, 40);
    assert(!seen.has(row.path.toLowerCase()), 'unique paths without case collisions'); seen.add(row.path.toLowerCase());
    const key = row.path + (row.type === 'tree' ? '/' : '');
    assert(previous === undefined || ordered(previous, key), 'Git recursive byte order'); previous = key;
    const slash = row.path.lastIndexOf('/');
    if (slash >= 0) assert.equal(paths.get(row.path.slice(0, slash)), 'tree', 'complete parent tree entry');
    paths.set(row.path, row.type);
    assert.equal(row.url, `${API}${row.type === 'tree' ? 'trees' : 'blobs'}/${row.sha}`);
    if (row.type === 'tree') assert.equal(row.mode, '040000');
    else {
      assert(['100644', '100755'].includes(row.mode), 'regular Git blob mode');
      integer(row.size, CENSUS_LIMITS.fileBytes); total += row.size;
      assert(total <= CENSUS_LIMITS.privateRepositoryBytes);
      files.push({ path: row.path, mode: row.mode, blob: row.sha });
      assert(files.length <= CENSUS_LIMITS.files);
    }
  }
  assert(files.length > 0);
  return { files, bytes: total };
}

/** read(path, maximum) returns retained bytes relative to the capture directory.
 * captures are the three captureSourceCensus return records, including manifestSha256.
 * Equal inventories reconcile retained SHA-256 claims only; no source bytes are supplied.
 */
export function reconcileSourceCensuses({ expected, gitTree, captures, read }) {
  exact(expected, ['commit', 'tree'], 'selected source identity'); hex(expected.commit, 40); hex(expected.tree, 40);
  exact(captures, Object.keys(LOCATIONS), 'all three source captures'); assert.equal(typeof read, 'function');
  // Snapshot all callback-visible inputs before reading retained material.
  const identity = { ...expected }, selected = platformFiles(identity, gitTree);
  const records = structuredClone(captures), results = []; let firstManifest;
  const load = (path, maximum) => {
    portable(path); const bytes = read(path, maximum);
    assert(Buffer.isBuffer(bytes) && bytes.length <= maximum, 'bounded retained census bytes');
    const copy = Buffer.alloc(bytes.length); // No exposed shared Buffer-pool backing storage.
    bytes.copy(copy); return copy;
  };
  for (const [name, directory] of Object.entries(LOCATIONS)) {
    const raw = load(`${directory}/manifest.json`, MAX_DOCUMENT), value = manifest(raw), record = records[name];
    exact(record, [...FIELDS, 'manifestSha256'], 'retained capture record');
    hex(record.manifestSha256, 64); assert.equal(sha256(raw), record.manifestSha256, 'exact retained manifest pin');
    const { manifestSha256, ...body } = record;
    assert.deepEqual(stable(body), stable(value), 'capture record equals original manifest');
    if (firstManifest) assert(raw.equals(firstManifest), 'three byte-identical source census manifests');
    else firstManifest = raw;
    assert.equal(value.protocol, 's418-private-source-census-v2');
    assert.equal(value.repository, 'https://github.com/zryna/zryna');
    assert.equal(value.commit, identity.commit); assert.equal(value.tree, identity.tree);
    assert.deepEqual(value.limits, CENSUS_LIMITS, 'unchanged census limits');
    assert.equal(value.files, selected.files.length); assert.equal(value.bytes, selected.bytes);
    hex(value.inventorySha256, 64);
    assert(Array.isArray(value.shards) && value.shards.length > 0 && value.shards.length <= MAX_SHARDS);
    const digest = createHash('sha256'); let count = 0, priorBytes = 0, inventoryBytes = 0;
    for (const [index, shard] of value.shards.entries()) {
      exact(shard, ['path', 'bytes', 'sha256', 'records'], 'inventory shard');
      assert.equal(shard.path, `inventory-${String(index).padStart(4, '0')}.txt`, 'closed shard sequence');
      integer(shard.bytes, CENSUS_LIMITS.shardBytes); assert(shard.bytes > 0); hex(shard.sha256, 64);
      integer(shard.records, CENSUS_LIMITS.files); assert(shard.records > 0);
      const bytes = load(`${directory}/${shard.path}`, CENSUS_LIMITS.shardBytes);
      assert.equal(bytes.length, shard.bytes); assert.equal(sha256(bytes), shard.sha256);
      const text = new TextDecoder('utf-8', { fatal: true }).decode(bytes);
      const lines = text.split('\n'); assert.equal(lines.pop(), '', 'complete inventory record newline');
      assert.equal(lines.length, shard.records);
      if (index > 0) assert(priorBytes + Buffer.byteLength(lines[0] + '\n') > CENSUS_LIMITS.shardBytes,
        'exact greedy 128 KiB record sharding');
      for (const line of lines) {
        const match = /^(100644|100755) ([a-f0-9]{40}) ([a-f0-9]{64}) (.+)$/u.exec(line);
        assert(match, 'closed regular inventory record');
        const [, mode, blob, retainedSha256, path] = match; portable(path);
        assert.deepEqual({ mode, blob, path }, selected.files[count], 'complete ordered platform blob inventory');
        hex(retainedSha256, 64); count += 1;
      }
      digest.update(bytes); inventoryBytes += bytes.length; priorBytes = bytes.length;
    }
    assert.equal(count, selected.files.length); assert.equal(digest.digest('hex'), value.inventorySha256);
    results.push({ name, manifestSha256, shards: value.shards.length, inventoryBytes });
  }
  return { format: 'zryna.source-census-metadata-reconciliation.v1', ...identity,
    files: selected.files.length, bytes: selected.bytes, inventorySha256: records.before.inventorySha256,
    captures: results, scope: 'Retained inventory versus supplied Git blob metadata; source-file SHA-256 claims are not independently verified' };
}
