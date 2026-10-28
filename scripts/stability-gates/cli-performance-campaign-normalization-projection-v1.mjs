// Pure declarations only. These projections issue no opaque evidence key or authority.
import assert from 'node:assert/strict';
import { basename } from 'node:path';
import { exact, canonical, parse } from './input.mjs';
import { digest, HOSTS, METHODS } from './cli-performance-threshold-profile-v1.mjs';
const BUILD = ['cargo', 'rustc', 'cc', 'linker', 'node', 'git'];
function pin(role, value) {
  assert(value && Number.isSafeInteger(value.bytes) && value.bytes > 0 && value.bytes <= 512 * 1024 * 1024);
  digest(value.sha256); return { role, bytes: value.bytes, sha256: value.sha256 };
}
const name = path => basename(path.replaceAll('\\', '/'));
function named(prefix, values) {
  assert(Array.isArray(values));
  const result = values.map(value => pin(`${prefix}:${name(value.path)}`, value));
  assert.equal(new Set(result.map(row => row.role)).size, result.length, 'unique logical tool role');
  return result.sort((a, b) => a.role < b.role ? -1 : a.role > b.role ? 1 : 0);
}
export function numericSemanticTools(input) {
  assert.equal(arguments.length, 1); exact(input, ['raw', 'host', 'provider', 'calibration'], 'pure tool projection');
  const { raw, host, provider, calibration } = input;
  assert(HOSTS.includes(raw.host)); assert.equal(host.host, raw.host);
  const release = BUILD.map(role => {
    const before = pin(role, raw.tools.buildBefore[role]), after = pin(role, raw.tools.buildAfter[role]);
    assert.deepEqual(before, after); return after;
  });
  assert.deepEqual(pin('node', raw.tools.measured.node), release.find(row => row.role === 'node'));
  exact(raw.tools.sdk, ['before', 'after'], 'original release SDK epochs');
  assert.deepEqual(raw.tools.sdk.before, raw.tools.sdk.after, 'stable original SDK selection');
  const sdk = raw.tools.sdk.after === null ? [] : named('release-sdk', raw.tools.sdk.after);
  let native, versions;
  if (raw.host === HOSTS[0]) {
    assert.equal(calibration, null); assert(provider && provider.providerOriginalsVerified === true);
    assert.equal(raw.tools.sdk.after, null);
    const r = provider.observedRuntime;
    assert.deepEqual(pin('time', raw.tools.measured.time), pin('time', provider.executable));
    native = [pin('time-package', provider.package), pin('time', provider.executable),
      pin('host-os-release-observed', r.host.osRelease), pin('curl-observed', r.acquisition.curl),
      ...['dpkgDeb', 'dpkgQuery', 'loader', 'libc'].map(role => pin(`${role}-observed`, r.runtime[role])),
      ...named('shared-provider-observed', r.runtime.sharedProviders)];
    versions = { providerRecordSha256: provider.providerRecordSha256, time: r.tool.version,
      curl: r.acquisition.version, runtime: structuredClone(r.runtime.versions) };
  } else {
    assert.equal(provider, null); assert(calibration && host.nativeParentBindingVerified === true);
    assert.equal(sdk.length, 2); assert.equal(calibration.selectedDirectHeaders.length, 12);
    assert.equal(calibration.selectedStaticLibraries.length, 5);
    native = [pin('job-metrics-helper', raw.tools.measured.helper), pin('parent-affinity-helper', host.toolInputs.helper),
      ...named('calibration-header', calibration.selectedDirectHeaders),
      ...named('calibration-library', calibration.selectedStaticLibraries)];
    versions = { compiler: calibration.compilerFileVersion, linker: calibration.linkerFileVersion,
      sdk: calibration.sdkVersion, vcTools: calibration.vcToolsVersion };
  }
  // Snapshot/bound each host selection separately; observed library claims remain explicitly observed.
  return parse(Buffer.from(canonical({ host: raw.host, release, sdk, native, versions })));
}
export function projectNumericRound(input) {
  assert.equal(arguments.length, 1);
  exact(input, ['raw', 'host', 'source', 'collectorSha256', 'toolchainSha256'], 'pure round projection');
  const { raw, host, source, collectorSha256, toolchainSha256 } = input;
  assert(HOSTS.includes(raw.host)); digest(collectorSha256); digest(toolchainSha256);
  const linux = raw.host === HOSTS[0];
  const cells = raw.cells.map(cell => ({ selector: cell.selector, attempts: cell.attempts.map((attempt, index) => {
    assert.equal(attempt.index, index); assert.equal(attempt.role, index === 0 ? 'warmup' : 'measured');
    return { index, role: attempt.role, outcome: 'correct-and-clean', ...(linux
      ? { wallNs: attempt.wallNs, peakRssKiB: attempt.peakRssKiB }
      : { wallTicks: attempt.wallTicks, qpcFrequency: attempt.qpcFrequency, peakJobMemoryUsedBytes: attempt.peakJobMemoryUsedBytes }) };
  }) }));
  return parse(Buffer.from(canonical({ format: 'zryna.cli-performance-numeric-input.v1', host: raw.host,
    role: raw.round, identity: { jobId: String(raw.jobId), artifactId: String(raw.artifact.id), rawSha256: raw.artifact.sha256 },
    source: structuredClone(source), build: { class: 'release', cliSha256: raw.image.sha256,
      recordSha256: raw.originals.releaseBuild.sha256 }, toolchainSha256, collectorSha256,
    catalogueSha256: raw.catalogueSha256, classId: host.classId, classBefore: structuredClone(host.facts),
    classAfter: structuredClone(host.facts), wallMethod: METHODS[raw.host].wall, memoryMethod: METHODS[raw.host].memory,
    startedAt: new Date(raw.startedAt).toISOString(), endedAt: new Date(raw.completedAt).toISOString(),
    sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
    closureStatus: 'blocked', thresholdAuthority: null, cells })));
}
