import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
export const REGISTRY_SHA256 = '7579250d3c6c7a6566f90552ab00e5847644e01bca07c9bee9ffc3045440cd3b';
export const REGISTRY_PATH = 'tests/m7-generic-acceptance-v1.json';
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
export function platformName(platform = process.platform, arch = process.arch) {
  if (arch !== 'x64') throw new Error('M7 packet requires the existing x86-64 CI host');
  if (platform === 'linux') return 'linux';
  if (platform === 'win32') return 'windows';
  throw new Error(`M7 packet does not claim this platform: ${platform}/${arch}`);
}
export function loadRegistry(root) {
  const bytes = readFileSync(resolve(root, REGISTRY_PATH));
  if (bytes.length > 256 * 1024 || sha256(bytes) !== REGISTRY_SHA256) {
    throw new Error('M7 acceptance registry differs from its reviewed fixed command set');
  }
  return JSON.parse(bytes.toString('utf8'));
}
export function commandsFor(registry, platform) {
  if (!['linux', 'windows'].includes(platform)) throw new Error('unknown M7 host');
  const selected = registry.commands.filter(command => command.platforms.includes(platform));
  if (selected.length !== (platform === 'linux' ? 1 : 3)) throw new Error('incomplete M7 command selection');
  for (const command of selected) {
    if (!Array.isArray(command.tests[platform]) || command.tests[platform].length === 0) {
      throw new Error(`${command.id}: missing actual nonzero test inventory`);
    }
  }
  return selected;
}
export function verifyTestOutput(stdout, stderr, command, platform) {
  const output = `${stdout}\n${stderr}`.replaceAll('\r\n', '\n');
  const passed = [...output.matchAll(/^test ([A-Za-z0-9_:]+) \.\.\. ok$/gmu)].map(match => match[1]);
  const expected = command.tests[platform];
  if (new Set(passed).size !== passed.length || passed.length !== expected.length
      || [...passed].sort().join('\n') !== [...expected].sort().join('\n')) {
    throw new Error(`${command.id}: exact required tests did not each execute and pass once`);
  }
  const summaries = [...output.matchAll(/^test result: (ok|FAILED)\. ([0-9]+) passed; ([0-9]+) failed; ([0-9]+) ignored; ([0-9]+) measured; ([0-9]+) filtered out;/gmu)];
  if (summaries.length !== 1 || summaries[0][1] !== 'ok'
      || Number(summaries[0][2]) !== expected.length || Number(summaries[0][3]) !== 0
      || Number(summaries[0][4]) !== 0 || Number(summaries[0][5]) !== 0
      || !Number.isSafeInteger(Number(summaries[0][6]))) {
    throw new Error(`${command.id}: zero, failed, ignored or extra test summary is not acceptance`);
  }
  return { executed: expected.length, passed: expected.length, failed: 0, ignored: 0,
    measured: 0, filtered: Number(summaries[0][6]), names: passed };
}
