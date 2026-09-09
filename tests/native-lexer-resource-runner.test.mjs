import assert from 'node:assert/strict';
import test from 'node:test';

import {
  REQUIRED_RESOURCE_TESTS,
  RESOURCE_COMMANDS,
  runNativeLexerResourceTests,
  verifyResourceTestOutput,
} from '../scripts/run-native-lexer-resource-tests.mjs';

const success = `${REQUIRED_RESOURCE_TESTS.map((name) => `test ${name} ... ok`).join('\n')}
test result: ok. 14 passed; 0 failed; 0 ignored;`;

test('resource output requires both production-limit tests and a nonzero summary', () => {
  assert.doesNotThrow(() => verifyResourceTestOutput(success));
  assert.throws(() => verifyResourceTestOutput('test result: ok. 0 passed;'), /did not pass/);
  assert.throws(
    () => verifyResourceTestOutput(`test ${REQUIRED_RESOURCE_TESTS[0]} ... ok\n`),
    /did not pass/,
  );
});

test('resource runner invokes locked Cargo without a shell', () => {
  const observed = [];
  runNativeLexerResourceTests((executable, args, options) => {
    observed.push({ executable, args, options });
    return { status: 0, stdout: success, stderr: '' };
  });
  assert.deepEqual(observed.map((entry) => entry.executable), ['cargo', 'cargo']);
  assert.deepEqual(observed.map((entry) => entry.args), RESOURCE_COMMANDS);
  assert(observed.every((entry) => entry.options.shell === false));
  assert(observed.every((entry) => entry.options.maxBuffer === 4 * 1024 * 1024));
});
