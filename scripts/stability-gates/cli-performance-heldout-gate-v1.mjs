// Fixed live held-out entry only. No callback, copied anchor or caller-supplied authority enters this owner.
import assert from 'node:assert/strict';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { canonical, MAX_DOCUMENT, parse, readSafe, sha256 } from './input.mjs';
import { pinFile } from './release-build-input.mjs';
import { captureHostedMetadata } from './cli-performance-host.mjs';
import { captureNumericCompanion, inspectNumericCompanionBefore } from './cli-performance-numeric-host.mjs';
import { verifyNumericCampaignAnchor, inspectNumericCampaignAnchor, closeNumericCampaignAnchor }
  from './cli-performance-campaign-anchor-v1.mjs';
import { numericSemanticTools } from './cli-performance-campaign-normalization-projection-v1.mjs';
import { openLinuxTimeProvider } from './cli-performance-linux-provider.mjs';
import { producerDocument } from './cli-performance-campaign-windows-host-v1.mjs';
import { compareHeldoutHost, compareHeldoutTools, HELDOUT_FLAGS } from './cli-performance-heldout-comparison-v1.mjs';
const gates = new WeakMap(), begun = new WeakSet();
const pin = value => ({ path: value.path, bytes: value.bytes, sha256: value.sha256 });
function save(output, name, value) {
  writeFileSync(join(output, name), canonical(value), { flag: 'wx', mode: 0o600 });
}
function own(key, beforeKey, input, root, output) {
  const value = gates.get(key);
  assert(value && !value.used && value.pid === process.pid && value.beforeKey === beforeKey &&
    value.root === root && value.output === output, 'same live unused held-out gate');
  assert.equal(value.inputSha256, sha256(canonical(input)));
  return value;
}
export async function beginNumericHeldoutGate(beforeKey, input, root, output) {
  assert.equal(arguments.length, 4, 'fixed held-out gate begin');
  assert.equal(input.round, 'held-out-candidate');
  const original = inspectNumericCompanionBefore(beforeKey, input, root, output);
  assert(!begun.has(beforeKey), 'one held-out gate per original before companion'); begun.add(beforeKey);
  const run = process.env.M418_ANCHOR_RUN_ID;
  assert(typeof run === 'string' && /^[1-9][0-9]*$/u.test(run) && Number.isSafeInteger(Number(run)), 'actual fixed anchor run ID');
  const platform = process.platform === 'win32' ? 'windows-x86_64' : 'linux-x86_64';
  let anchorKey;
  try {
    anchorKey = await verifyNumericCampaignAnchor({ root, anchorRunId: Number(run), platform });
    const anchor = inspectNumericCampaignAnchor(anchorKey);
    const current = inspectNumericCompanionBefore(beforeKey, input, root, output);
    assert.deepEqual(current, original, 'original before companion unchanged across anchor verification');
    const source = { commit: input.expectedHead, tree: input.expectedTree };
    const host = compareHeldoutHost(anchor, current, source);
    save(output, 'numeric-gate-before.json', { format: 'zryna.cli-performance-heldout-premeasurement.v1',
      phase: 'before-release-build', host, source, anchor: anchor.anchor,
      profileSha256: sha256(canonical(anchor.profile)), observedClassMatched: true, ...HELDOUT_FLAGS });
    const key = Object.freeze({}); gates.set(key, { pid: process.pid, beforeKey, root, output,
      inputSha256: sha256(canonical(input)), anchorKey, original: current, used: false }); return key;
  } catch (error) { if (anchorKey) closeNumericCampaignAnchor(anchorKey); throw error; }
}
function liveTools(observation, input, build, capture, provider) {
  for (const role of ['cargo', 'rustc', 'cc', 'linker', 'node', 'git'])
    assert.deepEqual(pin(pinFile(input.tools[role])), pin(build.tools.after[role]), 'actual release tool still matches build');
  for (const value of build.sdk.after ?? [])
    assert.deepEqual(pin(pinFile(value.path)), pin(value), 'actual release SDK still matches build');
  assert.deepEqual(pin(pinFile(capture.tools.cli.path)), capture.tools.cli, 'actual measured CLI copy unchanged');
  assert.deepEqual(pin(pinFile(capture.tools.node.path)), capture.tools.node, 'actual measured Node unchanged');
  const host = capture.host.platform, raw = { host, tools: { buildBefore: build.tools.before,
    buildAfter: build.tools.after, sdk: build.sdk, measured: capture.tools } };
  let liveProvider = null, calibration = null;
  const hostRecord = { host, nativeParentBindingVerified: host === 'windows-x86_64', toolInputs: null };
  if (host === 'windows-x86_64') {
    assert.deepEqual(pin(pinFile(provider.helper.path)), provider.helper, 'actual calibrated job metrics helper unchanged');
    calibration = producerDocument(readSafe(input.calibrationDirectory, 'toolchain-before-build.json', MAX_DOCUMENT));
    hostRecord.toolInputs = { helper: observation.nativeBinding.expected.helper };
  } else {
    const actual = openLinuxTimeProvider({ providerRoot: provider.providerRoot }); actual.check();
    assert.deepEqual(actual.binding, provider, 'same actual freshly checked GNU time provider');
    const receipt = parse(actual.receiptBytes);
    // Compatibility shape for a PURE projection; this is live file verification, never an archive-owner capability.
    liveProvider = { providerOriginalsVerified: true, executable: actual.binding.executable,
      package: receipt.package, providerRecordSha256: actual.binding.providerRecordSha256, observedRuntime: receipt };
  }
  return numericSemanticTools({ raw, host: hostRecord, provider: liveProvider, calibration });
}
export async function finishNumericHeldoutGate(key, beforeKey, input, root, output, build, capture, provider) {
  assert.equal(arguments.length, 8, 'fixed held-out gate finish');
  const value = own(key, beforeKey, input, root, output); value.used = true; // Consume before awaits; failure is terminal.
  const original = inspectNumericCompanionBefore(beforeKey, input, root, output);
  assert.deepEqual(original, value.original);
  const anchor = inspectNumericCampaignAnchor(value.anchorKey), source = { commit: input.expectedHead, tree: input.expectedTree };
  const tools = liveTools(original, input, build, capture, provider);
  assert.equal(build.profile.observed, 'release');
  assert.deepEqual({ commit: build.source.after.commit, tree: build.source.after.tree }, source);
  const compared = compareHeldoutTools(anchor, capture.host.platform, source, tools, capture.tools.cli.sha256, anchor.collector.collectorSha256);
  // Large source/tool/provider byte checks precede the final original host/C observations.
  const directory = join(output, 'numeric-premeasurement'); mkdirSync(directory, { mode: 0o700 });
  await captureHostedMetadata({ root, output: directory, phase: 'before', environment: input.environment });
  const freshKey = await captureNumericCompanion(input, root, directory, 'before');
  const fresh = inspectNumericCompanionBefore(freshKey, input, root, directory);
  assert.equal(fresh.companion.collectorNodePid, original.companion.collectorNodePid);
  assert.deepEqual(fresh.nativeBinding, original.nativeBinding, 'same actual native source/tool/SDK/helper material before timing');
  if (fresh.companion.native) assert.deepEqual(fresh.companion.native.decoded.parent, original.companion.native.decoded.parent,
    'same original collector PID and birth before timing');
  const host = compareHeldoutHost(anchor, fresh, source);
  save(output, 'numeric-gate-premeasurement.json', { format: 'zryna.cli-performance-heldout-premeasurement.v1',
    phase: 'immediately-before-timed-round', host, tools: compared, source,
    anchor: anchor.anchor, profileSha256: sha256(canonical(anchor.profile)),
    observedClassMatched: true, observedToolsMatched: true, ...HELDOUT_FLAGS });
}
export function closeNumericHeldoutGate(key) {
  assert.equal(arguments.length, 1);
  const value = gates.get(key); assert(value, 'own held-out gate'); gates.delete(key);
  closeNumericCampaignAnchor(value.anchorKey);
}
