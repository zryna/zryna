// Offline composition fixture only. Historical provider bytes do not become fresh hosted observations.
import assert from 'node:assert/strict';
import { canonical, MAX_DOCUMENT, sha256, parse, readSafe } from '../scripts/stability-gates/input.mjs';
import { fileURLToPath } from 'node:url';
import { openPerformanceArchive, listPerformanceArchive, readPerformanceArchiveEntry, closePerformanceArchive }
  from '../scripts/stability-gates/cli-performance-archive.mjs';
import { GNU_TIME, PACKAGE, timeExecutable } from '../scripts/stability-gates/cli-performance-linux-provider.mjs';
import { linkNumericCampaignPhase } from '../scripts/stability-gates/cli-performance-campaign-link-v1.mjs';
import { bindNumericCampaignSource } from '../scripts/stability-gates/cli-performance-campaign-source-v1.mjs';
import { reconcileNumericCampaignArchive, closeNumericCampaignArchive } from '../scripts/stability-gates/cli-performance-campaign-archive-v1.mjs';
import { reconcileNumericCampaignHost } from '../scripts/stability-gates/cli-performance-campaign-host-v1.mjs';
import { reconcileNumericCampaignLinuxProvider } from '../scripts/stability-gates/cli-performance-campaign-linux-provider-v1.mjs';
import { campaignArchiveFixture, campaignArchiveSource } from './cli-performance-campaign-archive-fixture.mjs';
import { campaignFixture } from './cli-performance-campaign-fixture.mjs';
import { linuxReplayFixture } from './cli-performance-linux-replay-fixture.mjs';
import { fixture as windowsHostFixture } from './cli-performance-campaign-windows-host-fixture.mjs';
import { addHosts } from './cli-performance-campaign-host-fixture.mjs';
export function retainedLinuxProviderFixture(bytes) {
  assert(Buffer.isBuffer(bytes) && bytes.length >= 22 && bytes.length <= 1024 * 1024);
  const image = Buffer.from(bytes), artifact = { bytes: image.length, sha256: sha256(image) };
  const key = openPerformanceArchive(image, artifact, 'linux-provider');
  try {
    const rows = listPerformanceArchive(key), candidates = rows.filter(row => row.path.endsWith('/runtime-receipt.json'));
    assert.equal(candidates.length, 1); assert(candidates[0].bytes <= 65536);
    const receipt = readPerformanceArchiveEntry(key, candidates[0].path), value = parse(receipt), root = value.providerRoot;
    const prefix = root.slice('/tmp/'.length) + '/';
    assert(rows.every(row => row.path.startsWith(prefix)));
    for (const [name, pin] of [['time.deb', PACKAGE], ['extracted/usr/bin/time', GNU_TIME]]) {
      const raw = readPerformanceArchiveEntry(key, prefix + name);
      assert.equal(raw.length, pin.bytes); assert.equal(sha256(raw), pin.sha256);
    }
    const executable = timeExecutable(root), binding = Buffer.from(canonical({ providerRoot: root,
      providerRecordSha256: value.providerRecordSha256, providerReceiptSha256: sha256(receipt), executable,
      authority: 'binding-only-no-source-or-baseline-authority' }));
    const repository = fileURLToPath(new URL('../', import.meta.url));
    const admission = readSafe(repository, 'scripts/stability-gates/provider-inputs/linux-gnu-time-debian-trixie-v2.json', MAX_DOCUMENT);
    return { bytes: image, root, receipt, binding, executable, admission };
  } finally { closePerformanceArchive(key); }
}
export async function campaignNormalizationFixture(t, providerBytes) {
  const provider = retainedLinuxProviderFixture(providerBytes), origin = campaignFixture();
  const source = campaignArchiveSource(t, origin, linuxReplayFixture().document, true), fixtures = [];
  for (const windows of [false, true]) for (const role of ['baseline-round-1', 'baseline-round-2']) {
    const f = await campaignArchiveFixture(t, { windows, role, sharedSource: source,
      hostEvidence: windows, provider: windows ? null : provider });
    const native = windows ? windowsHostFixture(f) : null;
    if (native) for (const [path, bytes] of native.files) f.files.set(path, bytes);
    addHosts(f, () => {}, native); await f.reseal(); fixtures.push(f);
  }
  const platform = campaignFixture('baselines', source.source);
  for (const f of fixtures) for (const [kind, bytes] of [['primary', f.bytes], ...(!f.host.startsWith('windows') ? [['provider', provider.bytes]] : [])]) {
    const name = kind === 'primary' ? `cli-performance-numeric-${f.host}-${f.role}-${platform.plan.runId}-1`
      : `cli-performance-numeric-linux-provider-${f.role}-${platform.plan.runId}-1`;
    const artifact = platform.artifacts.find(row => row.name === name); assert(artifact);
    artifact.size_in_bytes = bytes.length; artifact.digest = `sha256:${sha256(bytes)}`;
  }
  const selection = await linkNumericCampaignPhase(...platform.arguments());
  const sourceKey = bindNumericCampaignSource(selection, source.root), roundKeys = [];
  const close = () => { for (const row of roundKeys) closeNumericCampaignArchive(selection, sourceKey, row.rawKey); };
  try {
    for (const f of fixtures) {
      const rawKey = reconcileNumericCampaignArchive(selection, sourceKey, f.artifactId, f.bytes);
      const row = { rawKey, hostKey: null, providerKey: null }; roundKeys.push(row);
      row.hostKey = reconcileNumericCampaignHost(selection, sourceKey, rawKey);
      if (f.host === 'linux-x86_64') row.providerKey = reconcileNumericCampaignLinuxProvider(selection, sourceKey, rawKey, provider.bytes);
    }
    return { selection, sourceKey, roundKeys, fixtures, source, platform, close,
      scope: 'Synthetic four-slot records plus historical original provider bytes; no hosted execution/admission' };
  } catch (error) { close(); throw error; }
}
