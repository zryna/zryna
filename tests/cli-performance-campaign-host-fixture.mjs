// Shared synthetic original host records; no native helper, workload or network runs.
import { win32 } from 'node:path';
import { canonical, sha256 } from '../scripts/stability-gates/input.mjs';
import { COMPANION_FLAGS } from '../scripts/stability-gates/cli-performance-campaign-windows-host-v1.mjs';
import { PREFIX } from './cli-performance-campaign-windows-host-fixture.mjs';
export function addHosts(f, change = () => {}, native = null) {
  for (const phase of ['before', 'after']) {
    const host = { format: 'zryna.cli-performance-host-observation.v1', phase, platform: f.host,
      observedAt: '2026-10-28T00:00:00.000Z', image: Object.fromEntries(
        ['ImageOS', 'ImageVersion', 'RUNNER_OS', 'RUNNER_ARCH', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT']
          .map(key => [key, { status: 'observed-environment', value: f.release.operator.environment[key] }])),
      kernel: { type: 'Linux', release: 'fixture', version: 'synthetic', machine: 'x86_64' },
      cpu: { availableParallelism: 2, logicalCount: 2, models: ['synthetic CPU'], reportedMHz: [0, 0] },
      memory: { totalBytes: 1024, freeBytesAtObservation: phase === 'before' ? 512 : 256 },
      exclusiveHost: { status: 'unavailable', reason: 'synthetic' },
      affinity: { status: 'observed', path: '/proc/self/status',
        text: 'Pid:\t123\nTgid:\t123\nCpus_allowed:\t3\nCpus_allowed_list:\t0-1\n' },
      cgroupMembership: { status: 'unavailable', path: '/proc/self/cgroup', reason: 'synthetic' },
      quotas: [], quotaScope: 'synthetic retained observations', power: [],
      sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
    if (native) {
      host.kernel.type = 'Windows_NT'; delete host.cgroupMembership; delete host.quotaScope;
      host.quotas = { status: 'unavailable', reason: 'host Job/container CPU and memory quotas not exposed by Node' };
      for (const name of ['affinity', 'power']) {
        const command = name === 'affinity'
          ? { file: win32.join(f.release.operator.environment.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe'),
            args: ['-NoProfile', '-NonInteractive', '-Command', '(Get-Process -Id 123).ProcessorAffinity.ToInt64().ToString()'], cwd: f.release.operator.root, timeoutMs: 10000 }
          : { file: win32.join(f.release.operator.environment.SystemRoot, 'System32', 'powercfg.exe'), args: ['/getactivescheme'], cwd: f.release.operator.root, timeoutMs: 10000 };
        const bytes = Buffer.alloc(0);
        host[name] = { status: 'unavailable', command, exitCode: 1, error: null, signal: null,
          stdoutSha256: sha256(bytes), stderrSha256: sha256(bytes), stdout: '', stderr: '' };
        f.files.set(PREFIX + `host-${phase}-${name}.command.json`, Buffer.from(canonical(command)));
        for (const stream of ['stdout', 'stderr']) f.files.set(PREFIX + `host-${phase}-${name}.${stream}.raw`, bytes);
      }
    }
    if (native) { host.cpu.logicalCount = 4; host.cpu.availableParallelism = 4; host.cpu.reportedMHz = [0, 0, 0, 0]; }
    change(host, phase);
    const bytes = Buffer.from(canonical(host)); f.files.set(PREFIX + `host-${phase}.json`, bytes);
    f.files.set(PREFIX + `numeric-host-${phase}.json`, Buffer.from(canonical({
      format: 'zryna.cli-performance-numeric-host-companion.v1', phase, platform: f.host,
      source: { commit: f.source.commit, tree: f.source.tree }, collectorNodePid: 123, kernel: host.kernel,
      original: { path: `host-${phase}.json`, bytes: bytes.length, sha256: sha256(bytes) },
      ...(native ? { native: native.companions[phase === 'before' ? 0 : 1].native } : {}), ...COMPANION_FLAGS })));
  }
}
