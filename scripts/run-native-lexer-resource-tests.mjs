import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const ROOT = resolve(import.meta.dirname, '..');

export const REQUIRED_RESOURCE_TESTS = Object.freeze([
  'token_and_trivia_limits_accept_exact_and_reject_first_extra',
  'project_lexeme_limit_accepts_exact_and_rejects_first_extra',
  'raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs',
  'raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs',
]);

export const RESOURCE_COMMANDS = Object.freeze([
  Object.freeze([
    'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer',
    '--', '--include-ignored',
  ]),
  Object.freeze([
    'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer_admission',
    '--', '--ignored', '--exact',
    'raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs',
  ]),
]);

export function verifyResourceTestOutput(output) {
  for (const name of REQUIRED_RESOURCE_TESTS) {
    if (!output.includes(`test ${name} ... ok`)) {
      throw new Error(`required native lexer resource test did not pass: ${name}`);
    }
  }
  const summary = /test result: ok\. ([0-9]+) passed;/.exec(output);
  if (!summary || Number.parseInt(summary[1], 10) < REQUIRED_RESOURCE_TESTS.length) {
    throw new Error('native lexer resource command executed no required test set');
  }
}

export function runNativeLexerResourceTests(spawn = spawnSync) {
  let output = '';
  for (const args of RESOURCE_COMMANDS) {
    const result = spawn('cargo', args, {
      cwd: ROOT,
      encoding: 'utf8',
      maxBuffer: 4 * 1024 * 1024,
      shell: false,
      windowsHide: true,
    });
    if (result.stdout) process.stdout.write(result.stdout);
    if (result.stderr) process.stderr.write(result.stderr);
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`native lexer resource tests exited ${result.status}`);
    output += `${result.stdout}\n${result.stderr}\n`;
  }
  verifyResourceTestOutput(output);
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    runNativeLexerResourceTests();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
