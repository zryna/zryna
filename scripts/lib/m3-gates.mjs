import { spawnSync } from 'node:child_process';
import { workspaceRoot, digest, loadAndValidateM3Conformance } from '../check-m3-conformance.mjs';

function command(id, executable, args, timeout = 10 * 60_000) {
  return Object.freeze({ id, executable, args: Object.freeze(args), timeout });
}
const cargo = (id, args) => command(id, 'cargo', ['test', '--locked', ...args]);
export const QUICK = Object.freeze([
  command('registry-self-tests', 'node', ['--test', 'tests/m3-conformance.test.mjs', 'tests/m3-public-docs.test.mjs']),
  command('v4-source-contract', 'node', ['--test', 'tests/syntax-protocol-v4.test.mjs', 'adapters/typescript-6/test/worker-v4.test.mjs']),
  cargo('candidate-fixed-oracles', ['-p', 'zryna-driver', '--lib', 'ownership_commands::conformance::']),
  cargo('public-profile-corpus', ['-p', 'zryna', '--test', 'm3_public']),
]);
export const FULL = Object.freeze([
  ...QUICK,
  cargo('layout-boundaries', ['-p', 'zryna-layout', '--', '--include-ignored']),
  cargo('runtime-abi-boundaries', ['-p', 'zryna-ownership-runtime-abi', '--', '--include-ignored']),
  cargo('ownership-ir-boundaries', ['-p', 'zryna-ir', 'data_ownership_v1', '--', '--include-ignored']),
  cargo('ownership-source-boundaries', ['-p', 'zryna-semantics', 'data_ownership_v1', '--', '--include-ignored']),
  cargo('targets-and-native-mir', ['-p', 'zryna-backend-javascript', '-p', 'zryna-backend-webassembly', '-p', 'zryna-backend-native', '-p', 'zryna-native-mir', '--', '--include-ignored']),
  cargo('candidate-security-runtime-and-transactions', ['-p', 'zryna-driver', 'ownership_', '--', '--include-ignored']),
]);
const quickDigest = 'da1fd773a800a162fba3b3d275d8290d37c76643d4e6cb468b49936210348634';
const fullDigest = '8586548e11c9c588e9dd1fe314c9913b475943cf82d1938ff1a5a353279f2aaa';
export function validateCommands(commands, full = false) {
  if (digest(JSON.stringify(commands)) !== (full ? fullDigest : quickDigest)) {
    throw new Error('M3 command authority differs from the frozen inventory');
  }
  return commands;
}
export function runCommand(entry, spawn = spawnSync) {
  console.log(`[m3] ${entry.id}`);
  const result = spawn(entry.executable, entry.args, {
    cwd: workspaceRoot, env: process.env, shell: false, windowsHide: true,
    encoding: 'utf8', timeout: entry.timeout, maxBuffer: 16 * 1024 * 1024,
  });
  if (result.stdout) process.stdout.write(result.stdout);
  if (result.stderr) process.stderr.write(result.stderr);
  if (result.error || result.status !== 0 || result.signal) {
    throw new Error(`${entry.id} failed: ${result.error?.message ?? result.signal ?? result.status}`);
  }
  const output = result.stdout ?? '';
  if (entry.executable === 'cargo' && !/test result: ok\. [1-9][0-9]* passed;/.test(output)) {
    throw new Error(`${entry.id} executed no passing tests`);
  }
}
export function runM3(full = false, spawn = spawnSync, host = process.platform, arch = process.arch) {
  if (!['linux', 'win32'].includes(host) || arch !== 'x64') throw new Error('unsupported M3 gate host');
  const registry = loadAndValidateM3Conformance();
  for (const entry of validateCommands(full ? FULL : QUICK, full)) runCommand(entry, spawn);
  if (full) for (const evidence of registry.evidence) {
    if (evidence.host === 'linux-x86_64' && host !== 'linux') {
      console.log(`[m3] ${evidence.id}: unsupported on Windows; Linux evidence required by aggregate`);
      continue;
    }
    runCommand(cargo(evidence.id, ['-p', evidence.package, '--lib', evidence.test, '--', '--exact', '--include-ignored']), spawn);
  }
  console.log(`M3 ${full ? 'full' : 'quick'} gate passed on ${host}/${arch}.`);
}
