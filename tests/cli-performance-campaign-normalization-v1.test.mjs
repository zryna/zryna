// Pure projection positives and real owner-key rejection; no package, native or workload execution.
import assert from 'node:assert/strict';
import test from 'node:test';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { checkedRound } from '../scripts/stability-gates/cli-performance-numeric-round-v1.mjs';
import { fixture as declarations, catalogue } from './cli-performance-numeric-fixture.mjs';
import { numericSemanticTools, projectNumericRound }
  from '../scripts/stability-gates/cli-performance-campaign-normalization-projection-v1.mjs';
import { normalizeNumericCampaignBaselines } from '../scripts/stability-gates/cli-performance-campaign-normalization-v1.mjs';
import { reconcileNumericCampaignArchive, closeNumericCampaignArchive }
  from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { campaignArchiveFixture } from './cli-performance-campaign-archive-fixture.mjs';
const hash = label => sha256(Buffer.from(label));
const pin = path => ({ path, bytes: 128, sha256: hash(path.split(/[\\/]/u).pop()) });
function tools(windows = false) {
  const host = windows ? 'windows-x86_64' : 'linux-x86_64';
  const build = Object.fromEntries(['cargo', 'rustc', 'cc', 'linker', 'node', 'git'].map(role => [role, pin(`/tools/${role}`)]));
  const raw = { host, tools: { buildBefore: build, buildAfter: structuredClone(build),
    sdk: { before: windows ? [pin('/sdk/a.h'), pin('/sdk/b.h')] : null,
      after: windows ? [pin('/sdk/a.h'), pin('/sdk/b.h')] : null },
    measured: { node: pin('/tools/node'), ...(windows ? { helper: pin('/evidence/job-helper') } : { time: pin('/provider/time') }) } } };
  const facts = { host, nativeParentBindingVerified: windows, toolInputs: windows ? { helper: pin('/evidence/affinity-helper') } : null };
  const provider = windows ? null : { providerOriginalsVerified: true, executable: pin('/provider/time'), package: pin('/provider/time.deb'),
    providerRecordSha256: hash('provider'), observedRuntime: { host: { osRelease: pin('/usr/lib/os-release') },
      acquisition: { curl: pin('/usr/bin/curl'), version: 'observed curl\n' }, tool: { version: 'observed GNU time\n' },
      runtime: { ...Object.fromEntries(['dpkgDeb', 'dpkgQuery', 'loader', 'libc'].map(role => [role, pin(`/usr/lib/${role}`)])),
        sharedProviders: [pin('/usr/lib/loader'), pin('/usr/lib/libc')], versions: { loader: 'observed loader\n' } } } };
  const calibration = windows ? { selectedDirectHeaders: Array.from({ length: 12 }, (_, i) => pin(`/sdk/header-${i}.h`)),
    selectedStaticLibraries: Array.from({ length: 5 }, (_, i) => pin(`/sdk/library-${i}.lib`)),
    compilerFileVersion: 'observed compiler', linkerFileVersion: 'observed linker', sdkVersion: 'observed SDK', vcToolsVersion: 'observed VC' } : null;
  return { raw, host: facts, provider, calibration };
}
test('semantic host-keyed tools preserve observed hashes and versions while excluding volatile paths and file identity', () => {
  for (const windows of [false, true]) {
    const f = tools(windows), result = numericSemanticTools(f), changed = structuredClone(f);
    for (const group of [changed.raw.tools.buildBefore, changed.raw.tools.buildAfter]) for (const row of Object.values(group)) {
      row.path = '/fresh-checkout/another-name'; row.identity = { inode: 'new' };
    }
    if (windows) { changed.host.toolInputs.helper.path = '/fresh-parent/helper'; changed.raw.tools.measured.helper.path = '/fresh-job/helper'; }
    else { changed.provider.executable.path = '/fresh-provider/time'; changed.raw.tools.measured.time.path = '/fresh-provider/time'; }
    assert.deepEqual(numericSemanticTools(changed), result);
    assert.equal(result.host, f.raw.host); assert(result.native.length > 0);
    const digest = sha256(canonical(result));
    changed.raw.tools.buildAfter.git.sha256 = hash('different git');
    assert.throws(() => numericSemanticTools(changed));
    changed.raw.tools.buildBefore.git.sha256 = hash('different git');
    assert.notEqual(sha256(canonical(numericSemanticTools(changed))), digest);
  }
});
test('SDK dependency bytes and observed host providers remain part of semantic tool selection', () => {
  for (const windows of [false, true]) {
    const f = tools(windows), original = numericSemanticTools(f);
    const row = windows ? f.calibration.selectedDirectHeaders[0] : f.provider.observedRuntime.runtime.libc;
    row.sha256 = hash('different dependency');
    assert.notDeepEqual(numericSemanticTools(f), original);
  }
  const f = tools(true); f.calibration.selectedDirectHeaders.pop(); assert.throws(() => numericSemanticTools(f));
  const g = tools(false); g.provider = null; assert.throws(() => numericSemanticTools(g));
  const drift = tools(true); drift.raw.tools.sdk.after[0].sha256 = hash('changed SDK epoch');
  assert.throws(() => numericSemanticTools(drift), /stable original SDK/u);
  const flat = tools(false); flat.raw.tools.sdk = null; assert.throws(() => numericSemanticTools(flat));
});
test('pure declaration projection preserves all native samples and each original build-record digest', () => {
  const f = declarations();
  for (const round of f.rounds) {
    const hostPolicy = f.profile.hosts.find(host => host.host === round.host);
    const raw = { host: round.host, round: round.role, jobId: Number(round.identity.jobId),
      artifact: { id: Number(round.identity.artifactId), sha256: round.identity.rawSha256 }, image: { sha256: round.build.cliSha256 },
      originals: { releaseBuild: { sha256: round.build.recordSha256 } }, catalogueSha256: round.catalogueSha256,
      startedAt: round.startedAt.replace('.000Z', 'Z'), completedAt: round.endedAt.replace('.000Z', 'Z'), cells: structuredClone(round.cells) };
    const input = { raw, host: { classId: round.classId, facts: round.classBefore }, source: round.source,
      collectorSha256: round.collectorSha256, toolchainSha256: round.toolchainSha256 };
    const result = projectNumericRound(input);
    assert.deepEqual(checkedRound(result, f.profile, catalogue, hostPolicy, round.role).round, round);
    assert.equal(result.cells.length * 6, round.host === 'linux-x86_64' ? 144 : 120);
    assert.equal(result.sourceAuthenticated, false); assert.equal(result.performanceQualified, false);
    assert.equal(result.thresholdAuthority, null); assert.equal(result.closureStatus, 'blocked');
    raw.originals.releaseBuild.sha256 = hash('independent fresh original build record');
    const changed = projectNumericRound(input);
    assert.notEqual(changed.build.recordSha256, result.build.recordSha256);
    assert.equal(changed.build.cliSha256, result.build.cliSha256);
    assert.notEqual(sha256(canonical(changed)), sha256(canonical(result)));
  }
});
test('production composition rejects copied phase keys, omitted slots and real raw keys paired with copied host keys', async t => {
  assert.equal(normalizeNumericCampaignBaselines.length, 3);
  assert.throws(() => normalizeNumericCampaignBaselines({}, {}, [], () => {}), /fixed opaque/u);
  assert.throws(() => normalizeNumericCampaignBaselines({}, {}, []));
  const f = await campaignArchiveFixture(t);
  assert.throws(() => normalizeNumericCampaignBaselines(f.selection, f.sourceKey, []), /all four/u);
  const rawKey = reconcileNumericCampaignArchive(f.selection, f.sourceKey, f.artifactId, f.bytes);
  try {
    const keys = Array.from({ length: 4 }, () => ({ rawKey, hostKey: { hostFactBytesVerified: true }, providerKey: null }));
    assert.throws(() => normalizeNumericCampaignBaselines(f.selection, f.sourceKey, keys), /own host key/u);
    keys[0].rawKey = { rawMetrics: true };
    assert.throws(() => normalizeNumericCampaignBaselines(f.selection, f.sourceKey, keys), /own archive key/u);
  } finally { closeNumericCampaignArchive(f.selection, f.sourceKey, rawKey); }
});
