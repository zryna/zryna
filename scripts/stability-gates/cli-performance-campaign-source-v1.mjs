// Trusted private checkout bytes bound to the SAME live platform selection, not a copied census.
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { realpathSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import { canonical, portable, readSafe, sha256 } from './input.mjs';
import { git, snapshot } from './source.mjs';
import { CENSUS_LIMITS } from './source-census-v2.mjs';
import { inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { reconcileSourceCensuses } from './source-census-replay.mjs';
import { NUMERIC_WORKFLOWS } from './cli-performance-campaign-plan-v1.mjs';
export const NUMERIC_COLLECTOR_REQUIRED_INPUTS = Object.freeze([...NUMERIC_WORKFLOWS,
  'scripts/stability-gates/run-cli-performance-numeric-hosted.mjs',
  'scripts/stability-gates/linux-operator-owner.py',
  'scripts/stability-gates/windows-job-metrics-v1/calibration/hosted.ps1',
  'scripts/stability-gates/windows-parent-affinity-v1/hosted.ps1',
  'tests/cli-performance-methodology-v4.json', 'tests/cli-performance-prospective-policy-v1.json',
  'package.json', 'pnpm-lock.yaml', 'pnpm-workspace.yaml', 'Cargo.toml', 'Cargo.lock', 'rust-toolchain.toml']);
const sources = new WeakMap();
function source(selection, key) {
  const value = sources.get(key);
  assert(value && value.selection === selection, 'own source key and SAME live campaign selection');
  return value;
}
export function bindNumericCampaignSource(selection, root) {
  assert.equal(arguments.length, 2, 'fixed source owner; no inventory or source-reader override');
  const platform = inspectNumericCampaignPhase(selection);
  assert(typeof root === 'string' && isAbsolute(root)); root = realpathSync(root);
  const actual = snapshot(root); // Existing exact Git/index/blob/64MiB/clean-workspace checks.
  assert.equal(actual.commit, platform.plan.source.commit); assert.equal(actual.tree, platform.plan.source.tree);
  const records = git(root, ['ls-tree', '-r', '-t', '-l', '-z', '--full-tree', actual.tree]).split('\0');
  assert.equal(records.pop(), '');
  const files = new Map(), rows = [], api = 'https://api.github.com/repos/zryna/zryna/git/';
  for (const row of records) {
    const match = /^(040000|100644|100755) (tree|blob) ([a-f0-9]{40})\s+(-|[0-9]+)\t(.+)$/u.exec(row);
    assert(match, 'regular immutable Git source tree');
    const [, mode, type, sha, size, path] = match; portable(path);
    rows.push({ path, mode, type, sha, ...(type === 'blob' ? { size: Number(size) } : {}),
      url: `${api}${type === 'tree' ? 'trees' : 'blobs'}/${sha}` });
    if (type === 'blob') { assert(!files.has(path)); files.set(path, { mode, blob: sha }); }
  }
  assert.equal(files.size, actual.files);
  const gitTree = { sha: actual.tree, url: `${api}trees/${actual.tree}`, tree: rows, truncated: false };
  const key = Object.freeze({}); sources.set(key, { selection, root, actual, files, gitTree }); return key;
}
export function inspectNumericCampaignSource(selection, key) {
  const value = source(selection, key);
  return structuredClone({ format: 'zryna.cli-performance-campaign-source.v1', ...value.actual,
    sourceFileBytesVerified: true, sourceAuthenticated: false, proofQualified: false,
    performanceQualified: false, baselineAdmitted: false, closureStatus: 'blocked' });
}
export function readNumericCampaignOriginal(selection, key, path, maximum) {
  const value = source(selection, key); portable(path);
  assert(Number.isSafeInteger(maximum) && maximum >= 0 && maximum <= CENSUS_LIMITS.fileBytes);
  const expected = value.files.get(path); assert(expected, 'selected immutable source path only');
  const bytes = readSafe(value.root, path, maximum);
  const blob = createHash('sha1').update(`blob ${bytes.length}\0`).update(bytes).digest('hex');
  assert.equal(blob, expected.blob, 'original source bytes still match the selected Git tree');
  return bytes; // Fresh read-owned buffer; no stored backing buffer is exposed.
}
export function numericCampaignCollectorIdentity(selection, key) {
  assert.equal(arguments.length, 2);
  const value = source(selection, key);
  for (const path of NUMERIC_COLLECTOR_REQUIRED_INPUTS) assert(value.files.has(path), `required collector source ${path}`);
  const required = new Set(NUMERIC_COLLECTOR_REQUIRED_INPUTS), digest = createHash('sha256');
  let files = 0, bytes = 0;
  for (const [path, pin] of value.files) {
    if (!required.has(path) && !path.startsWith('scripts/stability-gates/') && !path.startsWith('scripts/m4/')) continue;
    const original = readNumericCampaignOriginal(selection, key, path, CENSUS_LIMITS.fileBytes);
    digest.update(`${pin.mode} ${pin.blob} ${sha256(original)} ${path}\n`); files++; bytes += original.length;
  }
  const identity = { format: 'zryna.cli-performance-collector-source-selection.v1',
    scope: 'all stability-gates and m4 evidence sources plus fixed collection setup inputs',
    files, bytes, bundleSha256: digest.digest('hex') };
  return { ...identity, collectorSha256: sha256(Buffer.from(canonical(identity))) };
}
export function reconcileNumericCampaignCensuses(selection, key, captures, read) {
  assert.equal(arguments.length, 4, 'private immutable tree metadata; no caller tree override');
  const value = source(selection, key);
  const result = reconcileSourceCensuses({ expected: { commit: value.actual.commit, tree: value.actual.tree },
    gitTree: value.gitTree, captures, read });
  assert.equal(result.inventorySha256, value.actual.inventorySha256, 'ALL retained source SHA256 values match independently read Git blob bytes');
  assert.equal(result.files, value.actual.files); assert.equal(result.bytes, value.actual.bytes);
  return { ...result, sourceFileBytesVerified: true, sourceAuthenticated: false, proofQualified: false,
    performanceQualified: false, scope: 'Three retained censuses match independent checkout bytes and the live selected immutable Git tree' };
}
