import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const WORKSPACE_ROOT = resolve(dirname(SCRIPT_PATH), '..');
const EXPECTED_COMMANDS_SHA256 = '9f546e43c21bd9cec716bf440e057d66dc01afaf6ddda8cbaa77aa41a0d82cc1';

export const PREFLIGHT_COMMANDS = Object.freeze([
  Object.freeze({
    id: 'portable-contract-tests',
    executable: 'node',
    args: Object.freeze([
      '--test',
      'tests/preflight.test.mjs',
      'tests/m0-conformance.test.mjs',
      'tests/docs-bundle.test.mjs',
      'tests/m2-contract.test.mjs',
      'tests/m3-contract.test.mjs',
      'tests/m2-conformance.test.mjs',
      'tests/syntax-protocol-v2.test.mjs',
      'tests/syntax-protocol-v3.test.mjs',
      'tests/syntax-protocol-v4.test.mjs',
      'adapters/typescript-6/test/worker.test.mjs',
      'adapters/typescript-6/test/worker-v3.test.mjs',
      'adapters/typescript-6/test/worker-v4.test.mjs',
    ]),
  }),
  Object.freeze({
    id: 'rust-format',
    executable: 'cargo',
    args: Object.freeze(['fmt', '--all', '--', '--check']),
  }),
  Object.freeze({
    id: 'm2-semantic-driver-tests',
    executable: 'cargo',
    args: Object.freeze([
      'test',
      '--locked',
      '-p',
      'zryna-semantics',
      '-p',
      'zryna-driver',
      '--lib',
    ]),
  }),
  Object.freeze({
    id: 'm3-layout-tests',
    executable: 'cargo',
    args: Object.freeze(['test', '--locked', '-p', 'zryna-layout']),
  }),
  Object.freeze({
    id: 'm3-data-ir-tests',
    executable: 'cargo',
    args: Object.freeze(['test', '--locked', '-p', 'zryna-ir', 'data_ownership_v1']),
  }),
  Object.freeze({
    id: 'm3-aggregate-semantics-tests',
    executable: 'cargo',
    args: Object.freeze([
      'test',
      '--locked',
      '-p',
      'zryna-semantics',
      'data_ownership_v1',
      '--',
      '--include-ignored',
    ]),
  }),
  Object.freeze({
    id: 'rust-workspace-check',
    executable: 'cargo',
    args: Object.freeze(['check', '--locked', '--workspace', '--all-targets', '--all-features']),
  }),
  Object.freeze({
    id: 'frontend-syntax-tests',
    executable: 'cargo',
    args: Object.freeze(['test', '--locked', '-p', 'zryna-frontend', '-p', 'zryna-syntax']),
  }),
]);

export function preflightCommandDigest(commands = PREFLIGHT_COMMANDS) {
  return createHash('sha256').update(JSON.stringify(commands)).digest('hex');
}

export function validatePreflightCommands(commands = PREFLIGHT_COMMANDS) {
  if (preflightCommandDigest(commands) !== EXPECTED_COMMANDS_SHA256) {
    throw new Error('preflight command declarations differ from the frozen command set');
  }
  return commands;
}

export function runPreflight(commands = PREFLIGHT_COMMANDS, spawn = spawnSync) {
  for (const [index, command] of commands.entries()) {
    console.log(`\n[preflight ${index + 1}/${commands.length}] ${command.id}`);
    const result = spawn(command.executable, command.args, {
      cwd: WORKSPACE_ROOT,
      env: process.env,
      shell: false,
      stdio: 'inherit',
      windowsHide: true,
    });
    if (result.error) throw new Error(`${command.id} could not start: ${result.error.message}`);
    if (result.status !== 0) {
      throw new Error(`${command.id} failed with exit status ${result.status ?? 'unknown'}`);
    }
  }
  console.log(`\nPreflight passed: ${commands.length} ordered checks.`);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    runPreflight(validatePreflightCommands());
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
