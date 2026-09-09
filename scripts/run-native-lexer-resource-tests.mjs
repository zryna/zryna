import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const SCRIPT_PATH = fileURLToPath(import.meta.url);
const ROOT = resolve(import.meta.dirname, '..');

export const REQUIRED_RESOURCE_TESTS = Object.freeze([
  'token_and_trivia_limits_accept_exact_and_reject_first_extra',
  'project_lexeme_limit_accepts_exact_and_rejects_first_extra',
  'raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs',
]);

export const RESOURCE_COMMANDS = Object.freeze([
  Object.freeze({
    args: Object.freeze([
      'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer',
      '--', '--include-ignored',
    ]),
    requiredTests: Object.freeze(REQUIRED_RESOURCE_TESTS.slice(0, 2)),
  }),
  Object.freeze({
    args: Object.freeze([
      'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer_admission',
      '--', '--ignored', '--exact',
      'raw_byte_limits_accept_exact_and_reject_complete_first_extra_inputs',
    ]),
    requiredTests: Object.freeze(REQUIRED_RESOURCE_TESTS.slice(2)),
  }),
]);

export function verifyResourceTestOutput(output, command) {
  for (const name of command.requiredTests) {
    const passedLine = `test ${name} ... ok`;
    if (output.split(/\r?\n/u).filter((line) => line === passedLine).length !== 1) {
      throw new Error(`required native lexer resource test did not pass: ${name}`);
    }
  }
  const summaries = [...output.matchAll(/test result: ok\. ([0-9]+) passed; ([0-9]+) failed;/gu)];
  const passed = summaries.length === 1 ? Number.parseInt(summaries[0][1], 10) : 0;
  const failed = summaries.length === 1 ? Number.parseInt(summaries[0][2], 10) : 1;
  if (passed < command.requiredTests.length || failed !== 0) {
    throw new Error('native lexer resource command did not report its required nonzero test set');
  }
}

export function runNativeLexerResourceTests(spawn = spawnSync) {
  for (const command of RESOURCE_COMMANDS) {
    const result = spawn('cargo', command.args, {
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
    verifyResourceTestOutput(`${result.stdout}\n${result.stderr}\n`, command);
  }
}

if (process.argv[1] && resolve(process.argv[1]) === SCRIPT_PATH) {
  try {
    runNativeLexerResourceTests();
  } catch (error) {
    console.error(error instanceof Error ? error.message : String(error));
    process.exitCode = 1;
  }
}
