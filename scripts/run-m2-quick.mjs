import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const workspaceRoot = resolve(dirname(scriptPath), '..');
const expectedCommandsSha256 = '8b1fcab7ceaa5777210c08f077e90603645946b3eb46176cf1d0c084f4b04898';
const maximumOutputBytes = 8 * 1024 * 1024;

function command(id, executable, args) {
  return Object.freeze({ id, executable, args: Object.freeze(args), timeout: 5 * 60_000 });
}

export const M2_QUICK_COMMANDS = Object.freeze([
  command('portable-contracts', 'node', [
    '--test', 'tests/m2-conformance.test.mjs', 'tests/m2-manifest-contract.test.mjs',
  ]),
  command('adapter-boundaries', 'node', [
    '--test', 'adapters/typescript-6/test/worker-v3.test.mjs',
  ]),
  command('public-control-flow-profile', 'cargo', [
    'test', '--locked', '-p', 'zryna', '--test', 'cli', 'control_flow_', '--', '--nocapture',
  ]),
  command('driver-pipeline', 'cargo', [
    'test', '--locked', '-p', 'zryna-driver', '--lib', 'pipeline::tests::',
  ]),
  command('javascript-backend', 'cargo', [
    'test', '--locked', '-p', 'zryna-backend-javascript',
  ]),
  command('webassembly-backend', 'cargo', [
    'test', '--locked', '-p', 'zryna-backend-webassembly',
  ]),
  command('native-backend', 'cargo', [
    'test', '--locked', '-p', 'zryna-backend-native',
  ]),
  command('native-mir', 'cargo', [
    'test', '--locked', '-p', 'zryna-native-mir',
  ]),
  command('syntax-boundaries', 'cargo', [
    'test', '--locked', '-p', 'zryna-syntax', '--lib', 'v3::tests::',
  ]),
  command('portable-ir', 'cargo', [
    'test', '--locked', '-p', 'zryna-ir', '--lib', 'control_flow_v1',
  ]),
  command('control-flow-semantics', 'cargo', [
    'test', '--locked', '-p', 'zryna-semantics', '--lib', 'control_flow_v1',
  ]),
  command('native-link-run', 'cargo', [
    'test', '--locked', '-p', 'zryna-driver', '--lib', 'control_flow_native',
  ]),
  command('retained-stage-identity', 'cargo', [
    'test', '--locked', '-p', 'zryna-driver', '--lib', 'retained_stage_identity',
  ]),
  command('module-closure', 'cargo', [
    'test', '--locked', '-p', 'zryna-driver', '--lib', 'module_closure',
  ]),
  command('workspace-source', 'cargo', [
    'test', '--locked', '-p', 'zryna-driver', '--lib', 'workspace_source::',
  ]),
]);

export function m2QuickCommandDigest(commands = M2_QUICK_COMMANDS) {
  return createHash('sha256').update(JSON.stringify(commands)).digest('hex');
}

export function validateM2QuickCommands(commands = M2_QUICK_COMMANDS) {
  if (m2QuickCommandDigest(commands) !== expectedCommandsSha256) {
    throw new Error('M2 quick commands differ from the frozen command set');
  }
  return commands;
}

export function runM2Quick(commands = M2_QUICK_COMMANDS, spawn = spawnSync) {
  for (const [index, entry] of commands.entries()) {
    process.stdout.write(`\n[m2-quick ${index + 1}/${commands.length}] ${entry.id}\n`);
    const result = spawn(entry.executable, entry.args, {
      cwd: workspaceRoot,
      env: process.env,
      shell: false,
      encoding: 'utf8',
      maxBuffer: maximumOutputBytes,
      timeout: entry.timeout,
      windowsHide: true,
    });
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    if (result.error) throw new Error(`${entry.id} could not complete: ${result.error.message}`);
    if (result.status !== 0) {
      throw new Error(`${entry.id} failed with exit status ${result.status ?? 'unknown'}`);
    }
  }
  process.stdout.write(`\nM2 quick gate passed: ${commands.length} ordered checks.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  try {
    runM2Quick(validateM2QuickCommands());
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
