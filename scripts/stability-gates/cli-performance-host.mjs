import assert from 'node:assert/strict';
import { closeSync, openSync, readSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import os from 'node:os';
import { canonical, sha256 } from './input.mjs';
import { execute } from './process.mjs';

function optionalFile(path) {
  let fd;
  try {
    fd = openSync(path, 'r'); const buffer = Buffer.alloc(65537);
    const bytes = readSync(fd, buffer, 0, buffer.length, 0);
    if (bytes > 65536) return { status: 'unavailable', reason: 'observation exceeds 64 KiB', path };
    return { status: 'observed', path, text: buffer.subarray(0, bytes).toString('utf8') };
  } catch (error) { return { status: 'unavailable', path, reason: String(error.code ?? error.message) }; }
  finally { if (fd !== undefined) closeSync(fd); }
}
export function imageProjection(environment) {
  const keys = ['ImageOS', 'ImageVersion', 'RUNNER_OS', 'RUNNER_ARCH', 'GITHUB_RUN_ID', 'GITHUB_RUN_ATTEMPT'];
  return Object.fromEntries(keys.map(key => [key, typeof environment[key] === 'string'
    ? { status: 'observed-environment', value: environment[key] }
    : { status: 'unavailable', reason: 'not exposed by runner' }]));
}
export async function captureHostedMetadata({ root, output, phase, environment }) {
  assert(['before', 'after'].includes(phase));
  const cpus = os.cpus();
  const record = { format: 'zryna.cli-performance-host-observation.v1', phase,
    platform: process.platform === 'win32' ? 'windows-x86_64' : 'linux-x86_64',
    observedAt: new Date().toISOString(), image: imageProjection(environment),
    kernel: { type: os.type(), release: os.release(), version: os.version(), machine: os.machine() },
    cpu: { availableParallelism: os.availableParallelism(), logicalCount: cpus.length,
      models: [...new Set(cpus.map(cpu => cpu.model))], reportedMHz: cpus.map(cpu => cpu.speed) },
    memory: { totalBytes: os.totalmem(), freeBytesAtObservation: os.freemem() },
    exclusiveHost: { status: 'unavailable', reason: 'shared hosted runner; exclusivity not established' },
    sourceAuthenticated: false, performanceQualified: false, baselineAuthority: null, thresholds: null };
  if (process.platform === 'linux') {
    record.affinity = optionalFile('/proc/self/status');
    record.cgroupMembership = optionalFile('/proc/self/cgroup');
    record.quotas = ['/sys/fs/cgroup/cpu.max', '/sys/fs/cgroup/memory.max',
      '/sys/fs/cgroup/cpuset.cpus.effective'].map(optionalFile);
    record.quotaScope = 'observed cgroup v2 root paths; membership retained; nested/v1 limits not inferred';
    record.power = ['/sys/devices/system/cpu/cpu0/cpufreq/scaling_governor',
      '/sys/devices/system/cpu/cpu0/cpufreq/scaling_cur_freq'].map(optionalFile);
  } else {
    record.quotas = { status: 'unavailable', reason: 'host Job/container CPU and memory quotas not exposed by Node' };
    const observations = [
      ['affinity', join(environment.SystemRoot, 'System32', 'WindowsPowerShell', 'v1.0', 'powershell.exe'),
        ['-NoProfile', '-NonInteractive', '-Command', `(Get-Process -Id ${process.pid}).ProcessorAffinity.ToInt64().ToString()`]],
      ['power', join(environment.SystemRoot, 'System32', 'powercfg.exe'), ['/getactivescheme']],
    ];
    for (const [name, file, args] of observations) {
      const command = { file, args, cwd: root, timeoutMs: 10000 };
      writeFileSync(join(output, `host-${phase}-${name}.command.json`), canonical(command), { flag: 'wx' });
      const result = await execute(file, args, { root, timeoutMs: 10000,
        env: { PATH: environment.PATH, SystemRoot: environment.SystemRoot, WINDIR: environment.WINDIR,
          TEMP: environment.TEMP, TMP: environment.TMP } });
      for (const stream of ['stdout', 'stderr'])
        writeFileSync(join(output, `host-${phase}-${name}.${stream}.raw`), result[stream], { flag: 'wx' });
      record[name] = { status: result.exitCode === 0 && !result.error && !result.signal ? 'observed' : 'unavailable',
        command, exitCode: result.exitCode, error: result.error, signal: result.signal,
        stdoutSha256: sha256(result.stdout), stderrSha256: sha256(result.stderr),
        stdout: result.stdout.toString('utf8'), stderr: result.stderr.toString('utf8') };
    }
  }
  writeFileSync(join(output, `host-${phase}.json`), canonical(record), { flag: 'wx', mode: 0o600 });
  return record;
}
