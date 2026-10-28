// Unqualified parameters only; neither a threshold issuer nor an authentication capability.
import assert from 'node:assert/strict';
import { canonical, exact, parse, sha256 } from './input.mjs';
import { matrixFor, validateMethodology } from './cli-performance-methodology-v4.mjs';
import { integer, MAX_WALL_NS, ratio } from './cli-performance-numeric-math-v1.mjs';
export const HOSTS = ['linux-x86_64', 'windows-x86_64'];
export const ROLES = ['baseline-round-1', 'baseline-round-2', 'held-out-candidate'];
export const METHODS = {
  'linux-x86_64': { wall: 'monotonic-process-wall-ns.v1', memory: 'gnu-time-wait4-maxrss-kib.v1' },
  'windows-x86_64': { wall: 'qpc-wall-ticks.v1', memory: 'job-peak-committed-bytes.v1' },
};
const FACTS = ['cpuModel', 'osImage', 'kernel', 'visibleCpuCount', 'affinity', 'virtualization',
  'outerCpuQuota', 'outerMemoryQuota', 'powerManagement', 'physicalExclusivity'];
export function digest(value, size = 64) {
  assert(typeof value === 'string' && new RegExp(`^[a-f0-9]{${size}}$`, 'u').test(value), 'exact digest');
}
export function text(value) {
  assert(typeof value === 'string' && value.length > 0 && value.length <= 512 &&
    value.trim() === value && !/[\x00-\x1f\x7f]/u.test(value), 'bounded literal class fact');
}
export function time(value) {
  assert(typeof value === 'string' && /^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{3}Z$/u.test(value));
  const milliseconds = Date.parse(value);
  assert(Number.isSafeInteger(milliseconds) && new Date(milliseconds).toISOString() === value, 'exact UTC timestamp');
  return milliseconds;
}
export function unqualified(value) {
  for (const key of ['sourceAuthenticated', 'proofQualified', 'performanceQualified']) assert.equal(value[key], false, key);
  assert.equal(value.closureStatus, 'blocked'); assert.equal(value.thresholdAuthority, null);
}
export function source(value) {
  exact(value, ['commit', 'tree'], 'source identity'); digest(value.commit, 40); digest(value.tree, 40);
}
export function identity(value) {
  exact(value, ['jobId', 'artifactId', 'rawSha256'], 'declared raw round identity');
  integer(value.jobId); integer(value.artifactId); digest(value.rawSha256);
}
function build(value) {
  exact(value, ['class', 'cliSha256'], 'common declared release image');
  assert.equal(value.class, 'release'); digest(value.cliSha256);
}
function hostClass(row, host) {
  exact(row, ['host', 'classId', 'facts', 'acceptedUnknownFacts', 'wallMethod', 'memoryMethod',
    'referenceBuild', 'candidateBuild'], 'host policy');
  assert.equal(row.host, host); text(row.classId);
  assert.equal(row.wallMethod, METHODS[host].wall); assert.equal(row.memoryMethod, METHODS[host].memory);
  exact(row.facts, FACTS, 'required host facts');
  const unknown = [];
  for (const [index, key] of FACTS.entries()) {
    const fact = row.facts[key];
    if (fact?.status === 'unavailable') {
      exact(fact, ['status', 'reason'], 'unavailable fact'); text(fact.reason);
      assert(index >= 6, 'CPU, image, kernel, visible allocation, affinity and virtualization must be observed');
      unknown.push(key);
    } else {
      exact(fact, ['status', 'value'], 'observed fact'); assert.equal(fact.status, 'observed'); text(fact.value);
      if (key === 'visibleCpuCount') integer(fact.value);
    }
  }
  assert.deepEqual(row.acceptedUnknownFacts, unknown, 'each accepted unknown is named, never inferred unlimited');
  build(row.referenceBuild); build(row.candidateBuild);
}
export function validateProfile(profile, catalogue) {
  canonical(profile); validateMethodology(catalogue);
  exact(profile, ['format', 'scope', 'catalogueSha256', 'sourceAuthenticated', 'proofQualified',
    'performanceQualified', 'closureStatus', 'thresholdAuthority', 'sources', 'toolchainSha256',
    'collectorSha256', 'hosts', 'baselinePins', 'anchor', 'cells'], 'prospective numeric profile');
  assert.equal(profile.format, 'zryna.cli-performance-threshold-parameters.v1');
  assert.equal(profile.scope, 'shared-hosted-vm-regression-only'); unqualified(profile);
  assert.equal(profile.catalogueSha256, sha256(canonical(catalogue)), 'frozen v4 workload identity');
  exact(profile.sources, ['reference', 'candidate'], 'source roles');
  source(profile.sources.reference); source(profile.sources.candidate);
  digest(profile.toolchainSha256); digest(profile.collectorSha256);
  assert(Array.isArray(profile.hosts) && profile.hosts.length === 2);
  profile.hosts.forEach((row, index) => hostClass(row, HOSTS[index]));
  assert.notEqual(profile.hosts[0].classId, profile.hosts[1].classId, 'host-native classes remain separate');
  exact(profile.anchor, ['kind', 'recordSha256', 'rationaleSha256', 'recordedAt'], 'declared anchor');
  assert.equal(profile.anchor.kind, 'declared-record-not-authentication');
  digest(profile.anchor.recordSha256); digest(profile.anchor.rationaleSha256); time(profile.anchor.recordedAt);
  assert(Array.isArray(profile.baselinePins) && profile.baselinePins.length === 4);
  const expectedPins = HOSTS.flatMap(host => ROLES.slice(0, 2).map(role => ({ host, role })));
  const identities = { jobId: new Set(), artifactId: new Set(), rawSha256: new Set() };
  profile.baselinePins.forEach((pin, index) => {
    exact(pin, ['host', 'role', 'identity'], 'both baseline pins');
    assert.equal(pin.host, expectedPins[index].host); assert.equal(pin.role, expectedPins[index].role);
    identity(pin.identity);
    for (const [key, seen] of Object.entries(identities)) {
      assert(!seen.has(pin.identity[key]), 'independent baseline identities'); seen.add(pin.identity[key]);
    }
  });
  const expectedCells = HOSTS.flatMap(host => matrixFor(catalogue, host).map(row => ({ host, selector: row.id })));
  assert(Array.isArray(profile.cells) && profile.cells.length === 44);
  profile.cells.forEach((cell, index) => {
    exact(cell, ['host', 'selector', 'maximumWallNs', 'maximumMemoryNativeUnits', 'wallSpread',
      'memorySpread', 'wallCrossRoundRatio', 'memoryCrossRoundRatio', 'wallRegressionRatio', 'memoryRegressionRatio'], 'cell limits');
    assert.equal(cell.host, expectedCells[index].host); assert.equal(cell.selector, expectedCells[index].selector);
    assert(integer(cell.maximumWallNs) <= MAX_WALL_NS, 'no threshold outside existing hosted containment');
    integer(cell.maximumMemoryNativeUnits);
    for (const key of ['wallSpread', 'memorySpread']) ratio(cell[key]);
    for (const key of ['wallCrossRoundRatio', 'memoryCrossRoundRatio', 'wallRegressionRatio', 'memoryRegressionRatio']) ratio(cell[key], true);
  });
  return profile;
}
export function parseProfile(bytes, catalogue) {
  return validateProfile(parse(bytes), catalogue);
}
