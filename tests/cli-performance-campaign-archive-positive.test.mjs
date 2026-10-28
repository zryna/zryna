// Full synthetic retained archives plus actual private Git source bytes; no workload or network.
import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { linkNumericCampaignPhase } from '../scripts/stability-gates/cli-performance-campaign-link-v1.mjs';
import { bindNumericCampaignSource } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { reconcileNumericCampaignArchive, inspectNumericCampaignArchive,
  readNumericCampaignArchiveEntry, closeNumericCampaignArchive } from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { campaignArchiveFixture } from './cli-performance-campaign-archive-fixture.mjs';
const PREFIX = 'm418-hosted-observations/capture/';
const issue = f => reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
const inspect = (f, key) => inspectNumericCampaignArchive(f.selection, f.sourceKey, key);
const close = (f, key) => closeNumericCampaignArchive(f.selection, f.sourceKey, key);
function successful(f) {
  const key = issue(f);
  try { return inspect(f, key); } finally { close(f, key); }
}
function rejected(f, expected) {
  let key;
  try { assert.throws(() => { key = issue(f); }, expected); } finally { if (key) close(f, key); }
}
function changeJson(f, path, mutate) {
  const name = PREFIX + path, value = JSON.parse(f.files.get(name)); mutate(value);
  f.files.set(name, Buffer.from(canonical(value)));
}

