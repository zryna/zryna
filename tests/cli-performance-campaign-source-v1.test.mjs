// Real tiny Git data only; no compiler, workload, archived command or candidate application runs.
import assert from 'node:assert/strict';
import { execFileSync } from 'node:child_process';
import { mkdtempSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';
import { sha256 } from '../scripts/stability-gates/input.mjs';
import { CENSUS_LIMITS } from '../scripts/stability-gates/source-census-v2.mjs';
import { NUMERIC_WORKFLOWS } from '../scripts/stability-gates/cli-performance-campaign-plan-v1.mjs';
import { linkNumericCampaignPhase } from '../scripts/stability-gates/cli-performance-campaign-link-v1.mjs';
import { bindNumericCampaignSource, inspectNumericCampaignSource, readNumericCampaignOriginal,
  reconcileNumericCampaignCensuses, NUMERIC_COLLECTOR_REQUIRED_INPUTS,
  numericCampaignCollectorIdentity } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { campaignFixture } from './cli-performance-campaign-fixture.mjs';
const API = 'https://api.github.com/repos/zryna/zryna/git/';
async function fixture(t, collector = false) {
  const root = mkdtempSync(join(tmpdir(), 'numeric-source-')); t.after(() => rmSync(root, { recursive: true }));
  const git = args => execFileSync('git', ['-c', 'core.hooksPath=/dev/null', '-c', 'commit.gpgSign=false', ...args],
    { cwd: root, encoding: 'utf8', stdio: ['ignore', 'pipe', 'pipe'] });
  git(['-c', 'init.templateDir=', 'init', '-q']);
  const origin = campaignFixture();
  for (const path of NUMERIC_WORKFLOWS) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), Buffer.from(origin.responses.get(`/repos/zryna/zryna/contents/${path}?ref=${origin.plan.source.commit}`).content, 'base64'));
  }
  if (collector) for (const path of NUMERIC_COLLECTOR_REQUIRED_INPUTS.filter(path => !NUMERIC_WORKFLOWS.includes(path))) {
    mkdirSync(dirname(join(root, path)), { recursive: true });
    writeFileSync(join(root, path), readFileSync(join(fileURLToPath(new URL('../', import.meta.url)), path)));
  }
  writeFileSync(join(root, 'program.txt'), 'source-only fixture\n');
  git(['add', '.']); git(['-c', 'user.name=Fixture', '-c', 'user.email=fixture@invalid', 'commit', '-qm', 'Source fixture']);
  const source = { commit: git(['rev-parse', 'HEAD']).trim(), tree: git(['rev-parse', 'HEAD^{tree}']).trim(), ref: origin.plan.source.ref };
  const f = campaignFixture('baselines', source), selection = await linkNumericCampaignPhase(...f.arguments());
  const rows = git(['ls-tree', '-r', '-t', '-l', source.tree]).trimEnd().split('\n').map(line => {
    const [meta, path] = line.split('\t'), [mode, type, sha, size] = meta.trim().split(/\s+/u);
    return { path, mode, type, sha, ...(type === 'blob' ? { size: Number(size) } : {}), url: `${API}${type === 'tree' ? 'trees' : 'blobs'}/${sha}` };
  });
  const gitTree = { sha: source.tree, url: `${API}trees/${source.tree}`, tree: rows, truncated: false };
  function retained(corrupt = false) {
    const files = rows.filter(row => row.type === 'blob');
    const inventory = Buffer.from(files.map(row => `${row.mode} ${row.sha} ${corrupt ? 'f'.repeat(64) : sha256(readFileSync(join(root, row.path)))} ${row.path}\n`).join(''));
    const body = { protocol: 's418-private-source-census-v2', repository: 'https://github.com/zryna/zryna',
      commit: source.commit, tree: source.tree, files: files.length, bytes: files.reduce((n, row) => n + row.size, 0),
      inventorySha256: sha256(inventory), limits: { ...CENSUS_LIMITS },
      shards: [{ path: 'inventory-0000.txt', bytes: inventory.length, sha256: sha256(inventory), records: files.length }] };
    const raw = Buffer.from(`${JSON.stringify(body)}\n`), data = new Map(), captures = {};
    for (const [phase, directory] of [['before', 'release-build/source-before'], ['after', 'release-build/source-after'], ['afterRound', 'source-after-round']]) {
      captures[phase] = { ...structuredClone(body), manifestSha256: sha256(raw) };
      data.set(`${directory}/manifest.json`, raw); data.set(`${directory}/inventory-0000.txt`, inventory);
    }
    return { captures, read: (path, maximum) => { const bytes = data.get(path); assert(bytes && bytes.length <= maximum); return bytes; } };
  }
  return { root, f, selection, gitTree, retained };
}
test('independent Git/index/blob bytes reconcile all three full source censuses without performance authority', async t => {
  const f = await fixture(t), key = bindNumericCampaignSource(f.selection, f.root);
  const report = inspectNumericCampaignSource(f.selection, key), retained = f.retained();
  assert.equal(report.files, 5); assert.equal(report.sourceFileBytesVerified, true);
  assert.equal(report.sourceAuthenticated, false); assert.equal(report.performanceQualified, false);
  const result = reconcileNumericCampaignCensuses(f.selection, key, retained.captures, retained.read);
  assert.equal(result.inventorySha256, report.inventorySha256); assert.equal(result.captures.length, 3);
  const bytes = readNumericCampaignOriginal(f.selection, key, 'program.txt', 100); bytes.fill(0);
  assert.equal(readNumericCampaignOriginal(f.selection, key, 'program.txt', 100).toString(), 'source-only fixture\n');
});
test('three self-consistent forged source SHA256 claims cannot replace independently read original bytes', async t => {
  const f = await fixture(t), key = bindNumericCampaignSource(f.selection, f.root), forged = f.retained(true);
  assert.throws(() => reconcileNumericCampaignCensuses(f.selection, key, forged.captures, forged.read), /independently read/u);
});
test('collector identity binds a fixed source bundle without volatile checkout or build-record paths', async t => {
  const missing = await fixture(t), absent = bindNumericCampaignSource(missing.selection, missing.root);
  assert.throws(() => numericCampaignCollectorIdentity(missing.selection, absent), /required collector source/u);
  const f = await fixture(t, true), key = bindNumericCampaignSource(f.selection, f.root);
  const identity = numericCampaignCollectorIdentity(f.selection, key);
  assert.equal(identity.files, NUMERIC_COLLECTOR_REQUIRED_INPUTS.length);
  assert(identity.bytes > 0); assert.match(identity.collectorSha256, /^[a-f0-9]{64}$/u);
  assert.deepEqual(numericCampaignCollectorIdentity(f.selection, key), identity);
  assert.throws(() => { NUMERIC_COLLECTOR_REQUIRED_INPUTS.push('override'); });
});
test('copied keys, another phase selection, changed original bytes and untracked checkout drift reject', async t => {
  const f = await fixture(t), key = bindNumericCampaignSource(f.selection, f.root);
  for (const copied of [{}, { ...key }, inspectNumericCampaignSource(f.selection, key)])
    assert.throws(() => inspectNumericCampaignSource(f.selection, copied));
  const other = await linkNumericCampaignPhase(...f.f.arguments());
  assert.throws(() => inspectNumericCampaignSource(other, key));
  assert.throws(() => readNumericCampaignOriginal(f.selection, key, '../escape', 100));
  assert.throws(() => readNumericCampaignOriginal(f.selection, key, 'program.txt', 1));
  writeFileSync(join(f.root, 'program.txt'), 'changed\n');
  assert.throws(() => readNumericCampaignOriginal(f.selection, key, 'program.txt', 100), /selected Git tree/u);
  assert.throws(() => bindNumericCampaignSource(f.selection, f.root), /clean source/u);
  writeFileSync(join(f.root, 'program.txt'), 'source-only fixture\n');
  writeFileSync(join(f.root, 'untracked.txt'), 'untracked fixture\n');
  assert.throws(() => bindNumericCampaignSource(f.selection, f.root), /clean source/u);
  assert.throws(() => readNumericCampaignOriginal(f.selection, key, 'untracked.txt', 100), /selected immutable source path/u);
});
