import assert from 'node:assert/strict';
import test from 'node:test';

import {
  REQUIRED_RESOURCE_TESTS,
  RESOURCE_COMMANDS,
  runNativeLexerResourceTests,
  verifyResourceTestOutput,
} from '../scripts/run-native-lexer-resource-tests.mjs';

function successfulOutput(command, passed = command.requiredTests.length) {
  return `${command.requiredTests.map((name) => `test ${name} ... ok`).join('\n')}
test result: ok. ${passed} passed; 0 failed; 0 ignored;`;
}

test('each resource command requires its own named tests and nonzero summary', () => {
  assert.equal(new Set(REQUIRED_RESOURCE_TESTS).size, REQUIRED_RESOURCE_TESTS.length);
  for (const command of RESOURCE_COMMANDS) {
    assert.doesNotThrow(() => verifyResourceTestOutput(successfulOutput(command), command));
  }
  assert.throws(
    () => verifyResourceTestOutput(`${successfulOutput(RESOURCE_COMMANDS[0])}\n${successfulOutput(RESOURCE_COMMANDS[0])}`, RESOURCE_COMMANDS[0]),
    /did not pass/,
  );
  assert.throws(() => verifyResourceTestOutput('test result: ok. 0 passed; 0 failed;', RESOURCE_COMMANDS[1]), /did not pass/);
});

test('duplicate, missing, and wrong-command output cannot satisfy the oracle', () => {
  const [nativeLexer, admission] = RESOURCE_COMMANDS;
  const duplicate = `${nativeLexer.requiredTests.map(() => `test ${nativeLexer.requiredTests[0]} ... ok`).join('\n')}
test result: ok. 2 passed; 0 failed;`;
  assert.throws(() => verifyResourceTestOutput(duplicate, nativeLexer), /did not pass/);
  assert.throws(
    () => verifyResourceTestOutput(`test ${nativeLexer.requiredTests[0]} ... ok\ntest result: ok. 1 passed; 0 failed;`, nativeLexer),
    /did not pass/,
  );
  assert.throws(() => verifyResourceTestOutput(successfulOutput(admission), nativeLexer), /did not pass/);
  assert.throws(() => verifyResourceTestOutput(successfulOutput(nativeLexer), admission), /did not pass/);
});

test('resource runner invokes locked Cargo without a shell', () => {
  const observed = [];
  let commandIndex = 0;
  runNativeLexerResourceTests((executable, args, options) => {
    observed.push({ executable, args, options });
    const stdout = successfulOutput(RESOURCE_COMMANDS[commandIndex]);
    commandIndex += 1;
    return { status: 0, stdout, stderr: '' };
  });
  assert.deepEqual(observed.map((entry) => entry.executable), ['cargo', 'cargo']);
  assert.deepEqual(observed.map((entry) => entry.args), RESOURCE_COMMANDS.map(({ args }) => args));
  assert(observed.every((entry) => entry.options.shell === false));
  assert(observed.every((entry) => entry.options.maxBuffer === 4 * 1024 * 1024));
});
