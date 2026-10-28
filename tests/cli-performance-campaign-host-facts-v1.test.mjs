import assert from 'node:assert/strict';
import test from 'node:test';
import { projectNumericHostFacts } from '../scripts/stability-gates/cli-performance-campaign-host-facts-v1.mjs';
const service = { status: 'observed', value: 'github-actions-hosted-vm' };
function fixture(windows = false) {
  const env = value => ({ status: 'observed-environment', value });
  const platform = windows ? 'windows-x86_64' : 'linux-x86_64';
  const host = { format: 'zryna.cli-performance-host-observation.v1', platform,
    cpu: { models: ['synthetic CPU'], logicalCount: 4, availableParallelism: 4, reportedMHz: [1, 2, 3, 4] },
    kernel: { type: windows ? 'Windows_NT' : 'Linux', release: 'synthetic', version: 'synthetic', machine: 'x86_64' },
    image: { RUNNER_OS: env(windows ? 'Windows' : 'Linux'), RUNNER_ARCH: env('X64'),
      ImageOS: env(windows ? 'win22' : 'ubuntu24'), ImageVersion: env('synthetic-image-version') },
    affinity: { status: 'observed', text: 'Pid:\t123\nCpus_allowed:\t0000000f\nCpus_allowed_list:\t0-3\n' },
    sourceAuthenticated: false, performanceQualified: false };
  const native = windows ? { format: 'zryna.windows-parent-affinity-projection.v1', status: 'observed-unqualified',
    hostAdmitted: false, affinity: { groupId: 0, processMask: '15', systemMask: '15', maskBitCount: 4, activeProcessorCount: 4 } } : null;
  return { platform, host, native };
}
const run = f => projectNumericHostFacts(f.platform, f.host, f.native, service);
test('host-native affinity defines visible allocation while every outer resource unknown remains explicit', () => {
  for (const windows of [false, true]) {
    const f = fixture(windows), before = structuredClone(f), result = run(f);
    assert.equal(result.facts.visibleCpuCount.value, '4'); assert.deepEqual(result.facts.virtualization, service);
    for (const key of ['outerCpuQuota', 'outerMemoryQuota', 'powerManagement', 'physicalExclusivity'])
      assert.equal(result.facts[key].status, 'unavailable');
    assert.match(result.classId, /^shared-hosted-vm:/u); assert.deepEqual(f, before);
    f.host.cpu.reportedMHz = [9, 9, 9, 9]; assert.equal(run(f).classId, result.classId, 'MHz is not a promised power-policy observation');
    f.host.kernel.release = 'different'; assert.notEqual(run(f).classId, result.classId);
  }
});
test('Linux multiword masks require exact ordered allowed ranges without duplicate or omitted CPUs', () => {
  const f = fixture();
  f.host.cpu.logicalCount = 64;
  f.host.affinity.text = 'Cpus_allowed:\t00000003,00000003\nCpus_allowed_list:\t0-1,32-33\n';
  assert.equal(run(f).facts.visibleCpuCount.value, '4');
  for (const list of ['0-1,32', '0-1,1,32-33', '32-33,0-1', '0-65535']) {
    f.host.affinity.text = `Cpus_allowed:\t00000003,00000003\nCpus_allowed_list:\t${list}\n`;
    assert.throws(() => run(f));
  }
});
test('verbatim padded Windows CPU models remain exact class facts without trimming or pooling', () => {
  const f = fixture(true), model = 'AMD EPYC 7763 64-Core Processor                ';
  f.host.cpu.models = [model]; const result = run(f);
  assert.equal(JSON.parse(result.facts.cpuModel.value), model);
  f.host.cpu.models = [model.trimEnd()]; assert.notEqual(run(f).classId, result.classId);
  for (const invalid of ['   ', '', 'CPU\nmodel', 'x'.repeat(513)]) {
    f.host.cpu.models = [invalid]; assert.throws(() => run(f));
  }
});
test('missing required observations, mixed CPU classes and unavailable affinity fail closed', () => {
  for (const mutate of [f => { f.host.cpu.models.push('another CPU'); }, f => { f.host.cpu.models = []; },
    f => { f.host.affinity.status = 'unavailable'; }, f => { f.host.image.ImageVersion.status = 'unavailable'; },
    f => { f.host.cpu.availableParallelism = 5; }, f => { f.host.kernel.machine = ''; },
    f => { f.host.kernel.machine = 'aarch64'; }, f => { f.host.kernel.type = 'Darwin'; },
    f => { f.host.sourceAuthenticated = true; }]) { const f = fixture(); mutate(f); assert.throws(() => run(f)); }
  const f = fixture(); assert.throws(() => projectNumericHostFacts(f.platform, f.host, f.native,
    { status: 'unavailable', reason: 'no platform provenance' }));
});
test('Windows group masks and actual process bit count remain separate from system capacity', () => {
  const f = fixture(true); f.native.affinity.systemMask = '255'; f.native.affinity.activeProcessorCount = 8;
  assert.equal(run(f).facts.visibleCpuCount.value, '4');
  for (const mutate of [g => { g.native.affinity.maskBitCount = 3; }, g => { g.native.affinity.systemMask = '7'; },
    g => { g.native.affinity.systemMask = '255'; },
    g => { g.native.affinity.groupId = 65535; }, g => { g.native.hostAdmitted = true; },
    g => { g.native.affinity.processMask = '18446744073709551616'; }]) {
    const g = fixture(true); mutate(g); assert.throws(() => run(g));
  }
});
