import assert from 'node:assert/strict';
import test from 'node:test';
import { terminateWindowsTree } from '../scripts/stability-gates/process.mjs';
import { childEnvironment } from '../scripts/stability-gates/fuzz-files.mjs';

// Inert spawn injection: no taskkill, process or native program executes in this test.
test('Windows cleanup forwards the filtered caller environment for every stopping reason', () => {
  for (const reason of ['ETIMEDOUT', 'ECANCELED', 'MAXOUTPUT']) {
    const supplied = childEnvironment({ GH_TOKEN: 'secret', github_token: 'secret',
      HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/trusted/ca', PATH: 'tools' });
    let calls = 0;
    const expected = { status: 0, error: undefined };
    const result = terminateWindowsTree(123, supplied, (executable, args, options) => {
      calls += 1;
      assert(executable.endsWith('taskkill.exe'), reason);
      assert.deepEqual(args, ['/PID', '123', '/T', '/F']);
      assert.equal(options.env, supplied);
      assert.deepEqual(options.env, { HTTPS_PROXY: 'proxy', NODE_EXTRA_CA_CERTS: '/trusted/ca', PATH: 'tools' });
      assert.equal(options.timeout, 10000);
      assert.equal(options.maxBuffer, 65536);
      assert.equal(options.shell, false);
      assert.equal(options.windowsHide, true);
      return expected;
    });
    assert.equal(result, expected);
    assert.equal(calls, 1);
  }
});
