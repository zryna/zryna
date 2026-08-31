import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { loadAndValidateM2Conformance } from './check-m2-conformance.mjs';

const scriptPath = fileURLToPath(import.meta.url);
const workspaceRoot = resolve(dirname(scriptPath), '..');
const expectedCommandsSha256 = 'ab4433d19f862f1c6a189023e5d9d265051795c18bc97fd4cc66bb4f6031fde3';
const maximumOutputBytes = 8 * 1024 * 1024;

export const M2_CONFORMANCE_COMMANDS = Object.freeze([
  Object.freeze({
    id: 'registry-and-fixture-contract',
    executable: 'node',
    args: Object.freeze(['--test', 'tests/m2-conformance.test.mjs']),
    timeout: 30_000,
  }),
  Object.freeze({
    id: 'public-fixed-oracle-corpus',
    executable: 'cargo',
    args: Object.freeze([
      'test',
      '--locked',
      '-p',
      'zryna',
      '--test',
      'm2_conformance',
      '--',
      '--nocapture',
    ]),
    timeout: 5 * 60_000,
  }),
  Object.freeze({
    id: 'internal-boundary-evidence',
    executable: 'node',
    args: Object.freeze(['scripts/run-m2-quick.mjs']),
    timeout: 10 * 60_000,
  }),
]);

export function m2ConformanceCommandDigest(commands = M2_CONFORMANCE_COMMANDS) {
  return createHash('sha256').update(JSON.stringify(commands)).digest('hex');
}

export function validateM2ConformanceCommands(commands = M2_CONFORMANCE_COMMANDS) {
  if (m2ConformanceCommandDigest(commands) !== expectedCommandsSha256) {
    throw new Error('M2 conformance commands differ from the frozen command set');
  }
  return commands;
}

export function runM2Conformance(commands = M2_CONFORMANCE_COMMANDS, spawn = spawnSync) {
  loadAndValidateM2Conformance();
  for (const [index, command] of commands.entries()) {
    process.stdout.write(`\n[m2 ${index + 1}/${commands.length}] ${command.id}\n`);
    const result = spawn(command.executable, command.args, {
      cwd: workspaceRoot,
      env: process.env,
      shell: false,
      encoding: 'utf8',
      maxBuffer: maximumOutputBytes,
      timeout: command.timeout,
      windowsHide: true,
    });
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    if (result.error) throw new Error(`${command.id} could not complete: ${result.error.message}`);
    if (result.status !== 0) {
      throw new Error(`${command.id} failed with exit status ${result.status ?? 'unknown'}`);
    }
  }
  process.stdout.write(`\nM2 conformance passed: ${commands.length} ordered gates.\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
  try {
    runM2Conformance(validateM2ConformanceCommands());
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
