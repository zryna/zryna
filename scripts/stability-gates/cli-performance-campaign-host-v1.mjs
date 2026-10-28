// Original-byte host facts only; global source/runtime/performance admission remains separate.
import assert from 'node:assert/strict';
import { exact, MAX_DOCUMENT, parse, sha256 } from './input.mjs';
import { inspectNumericCampaignPhase } from './cli-performance-campaign-link-v1.mjs';
import { inspectNumericCampaignSource, readNumericCampaignOriginal } from './cli-performance-campaign-source-v1.mjs';
import { inspectNumericCampaignArchive, readNumericCampaignArchiveEntry } from './cli-performance-campaign-archive-v1.mjs';
import { projectNumericHostFacts } from './cli-performance-campaign-host-facts-v1.mjs';
import { bindWindowsHostObservations, COMPANION_FLAGS } from './cli-performance-campaign-windows-host-v1.mjs';
const hosts = new WeakMap(), PREFIX = 'm418-hosted-observations/capture/';
const pid = value => { assert(Number.isSafeInteger(value) && value > 0 && value <= 0xffffffff); return value; };
function linuxPid(record, expected) {
  const text = record.affinity.text;
  assert(typeof text === 'string' && Buffer.byteLength(text) <= 65536);
  for (const key of ['Pid', 'Tgid']) {
    const rows = text.split('\n').filter(row => row.startsWith(`${key}:`)); assert.equal(rows.length, 1);
    const value = rows[0].slice(key.length + 1).trim(); assert(/^[1-9][0-9]{0,9}$/u.test(value));
    assert.equal(pid(Number(value)), expected, 'original proc process identity matches the collector companion');
  }
}
function originalHost(host, operator, phase, windows) {
  exact(host, ['format', 'phase', 'platform', 'observedAt', 'image', 'kernel', 'cpu', 'memory', 'exclusiveHost',
    'sourceAuthenticated', 'performanceQualified', 'baselineAuthority', 'thresholds', 'affinity', 'quotas', 'power',
    ...(!windows ? ['cgroupMembership', 'quotaScope'] : [])], 'original host schema');
  assert.equal(host.baselineAuthority, null); assert.equal(host.thresholds, null);
  assert(typeof host.observedAt === 'string' && Number.isFinite(Date.parse(host.observedAt)));
  exact(host.cpu, ['availableParallelism', 'logicalCount', 'models', 'reportedMHz'], 'original CPU fields');
  assert(Array.isArray(host.cpu.reportedMHz) && host.cpu.reportedMHz.length === host.cpu.logicalCount);
  for (const mhz of host.cpu.reportedMHz) assert(Number.isFinite(mhz) && mhz >= 0);
  exact(host.memory, ['totalBytes', 'freeBytesAtObservation'], 'original memory fields');
  assert(Number.isSafeInteger(host.memory.totalBytes) && host.memory.totalBytes > 0);
  assert(Number.isSafeInteger(host.memory.freeBytesAtObservation) && host.memory.freeBytesAtObservation >= 0 &&
    host.memory.freeBytesAtObservation <= host.memory.totalBytes);
  exact(host.image, ['ImageOS', 'ImageVersion', 'RUNNER_OS', 'RUNNER_ARCH', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT'], 'original image projection');
  for (const key of Object.keys(host.image))
    assert.deepEqual(host.image[key], { status: 'observed-environment', value: operator.environment[key] });
  assert.equal(operator.environment.GITHUB_RUN_ID, String(phase.plan.runId));
  assert.equal(operator.environment.GITHUB_RUN_ATTEMPT, '1');
}
export function reconcileNumericCampaignHost(selection, sourceKey, rawKey) {
  assert.equal(arguments.length, 3, 'fixed live phase/source/raw owner; no fact or native projection override');
  const phase = inspectNumericCampaignPhase(selection), source = inspectNumericCampaignSource(selection, sourceKey);
  const archive = inspectNumericCampaignArchive(selection, sourceKey, rawKey);
  const selected = phase.jobs.filter(job => job.jobId === archive.jobId && job.host === archive.host && job.role === archive.round);
  assert.equal(selected.length, 1, 'same actual selected job and original raw archive');
  const job = selected[0], expectedSource = { commit: source.commit, tree: source.tree }, bindings = [];
  const read = (path, maximum) => {
    const bytes = readNumericCampaignArchiveEntry(selection, sourceKey, rawKey, path, maximum);
    bindings.push({ path, bytes: bytes.length, sha256: sha256(bytes) }); return bytes;
  };
  const json = relative => parse(read(PREFIX + relative, MAX_DOCUMENT));
  const operator = json('operator-input.json');
  assert.equal(operator.expectedHead, source.commit); assert.equal(operator.expectedTree, source.tree);
  assert.equal(operator.round, job.role);
  const originals = [], companions = [];
  for (const name of ['before', 'after']) {
    const raw = read(`${PREFIX}host-${name}.json`, MAX_DOCUMENT), host = parse(raw);
    const companion = json(`numeric-host-${name}.json`), windows = archive.host === 'windows-x86_64';
    exact(companion, ['format', 'phase', 'platform', 'source', 'collectorNodePid', 'original', 'kernel',
      ...(windows ? ['native'] : []), ...Object.keys(COMPANION_FLAGS)], 'original companion host record');
    assert.equal(companion.format, 'zryna.cli-performance-numeric-host-companion.v1');
    assert.equal(companion.phase, name); assert.equal(companion.platform, archive.host);
    assert.equal(host.phase, name); assert.equal(host.platform, archive.host);
    originalHost(host, operator, phase, windows);
    assert.deepEqual(companion.source, expectedSource);
    assert.deepEqual(companion.kernel, host.kernel);
    assert.deepEqual(companion.original, { path: `host-${name}.json`, bytes: raw.length, sha256: sha256(raw) });
    for (const [key, value] of Object.entries(COMPANION_FLAGS)) assert.equal(companion[key], value);
    pid(companion.collectorNodePid);
    if (companions.length) assert.equal(companion.collectorNodePid, companions[0].collectorNodePid, 'same original collector PID');
    if (!windows) linuxPid(host, companion.collectorNodePid);
    originals.push(host); companions.push(companion);
  }
  assert.equal(originals[0].cpu.logicalCount, originals[1].cpu.logicalCount);
  assert.equal(originals[0].memory.totalBytes, originals[1].memory.totalBytes);
  const windows = archive.host === 'windows-x86_64';
  const native = windows ? bindWindowsHostObservations({ source: expectedSource, operator, tools: archive.tools,
    companions, read, readOriginal: (path, maximum) => readNumericCampaignOriginal(selection, sourceKey, path, maximum) }) : null;
  const projected = originals.map((host, index) => projectNumericHostFacts(archive.host, host,
    native?.decoded[index] ?? null, job.virtualization));
  assert.deepEqual(projected[0], projected[1], 'original before/after host facts and class agree');
  const value = { format: 'zryna.cli-performance-campaign-host.v1', host: archive.host, source: expectedSource,
    jobId: archive.jobId, artifact: archive.artifact, classId: projected[0].classId, facts: projected[0].facts,
    acceptedUnknownFacts: Object.entries(projected[0].facts).filter(([, fact]) => fact.status === 'unavailable').map(([name]) => name),
    parent: native?.parent ?? { pid: companions[0].collectorNodePid, created: null },
    originalBindings: bindings, toolInputs: native?.toolInputs ?? null,
    materialScope: native?.materialScope ?? 'Original proc PID/affinity and companion bytes only; no native birth proof',
    hostFactBytesVerified: true, nativeParentBindingVerified: windows,
    ...COMPANION_FLAGS, baselineAdmitted: false };
  const key = Object.freeze({}); hosts.set(key, { selection, sourceKey, rawKey, value }); return key;
}
export function inspectNumericCampaignHost(selection, sourceKey, rawKey, hostKey) {
  assert.equal(arguments.length, 4);
  const value = hosts.get(hostKey);
  assert(value && value.selection === selection && value.sourceKey === sourceKey && value.rawKey === rawKey,
    'own host key and SAME phase/source/raw keys, not a copied host report');
  inspectNumericCampaignArchive(selection, sourceKey, rawKey); // Closed source/archive keys cannot be revived by a host copy.
  return structuredClone(value.value);
}

// Reuse the unchanged original record validator; this supplies no owner key or authority.
export { originalHost as validateNumericOriginalHost };
