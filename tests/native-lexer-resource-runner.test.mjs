import assert from 'node:assert/strict';
import test from 'node:test';

import {
  REQUIRED_RESOURCE_TESTS,
  runNativeLexerResourceTests,
  verifyResourceTestOutput,
} from '../scripts/run-native-lexer-resource-tests.mjs';

const success = `${REQUIRED_RESOURCE_TESTS.map((name) => `test ${name} ... ok`).join('\n')}
test result: ok. 13 passed; 0 failed; 0 ignored;`;

test('resource output requires both production-limit tests and a nonzero summary', () => {
  assert.doesNotThrow(() => verifyResourceTestOutput(success));
  assert.throws(() => verifyResourceTestOutput('test result: ok. 0 passed;'), /did not pass/);
  assert.throws(
    () => verifyResourceTestOutput(`test ${REQUIRED_RESOURCE_TESTS[0]} ... ok\n`),
    /did not pass/,
  );
});

test('resource runner invokes locked Cargo without a shell', () => {
  let observed;
  runNativeLexerResourceTests((executable, args, options) => {
    observed = { executable, args, options };
    return { status: 0, stdout: success, stderr: '' };
  });
  assert.equal(observed.executable, 'cargo');
  assert.deepEqual(observed.args, [
    'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer',
    '--', '--include-ignored',
  ]);
  assert.equal(observed.options.shell, false);
  assert.equal(observed.options.maxBuffer, 4 * 1024 * 1024);
});
