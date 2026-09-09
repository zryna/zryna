import assert from 'node:assert/strict';
import test from 'node:test';

import {
  REQUIRED_DIFFERENTIAL_TEST,
  runNativeLexerProviderDifferential,
  verifyDifferentialOutput,
} from '../scripts/run-native-lexer-provider-differential.mjs';

const success = `test ${REQUIRED_DIFFERENTIAL_TEST} ... ok
test result: ok. 1 passed; 0 failed; 0 ignored;`;

test('provider differential output requires the exact nonzero test', () => {
  assert.doesNotThrow(() => verifyDifferentialOutput(success));
  assert.throws(() => verifyDifferentialOutput('test result: ok. 0 passed;'), /did not pass/);
});

test('provider differential runner invokes one locked ignored test without a shell', () => {
  let observed;
  runNativeLexerProviderDifferential((executable, args, options) => {
    observed = { executable, args, options };
    return { status: 0, stdout: success, stderr: '' };
  });
  assert.equal(observed.executable, 'cargo');
  assert.deepEqual(observed.args, [
    'test', '--locked', '-p', 'zryna-frontend', '--test', 'native_lexer_provider',
    '--', '--ignored', '--exact', REQUIRED_DIFFERENTIAL_TEST,
  ]);
  assert.equal(observed.options.shell, false);
});
