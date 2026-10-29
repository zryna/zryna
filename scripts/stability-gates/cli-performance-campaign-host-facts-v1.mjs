// Pure projection of observations only. The archive/platform owner must establish their provenance.
import assert from 'node:assert/strict';
import { canonical, exact, sha256 } from './input.mjs';
import { text } from './cli-performance-threshold-profile-v1.mjs';
const observed = value => { text(value); return { status: 'observed', value }; };
const unavailable = reason => ({ status: 'unavailable', reason });
function positive(value) { assert(Number.isSafeInteger(value) && value > 0 && value <= 65536); return value; }
function environment(image, key) {
  assert.equal(image[key]?.status, 'observed-environment'); text(image[key].value); return image[key].value;
}
function linuxAffinity(host) {
  assert.equal(host.affinity?.status, 'observed');
  const input = host.affinity.text; assert(typeof input === 'string' && Buffer.byteLength(input) <= 65536);
  const field = name => {
    const rows = input.split('\n').filter(row => row.startsWith(`${name}:`)); assert.equal(rows.length, 1);
    const value = rows[0].slice(name.length + 1).trim(); text(value); return value;
  };
  const maskText = field('Cpus_allowed'), list = field('Cpus_allowed_list');
  assert(/^[a-f0-9]{1,8}(?:,[a-f0-9]{8})*$/u.test(maskText));
  const hex = maskText.replaceAll(',', ''), mask = BigInt(`0x${hex}`); assert(mask > 0n);
  let selected = 0n, previous = -1, count = 0;
  for (const range of list.split(',')) {
    const parts = /^(0|[1-9][0-9]{0,4})(?:-(0|[1-9][0-9]{0,4}))?$/u.exec(range); assert(parts);
    const start = Number(parts[1]), end = parts[2] === undefined ? start : Number(parts[2]);
    assert(start > previous && start <= end && end < hex.length * 4, 'ordered bounded affinity ranges');
    const length = end - start + 1; count += length;
    selected |= ((1n << BigInt(length)) - 1n) << BigInt(start); previous = end;
  }
  assert.equal(selected, mask, 'kernel CPU mask and allowed-list agree'); positive(count);
  return { count, value: canonical({ mask: maskText, allowedList: list,
    nodeAvailableParallelism: host.cpu.availableParallelism }).trimEnd() };
}
function windowsAffinity(host, native) {
  assert.equal(native?.format, 'zryna.windows-parent-affinity-projection.v1');
  assert.equal(native.status, 'observed-unqualified'); assert.equal(native.hostAdmitted, false);
  const value = native.affinity;
  exact(value, ['groupId', 'processMask', 'systemMask', 'maskBitCount', 'activeProcessorCount'], 'decoded native affinity');
  assert(Number.isSafeInteger(value.groupId) && value.groupId >= 0 && value.groupId < 65535);
  assert(positive(value.maskBitCount) <= positive(value.activeProcessorCount) && value.activeProcessorCount <= 64);
  for (const key of ['processMask', 'systemMask']) assert(typeof value[key] === 'string' && /^[1-9][0-9]{0,19}$/u.test(value[key]));
  const mask = BigInt(value.processMask), system = BigInt(value.systemMask);
  assert(system <= 0xffffffffffffffffn && (mask & ~system) === 0n);
  let count = 0; for (let bits = mask; bits; bits >>= 1n) count += Number(bits & 1n);
  assert.equal(count, value.maskBitCount);
  let systemCount = 0; for (let bits = system; bits; bits >>= 1n) systemCount += Number(bits & 1n);
  assert(systemCount <= value.activeProcessorCount, 'same-group system mask fits observed active processors');
  return { count, value: canonical({ ...value, nodeAvailableParallelism: host.cpu.availableParallelism }).trimEnd() };
}
export function projectNumericHostFacts(platform, host, native, service) {
  assert.equal(arguments.length, 4); assert(['linux-x86_64', 'windows-x86_64'].includes(platform));
  assert.equal(host.format, 'zryna.cli-performance-host-observation.v1'); assert.equal(host.platform, platform);
  assert.equal(host.sourceAuthenticated, false); assert.equal(host.performanceQualified, false);
  assert.deepEqual(service, { status: 'observed', value: 'github-actions-hosted-vm' });
  assert(Array.isArray(host.cpu.models) && host.cpu.models.length === 1);
  const model = host.cpu.models[0];
  assert(typeof model === 'string' && model.length <= 512 && model.trim().length > 0 && !/[\x00-\x1f\x7f]/u.test(model));
  const modelLiteral = JSON.stringify(model); text(modelLiteral); // Preserve Windows model padding exactly, inside a literal.
  positive(host.cpu.logicalCount); positive(host.cpu.availableParallelism);
  assert(host.cpu.availableParallelism <= host.cpu.logicalCount);
  exact(host.kernel, ['type', 'release', 'version', 'machine'], 'observed kernel');
  for (const value of Object.values(host.kernel)) text(value);
  assert.equal(environment(host.image, 'RUNNER_ARCH'), 'X64');
  const windows = platform === 'windows-x86_64';
  assert.equal(host.kernel.machine, 'x86_64'); assert.equal(host.kernel.type, windows ? 'Windows_NT' : 'Linux');
  assert.equal(environment(host.image, 'RUNNER_OS'), windows ? 'Windows' : 'Linux');
  const imageOS = environment(host.image, 'ImageOS'); assert.equal(imageOS, windows ? 'win22' : 'ubuntu24');
  if (!windows) assert.equal(native, null);
  const affinity = windows ? windowsAffinity(host, native) : linuxAffinity(host);
  assert(affinity.count <= host.cpu.logicalCount && host.cpu.availableParallelism <= affinity.count);
  const facts = { cpuModel: observed(modelLiteral),
    osImage: observed(canonical({ imageOS, version: environment(host.image, 'ImageVersion') }).trimEnd()),
    kernel: observed(canonical(host.kernel).trimEnd()), visibleCpuCount: observed(String(affinity.count)),
    affinity: observed(affinity.value), virtualization: structuredClone(service),
    outerCpuQuota: unavailable('outer CPU quota is not established by the hosted process observations'),
    outerMemoryQuota: unavailable('outer memory quota is not established by the hosted process observations'),
    powerManagement: unavailable('outer power management is not established by the hosted process observations'),
    physicalExclusivity: unavailable('shared hosted VM does not establish exclusive physical hardware') };
  return { facts, classId: `shared-hosted-vm:${platform}:${sha256(Buffer.from(canonical(facts)))}` };
}