test('both host conventions and heldout/baseline roles issue after every actual composite replay', async t => {
  for (const windows of [false, true]) for (const phase of ['baselines', 'heldout']) {
    const f = await campaignArchiveFixture(t, { windows, phase }), report = successful(f);
    assert.equal(report.round, phase === 'heldout' ? 'held-out-candidate' : 'baseline-round-1');
    assert.equal(report.host, windows ? 'windows-x86_64' : 'linux-x86_64');
    assert.equal(report.jobId, report.checkRunId); assert.equal(report.run.attempt, 1);
    assert.equal(report.source.commit, f.source.commit); assert.equal(report.source.tree, f.source.tree);
    assert.equal(report.originals.releaseBuild.sha256, sha256(f.files.get(PREFIX + 'release-build/capture.json')));
    assert.deepEqual(report.reconciliations, { rawMetrics: true, releaseImage: true, pairedOraclesAndArchitecture: true, sourceFileBytes: true });
    assert.equal(report.sourceCensuses.captures.length, 3);
    assert.equal(report.cellCount, windows ? 20 : 24); assert.equal(report.attemptCount, windows ? 120 : 144);
    assert.equal(report.measuredCount, windows ? 100 : 120);
    for (const cell of report.cells) {
      assert.equal(cell.attempts.length, 6); assert.equal(cell.attempts[0].role, 'warmup');
      assert(cell.attempts.slice(1).every(row => row.role === 'measured'));
    }
    if (windows) {
      assert.equal(report.cells[0].attempts[1].wallTicks, '9007199254740993');
      assert.equal(report.cells[0].attempts[0].peakJobMemoryUsedBytes, '9007199254741000');
    } else assert.equal(report.cells[0].attempts[0].wallNs, '14000000000');
    for (const flag of ['sourceAuthenticated', 'rawAuthenticated', 'hostAuthenticated', 'proofQualified',
      'performanceQualified', 'baselineAdmitted', 'hostAdmitted']) assert.equal(report[flag], false);
    assert.equal(report.thresholds, null); assert.equal(report.closureStatus, 'blocked');
  }
});
test('retained original entries and projections own copies after caller ZIP and views mutate', async t => {
  for (const windows of [false, true]) {
    const f = await campaignArchiveFixture(t, { windows });
    const name = 'm418-hosted-observations/operator.stdout.raw', original = Buffer.from('synthetic outer collector stream\n');
    f.files.set(name, Buffer.from(original)); await f.reseal();
    const key = issue(f);
    try {
      const read = maximum => readNumericCampaignArchiveEntry(f.selection, f.sourceKey, key, name, maximum);
      assert(read(original.length).equals(original)); read(original.length).fill(0);
      f.bytes.fill(0); f.files.get(name).fill(0);
      assert(read(original.length).equals(original)); assert.throws(() => read(original.length - 1));
      const report = inspect(f, key); report.cells[0].attempts[0].id = 'mutated'; report.tools.buildBefore.node.sha256 = '0'.repeat(64);
      assert.notEqual(inspect(f, key).cells[0].attempts[0].id, 'mutated');
      assert.notEqual(inspect(f, key).tools.buildBefore.node.sha256, '0'.repeat(64));
      for (const path of ['../escape', '/absolute', 'm418-hosted-observations/missing.raw']) {
        assert.throws(() => readNumericCampaignArchiveEntry(f.selection, f.sourceKey, key, path, 262144));
      }
      assert.throws(() => readNumericCampaignArchiveEntry(f.selection, f.sourceKey, key, name, 64 * 1024 * 1024 + 1));
    } finally { close(f, key); }
    assert.throws(() => inspect(f, key));
    assert.throws(() => readNumericCampaignArchiveEntry(f.selection, f.sourceKey, key, name, original.length));
    assert.throws(() => close(f, key));
  }
});
test('genuine keys remain bound to the same selection and source objects, not equal copied reports', async t => {
  const f = await campaignArchiveFixture(t), key = issue(f);
  try {
    const otherSelection = await linkNumericCampaignPhase(...f.platform.arguments());
    const otherSource = bindNumericCampaignSource(otherSelection, f.root);
    const sameSelectionOtherSource = bindNumericCampaignSource(f.selection, f.root);
    for (const [selection, source] of [[otherSelection, otherSource], [f.selection, sameSelectionOtherSource]]) {
      assert.throws(() => inspectNumericCampaignArchive(selection, source, key));
      assert.throws(() => readNumericCampaignArchiveEntry(selection, source, key, PREFIX + 'operator-input.json', 262144));
      assert.throws(() => closeNumericCampaignArchive(selection, source, key));
    }
    assert.throws(() => reconcileNumericCampaignArchive(otherSelection, f.sourceKey, f.artifactId, f.bytes));
    for (const copied of [{ ...key }, inspect(f, key), structuredClone(inspect(f, key))]) {
      assert.throws(() => inspectNumericCampaignArchive(f.selection, f.sourceKey, copied));
    }
    assert.equal(inspect(f, key).source.commit, f.source.commit);
  } finally { close(f, key); }
});
test('provider IDs and corrupted or truncated original ZIP bytes reject without poisoning pristine issuance', async t => {
  const f = await campaignArchiveFixture(t), provider = f.platform.artifacts.find(a => a.name.includes('linux-provider'));
  assert.throws(() => reconcileNumericCampaignArchive(f.selection, f.sourceKey, provider.id, f.bytes));
  const changed = Buffer.from(f.bytes); changed[30] ^= 1;
  assert.throws(() => reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, changed));
  assert.throws(() => reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes.subarray(0, -1)));
  assert.equal(successful(f).round, 'baseline-round-1');
});
test('platform-rebound malformed build/oracle/third-census gates fail and pristine retries still issue', async t => {
  for (const windows of [false, true]) {
    const f = await campaignArchiveFixture(t, { windows });
    for (const [path, mutate] of [
      ['release-build/capture.json', value => { value.profile.observed = 'debug'; }],
      ['oracles/00-0.after.json', value => { value.status = 'failed'; }],
      ['source-after-round.json', value => { value.files++; }],
      ['round/capture.json', value => { value.round = 'baseline-round-2'; }],
    ]) {
      const original = f.files.get(PREFIX + path); changeJson(f, path, mutate); await f.reseal();
      rejected(f);
      f.files.set(PREFIX + path, original); await f.reseal(); assert.equal(successful(f).round, 'baseline-round-1');
    }
  }
});
test('all composite source material reads use the independent actual Git source, with restoration retry', async t => {
  const f = await campaignArchiveFixture(t), material = 'examples/universal/add.zry';
  const path = join(f.root, material), original = readFileSync(path);
  try {
    assert(original.length > 0);
    const changed = Buffer.from(original); changed[0] ^= 1;
    writeFileSync(path, changed); // Same length reaches the Git-blob check independently of the byte cap.
    rejected(f, /selected Git tree/u);
  } finally { writeFileSync(path, original); }
  await f.reseal(); assert.equal(successful(f).reconciliations.sourceFileBytes, true);
});

test('platform-rebound raw cleanup failures reject despite otherwise consistent helper and stream copies', async t => {
  for (const windows of [false, true]) {
    const f = await campaignArchiveFixture(t, { windows }), id = f.session.cells[0].attempts[0].id;
    const paths = windows ? [`round/${id}.metadata.raw`, `round/${id}.sample/result.json`, `round/${id}.sample-read.json`]
      : [`round/${id}.json`];
    const originals = paths.map(path => [PREFIX + path, f.files.get(PREFIX + path)]);
    if (windows) {
      const h = JSON.parse(originals[0][1]); h.forcedTermination = true;
      const bytes = Buffer.from(JSON.stringify(h) + '\n');
      f.files.set(originals[0][0], bytes); f.files.set(originals[1][0], bytes);
      changeJson(f, paths[2], value => { value.metadataBytes = bytes.length; });
    } else changeJson(f, paths[0], value => { value.error = 'CLEANUP'; });
    await f.reseal(); rejected(f);
    for (const [path, bytes] of originals) f.files.set(path, bytes);
    await f.reseal(); assert.equal(successful(f).reconciliations.rawMetrics, true);
  }
});
