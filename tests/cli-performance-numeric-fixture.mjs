// Synthetic arithmetic declarations only. No real profile, review, source, host or platform authority.
import { readFileSync } from 'node:fs';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { matrixFor, parseMethodology } from '../scripts/stability-gates/cli-performance-methodology-v4.mjs';
import { HOSTS, METHODS, ROLES } from '../scripts/stability-gates/cli-performance-threshold-profile-v1.mjs';
export const catalogue = parseMethodology(readFileSync(new URL('./cli-performance-methodology-v4.json', import.meta.url)));
export const clone = value => structuredClone(value);
export const one = () => ({ numerator: '1', denominator: '1' });
const hash = label => sha256(Buffer.from(`synthetic-only:${label}`));
const observed = value => ({ status: 'observed', value });
const unavailable = () => ({ status: 'unavailable', reason: 'synthetic shared guest cannot observe outer host' });
const identities = HOSTS.flatMap(host => ROLES.map(role => ({ host, role,
  identity: { jobId: String(100 + HOSTS.indexOf(host) * 3 + ROLES.indexOf(role)),
    artifactId: String(200 + HOSTS.indexOf(host) * 3 + ROLES.indexOf(role)), rawSha256: hash(`${host}/${role}`) } })));
const facts = host => ({ cpuModel: observed(`synthetic ${host}`), osImage: observed('synthetic-image'),
  kernel: observed(host === 'linux-x86_64' ? 'synthetic-linux-kernel-v1' : 'synthetic-windows-kernel-v1'),
  visibleCpuCount: observed('4'), affinity: observed('0-3'), virtualization: observed('synthetic-shared-vm'),
  outerCpuQuota: unavailable(), outerMemoryQuota: unavailable(), powerManagement: unavailable(), physicalExclusivity: unavailable() });
const build = label => ({ class: 'release', cliSha256: hash(`${label}-cli`) });
const source = label => ({ commit: hash(`${label}-commit`).slice(0, 40), tree: hash(`${label}-tree`).slice(0, 40) });
const flags = () => ({ sourceAuthenticated: false, proofQualified: false, performanceQualified: false,
  closureStatus: 'blocked', thresholdAuthority: null });
export function fixture() {
  const profile = { format: 'zryna.cli-performance-threshold-parameters.v1', scope: 'shared-hosted-vm-regression-only',
    catalogueSha256: sha256(canonical(catalogue)), ...flags(), sources: { reference: source('reference'), candidate: source('candidate') },
    toolchainSha256: hash('tools'), collectorSha256: hash('collector'), hosts: HOSTS.map(host => ({ host,
      classId: `synthetic-${host}`, facts: facts(host),
      acceptedUnknownFacts: ['outerCpuQuota', 'outerMemoryQuota', 'powerManagement', 'physicalExclusivity'],
      wallMethod: METHODS[host].wall, memoryMethod: METHODS[host].memory,
      referenceBuild: build(`${host}-reference`), candidateBuild: build(`${host}-candidate`) })),
    baselinePins: clone(identities.filter(row => row.role !== 'held-out-candidate')),
    anchor: { kind: 'declared-record-not-authentication', recordSha256: hash('anchor'), rationaleSha256: hash('rationale'),
      recordedAt: '2030-01-04T00:00:00.000Z' },
    cells: HOSTS.flatMap(host => matrixFor(catalogue, host).map(row => ({ host, selector: row.id,
      maximumWallNs: '1000000000', maximumMemoryNativeUnits: '1000', wallSpread: one(), memorySpread: one(),
      wallCrossRoundRatio: one(), memoryCrossRoundRatio: one(), wallRegressionRatio: one(), memoryRegressionRatio: one() }))) };
  const rounds = identities.map(({ host, role, identity }) => {
    const h = profile.hosts[HOSTS.indexOf(host)], ordinal = ROLES.indexOf(role), day = [1, 2, 5][ordinal];
    return { format: 'zryna.cli-performance-numeric-input.v1', host, role, identity: clone(identity),
      source: clone(profile.sources[ordinal === 2 ? 'candidate' : 'reference']),
      build: { ...clone(h[ordinal === 2 ? 'candidateBuild' : 'referenceBuild']), recordSha256: hash(`${host}/${role}/original-build-record`) },
      toolchainSha256: profile.toolchainSha256,
      collectorSha256: profile.collectorSha256, catalogueSha256: profile.catalogueSha256,
      classId: h.classId, classBefore: clone(h.facts), classAfter: clone(h.facts),
      wallMethod: h.wallMethod, memoryMethod: h.memoryMethod,
      startedAt: `2030-01-0${day}T00:00:00.000Z`, endedAt: `2030-01-0${day}T00:01:00.000Z`, ...flags(),
      cells: matrixFor(catalogue, host).map(row => ({ selector: row.id,
        attempts: Array.from({ length: 6 }, (_, index) => ({ index, role: index === 0 ? 'warmup' : 'measured',
          outcome: 'correct-and-clean', ...(host === HOSTS[0] ? { wallNs: '1000000000', peakRssKiB: '1000' } :
            { wallTicks: '10000000', qpcFrequency: '10000000', peakJobMemoryUsedBytes: '1000' }) })) })) };
  });
  return { profile, rounds };
}
